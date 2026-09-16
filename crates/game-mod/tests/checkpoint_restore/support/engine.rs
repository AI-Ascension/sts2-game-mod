// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use sts2_game_mod::{
    ExactRestoreCapability, ExactRestoreCurrentOwner, ExactRestoreEngine, ExactRestoreError,
    ExactRestoreOwnerFence, ExactRestoreStore, RestoreApplyOutcome, RestoreClosureView,
    RestoreHostApplier, RestoreOwnerProvider,
};

use super::PRINCIPAL;
use super::fixtures::{Fixture, authorization_for, chunk_payload, request};

pub(crate) fn send<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    kind: &str,
    payload: Value,
) -> Result<Value, Box<dyn std::error::Error>>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let frame = request(kind, payload);
    let authorization = authorization_for(&frame, PRINCIPAL)?;
    let body = serde_json::to_vec(&frame)?;
    let response = engine.handle(&body, &authorization)?;
    assert!(response.body.len() <= 16_384);
    Ok(serde_json::from_slice(&response.body)?)
}

pub(crate) fn send_blob<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    fixture: &Fixture,
    bytes: &[u8],
    artifact_digest: &str,
) -> Result<Value, Box<dyn std::error::Error>>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let chunk = chunk_payload(fixture, bytes, 0, artifact_digest);
    let result = send(engine, "exact_restore_chunk_request", chunk)?;
    assert_eq!(result["payload"]["result"], "CHUNK_ACCEPTED", "{result:?}");
    let finished = send(
        engine,
        "exact_restore_finish_blob_request",
        json!({
            "operation_id": fixture.operation_id,
            "expected_owner": fixture.owner,
            "artifact_digest": artifact_digest,
            "total_bytes": bytes.len()
        }),
    )?;
    assert_eq!(finished["payload"]["result"], "BLOB_VERIFIED");
    Ok(finished)
}

pub(crate) fn create_engine<A>(
    store: Arc<Mutex<StoreState>>,
    owner: Arc<Mutex<ExactRestoreCurrentOwner>>,
    applier: A,
) -> Result<ExactRestoreEngine<MemoryStore, SharedOwner, A>, ExactRestoreError>
where
    A: RestoreHostApplier,
{
    ExactRestoreEngine::new(
        MemoryStore(store),
        SharedOwner(owner),
        applier,
        PRINCIPAL.into(),
    )
}

pub(crate) fn current_owner(owner: &ExactRestoreOwnerFence) -> ExactRestoreCurrentOwner {
    ExactRestoreCurrentOwner {
        fence: owner.clone(),
        observed_at_millis: 1_800_000_000_000,
    }
}

pub(crate) fn owner_with_generation(
    owner: &ExactRestoreOwnerFence,
    authority_generation: u64,
) -> Result<ExactRestoreOwnerFence, ExactRestoreError> {
    ExactRestoreOwnerFence::new(
        owner.deployment_id().to_owned(),
        owner.instance_id().to_owned(),
        owner.instance_incarnation().to_owned(),
        owner.boot_id().to_owned(),
        authority_generation,
        owner.host_fence_id().to_owned(),
        owner.host_fence_generation(),
        owner.lease_id().to_owned(),
        owner.lease_epoch(),
        owner.session_id().to_owned(),
        owner.lease_expires_at_millis(),
    )
}

#[derive(Default)]
pub(crate) struct StoreState {
    pub(crate) index: Vec<u8>,
    pub(crate) blobs: BTreeMap<String, Vec<u8>>,
    boundaries: BTreeMap<String, BTreeSet<u64>>,
    pub(crate) append_calls: usize,
}

pub(crate) struct MemoryStore(pub(crate) Arc<Mutex<StoreState>>);

impl ExactRestoreStore for MemoryStore {
    fn read_index(&mut self) -> Result<Vec<u8>, ExactRestoreError> {
        Ok(self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?
            .index
            .clone())
    }

    fn write_index(&mut self, index: &[u8]) -> Result<(), ExactRestoreError> {
        self.0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?
            .index = index.to_vec();
        Ok(())
    }

    fn append_chunk(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), ExactRestoreError> {
        if bytes.is_empty()
            || bytes.len() > 8192
            || offset
                .checked_add(bytes.len() as u64)
                .is_none_or(|end| end > total_bytes)
        {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut state = self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let blob = state.blobs.entry(key.to_owned()).or_default();
        if blob.len() as u64 != offset {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        blob.extend_from_slice(bytes);
        state
            .boundaries
            .entry(key.to_owned())
            .or_default()
            .insert(offset);
        state.append_calls += 1;
        Ok(())
    }

    fn read_blob(&mut self, key: &str, maximum_bytes: u64) -> Result<Vec<u8>, ExactRestoreError> {
        let state = self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let bytes = state.blobs.get(key).cloned().unwrap_or_default();
        if bytes.len() as u64 > maximum_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        Ok(bytes)
    }

    fn read_blob_range(
        &mut self,
        key: &str,
        offset: u64,
        bytes: usize,
    ) -> Result<Vec<u8>, ExactRestoreError> {
        let state = self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let blob = state
            .blobs
            .get(key)
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        let start = usize::try_from(offset).map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let end = start
            .checked_add(bytes)
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        blob.get(start..end)
            .map(<[u8]>::to_vec)
            .ok_or(ExactRestoreError::StorageUnavailable)
    }

    fn has_chunk_start(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
    ) -> Result<bool, ExactRestoreError> {
        if offset >= total_bytes {
            return Ok(false);
        }
        Ok(self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?
            .boundaries
            .get(key)
            .is_some_and(|boundaries| boundaries.contains(&offset)))
    }

    fn next_chunk_start(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        committed_bytes: u64,
    ) -> Result<u64, ExactRestoreError> {
        if committed_bytes > total_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let state = self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        Ok(state
            .boundaries
            .get(key)
            .and_then(|boundaries| {
                boundaries
                    .range((
                        std::ops::Bound::Excluded(offset),
                        std::ops::Bound::Unbounded,
                    ))
                    .next()
                    .copied()
            })
            .unwrap_or(committed_bytes))
    }

    fn reconcile_blob(
        &mut self,
        key: &str,
        total_bytes: u64,
        committed_bytes: u64,
    ) -> Result<(), ExactRestoreError> {
        if committed_bytes > total_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut state = self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        if let Some(blob) = state.blobs.get_mut(key) {
            if blob.len() < committed_bytes as usize {
                return Err(ExactRestoreError::StorageUnavailable);
            }
            blob.truncate(committed_bytes as usize);
        } else if committed_bytes != 0 {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        if let Some(boundaries) = state.boundaries.get_mut(key) {
            boundaries.retain(|offset| *offset < committed_bytes);
        }
        Ok(())
    }

    fn remove_operation(&mut self, operation_id: &str) -> Result<(), ExactRestoreError> {
        let prefix = format!("{operation_id}-");
        let mut state = self
            .0
            .lock()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        state.blobs.retain(|key, _| !key.starts_with(&prefix));
        state.boundaries.retain(|key, _| !key.starts_with(&prefix));
        Ok(())
    }
}

pub(crate) struct SharedOwner(pub(crate) Arc<Mutex<ExactRestoreCurrentOwner>>);

impl RestoreOwnerProvider for SharedOwner {
    fn current_owner(&mut self) -> Result<ExactRestoreCurrentOwner, ExactRestoreError> {
        self.0
            .lock()
            .map_err(|_| ExactRestoreError::OwnerUnavailable)
            .map(|owner| owner.clone())
    }
}

pub(crate) struct RecordingApplier {
    calls: Arc<AtomicUsize>,
    outcome: ApplierOutcome,
}

enum ApplierOutcome {
    Verified(String),
    Unknown,
    Interrupt,
}

impl RecordingApplier {
    pub(crate) fn verified(calls: Arc<AtomicUsize>, exact_state_digest: String) -> Self {
        Self {
            calls,
            outcome: ApplierOutcome::Verified(exact_state_digest),
        }
    }

    pub(crate) fn unknown(calls: Arc<AtomicUsize>) -> Self {
        Self {
            calls,
            outcome: ApplierOutcome::Unknown,
        }
    }

    pub(crate) fn interrupt_after_start(calls: Arc<AtomicUsize>) -> Self {
        Self {
            calls,
            outcome: ApplierOutcome::Interrupt,
        }
    }
}

impl RestoreHostApplier for RecordingApplier {
    fn capability(&self) -> ExactRestoreCapability {
        ExactRestoreCapability::Available
    }

    fn restore(
        &mut self,
        _request: &Value,
        closure: RestoreClosureView<'_>,
    ) -> RestoreApplyOutcome {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(!closure.manifest.is_empty());
        assert!(!closure.blobs.is_empty());
        match &self.outcome {
            ApplierOutcome::Verified(digest) => RestoreApplyOutcome::Verified {
                recaptured_exact_state_digest: digest.clone(),
            },
            ApplierOutcome::Unknown => RestoreApplyOutcome::Unknown,
            ApplierOutcome::Interrupt => {
                std::panic::resume_unwind(Box::new("synthetic interruption after durable intent"))
            }
        }
    }
}
