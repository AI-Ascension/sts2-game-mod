// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::wire::{RequestFrame, decode_request, encode_frame, parse_unique_value};
use super::super::{
    ExactRestoreAuthorization, ExactRestoreCurrentOwner, ExactRestoreError, ExactRestoreOwnerFence,
    ExactRestoreStore, RestoreHostApplier, RestoreOwnerProvider,
};
use super::validation::validate_index;
use super::{MAX_INDEX_BYTES, STAGING_TTL_MILLIS, dispatch};

/// Durable state of one restore operation.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum RestorePhaseState {
    Staging,
    ClosureVerified,
    CommitIntent,
    Unknown,
    RestoreVerified,
}

impl RestorePhaseState {
    pub(super) const fn is_unfinished(self) -> bool {
        matches!(
            self,
            Self::Staging | Self::ClosureVerified | Self::CommitIntent | Self::Unknown
        )
    }
}

/// Progress for the manifest or one distinct canonical/restore blob.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RestoreBlobProgress {
    pub(super) digest: String,
    pub(super) total_bytes: u64,
    pub(super) next_offset: u64,
    pub(super) verified: bool,
    pub(super) manifest: bool,
}

impl RestoreBlobProgress {
    pub(super) fn key(&self, operation_id: &str) -> String {
        let role = if self.manifest {
            String::from("manifest")
        } else {
            format!("blob-{}", self.digest.trim_start_matches("sha256:"))
        };
        format!("{operation_id}-{role}")
    }
}

/// One durable logical operation.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RestoreOperation {
    pub(super) operation_id: String,
    pub(super) owner: ExactRestoreOwnerFence,
    pub(super) immutable_begin: Value,
    pub(super) phase: RestorePhaseState,
    pub(super) created_at_millis: u64,
    pub(super) blobs: Vec<RestoreBlobProgress>,
    pub(super) receipt: Option<Value>,
    pub(super) commit_request_digest: Option<String>,
}

/// Bounded private metadata index. Blob bytes live in separate digest-derived files.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RestoreIndex {
    pub(super) version: u32,
    pub(super) operations: BTreeMap<String, RestoreOperation>,
}

/// Bounded result suitable for the fixed native HTTP adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRestoreResponse {
    /// HTTP status mapped from the typed protocol outcome.
    pub status: u16,
    /// Complete canonical protocol response frame.
    pub body: Vec<u8>,
}

/// Persistent exact-restore phase machine.
pub struct ExactRestoreEngine<S, O, A> {
    store: S,
    owner_provider: O,
    applier: A,
    configured_principal: String,
    index: RestoreIndex,
}

impl<S, O, A> Debug for ExactRestoreEngine<S, O, A> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExactRestoreEngine")
            .field("operation_count", &self.index.operations.len())
            .field(
                "unfinished_count",
                &self
                    .index
                    .operations
                    .values()
                    .filter(|operation| operation.phase.is_unfinished())
                    .count(),
            )
            .finish_non_exhaustive()
    }
}

impl<S, O, A> ExactRestoreEngine<S, O, A>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    /// Loads and reconciles a bounded durable staging index.
    pub fn new(
        mut store: S,
        owner_provider: O,
        applier: A,
        configured_principal: String,
    ) -> Result<Self, ExactRestoreError> {
        if configured_principal.is_empty() || configured_principal.len() > 512 {
            return Err(ExactRestoreError::Unauthorized);
        }
        let bytes = store.read_index()?;
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut index = if bytes.is_empty() {
            RestoreIndex {
                version: 1,
                operations: BTreeMap::new(),
            }
        } else {
            let value =
                parse_unique_value(&bytes).map_err(|_| ExactRestoreError::StorageUnavailable)?;
            serde_json::from_value(value).map_err(|_| ExactRestoreError::StorageUnavailable)?
        };
        for operation in index.operations.values_mut() {
            if operation.phase == RestorePhaseState::CommitIntent {
                operation.phase = RestorePhaseState::Unknown;
            }
        }
        validate_index(&index)?;
        for operation in index.operations.values() {
            if operation.phase.is_unfinished() {
                for blob in &operation.blobs {
                    store.reconcile_blob(
                        &blob.key(&operation.operation_id),
                        blob.total_bytes,
                        blob.next_offset,
                    )?;
                }
            }
        }
        let now = system_time_millis().ok_or(ExactRestoreError::StorageUnavailable)?;
        let expired = index
            .operations
            .values()
            .filter(|operation| {
                matches!(
                    operation.phase,
                    RestorePhaseState::Staging | RestorePhaseState::ClosureVerified
                ) && now.saturating_sub(operation.created_at_millis) > STAGING_TTL_MILLIS
            })
            .map(|operation| operation.operation_id.clone())
            .collect::<Vec<_>>();
        for operation_id in expired {
            store.remove_operation(&operation_id)?;
            index.operations.remove(&operation_id);
        }
        let mut engine = Self {
            store,
            owner_provider,
            applier,
            configured_principal,
            index,
        };
        engine.persist_index()?;
        Ok(engine)
    }

    /// Handles one canonical, already bounded protocol request.
    pub fn handle(
        &mut self,
        body: &[u8],
        authorization: &ExactRestoreAuthorization,
    ) -> Result<ExactRestoreResponse, ExactRestoreError> {
        let frame = decode_request(body)?;
        if let Err(error) = self.validate_authorization(&frame, authorization) {
            return dispatch::error_response(self, &frame, error);
        }
        let current = match self.owner_provider.current_owner() {
            Ok(owner) => owner,
            Err(error) => return dispatch::error_response(self, &frame, error),
        };
        if let Err(error) = self.validate_current_owner(&frame, authorization, &current) {
            return dispatch::error_response(self, &frame, error);
        }
        dispatch::dispatch(self, frame, current)
    }

    /// Returns the number of durable operations retained by this instance.
    #[must_use]
    pub fn operation_count(&self) -> usize {
        self.index.operations.len()
    }

    /// Returns the number of accepted bytes without exposing staged content.
    #[must_use]
    pub fn staged_bytes(&self) -> u64 {
        self.index
            .operations
            .values()
            .flat_map(|operation| operation.blobs.iter())
            .map(|blob| blob.next_offset)
            .sum()
    }

    pub(super) fn validate_authorization(
        &self,
        frame: &RequestFrame,
        authorization: &ExactRestoreAuthorization,
    ) -> Result<(), ExactRestoreError> {
        if authorization.principal != self.configured_principal
            || authorization.correlation_id != frame.correlation_id
        {
            return Err(ExactRestoreError::Unauthorized);
        }
        let expected: ExactRestoreOwnerFence = serde_json::from_value(frame.expected_owner.clone())
            .map_err(|_| ExactRestoreError::InvalidFrame)?;
        expected.validate()?;
        if !expected.matches_transport(authorization) {
            return Err(ExactRestoreError::StaleOwner);
        }
        Ok(())
    }

    pub(super) fn validate_current_owner(
        &self,
        frame: &RequestFrame,
        authorization: &ExactRestoreAuthorization,
        current: &ExactRestoreCurrentOwner,
    ) -> Result<(), ExactRestoreError> {
        current.fence.validate()?;
        if current.fence.lease_expires_at_millis <= current.observed_at_millis
            || current.fence
                != serde_json::from_value(frame.expected_owner.clone())
                    .map_err(|_| ExactRestoreError::InvalidFrame)?
            || !current.fence.matches_transport(authorization)
        {
            return Err(ExactRestoreError::StaleOwner);
        }
        Ok(())
    }

    pub(super) fn persist_index(&mut self) -> Result<(), ExactRestoreError> {
        validate_index(&self.index)?;
        let bytes =
            serde_json::to_vec(&self.index).map_err(|_| ExactRestoreError::StorageUnavailable)?;
        if bytes.len() > MAX_INDEX_BYTES {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        self.store.write_index(&bytes)
    }

    pub(super) fn response(
        &self,
        frame: &RequestFrame,
        kind: &str,
        payload: Value,
        status: u16,
    ) -> Result<ExactRestoreResponse, ExactRestoreError> {
        let value = super::super::wire::response_base(kind, &frame.message_id, payload)?;
        let bytes = encode_frame(&value)?;
        Ok(ExactRestoreResponse {
            status,
            body: bytes,
        })
    }

    pub(super) fn store_mut(&mut self) -> &mut S {
        &mut self.store
    }

    pub(super) fn index(&self) -> &RestoreIndex {
        &self.index
    }

    pub(super) fn index_mut(&mut self) -> &mut RestoreIndex {
        &mut self.index
    }

    pub(super) fn applier(&self) -> &A {
        &self.applier
    }

    pub(super) fn applier_mut(&mut self) -> &mut A {
        &mut self.applier
    }
}

fn system_time_millis() -> Option<u64> {
    let elapsed = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    u64::try_from(elapsed.as_millis()).ok()
}
