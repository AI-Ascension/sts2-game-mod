// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::super::super::wire::RequestFrame;
use super::super::super::{
    EXACT_RESTORE_MAX_BLOB_BYTES, EXACT_RESTORE_MAX_CLOSURE_BYTES, EXACT_RESTORE_MAX_REFERENCES,
    EXACT_RESTORE_MAX_TERMINAL_RECEIPTS, ExactRestoreCapability, ExactRestoreError,
    ExactRestoreStore, RestoreHostApplier, RestoreOwnerProvider,
};
use super::super::state::{
    ExactRestoreEngine, ExactRestoreResponse, RestoreBlobProgress, RestoreOperation,
    RestorePhaseState,
};
use super::{parse_owner, phase_payload};

pub(super) fn begin<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    _current: &super::super::super::ExactRestoreCurrentOwner,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    if engine.applier().capability() != ExactRestoreCapability::Available {
        return Err(ExactRestoreError::NoRestoreAdapter);
    }
    let payload = &frame.payload;
    let operation_id = super::request_operation(frame)?;
    let owner_value = parse_owner(frame)?;
    let owner: super::super::super::ExactRestoreOwnerFence =
        serde_json::from_value(owner_value.clone()).map_err(|_| ExactRestoreError::InvalidFrame)?;
    let blobs = validate_begin_artifacts(payload)?;

    if let Some(existing) = engine.index().operations.get(&operation_id) {
        if existing.immutable_begin != *payload || existing.owner != owner {
            return Err(ExactRestoreError::OperationConflict);
        }
        let state = phase_name(existing.phase);
        let result = phase_payload(
            frame,
            "EXISTING",
            state,
            json!({"receipt": existing.receipt}),
        );
        return engine.response(
            frame,
            "exact_restore_begin_response",
            remove_null_receipt(result),
            200,
        );
    }

    let index = engine.index();
    let unfinished = index
        .operations
        .values()
        .filter(|operation| operation.phase.is_unfinished())
        .count();
    let terminal = index.operations.len().saturating_sub(unfinished);
    if unfinished != 0 || terminal >= EXACT_RESTORE_MAX_TERMINAL_RECEIPTS {
        return Err(ExactRestoreError::CapacityFull);
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    let operation = RestoreOperation {
        operation_id: operation_id.clone(),
        owner,
        immutable_begin: payload.clone(),
        phase: RestorePhaseState::Staging,
        created_at_millis: now,
        blobs,
        receipt: None,
        commit_request_digest: None,
    };
    let before = engine.index().clone();
    engine
        .index_mut()
        .operations
        .insert(operation_id, operation);
    if let Err(error) = engine.persist_index() {
        *engine.index_mut() = before;
        return Err(error);
    }
    let response = phase_payload(frame, "CREATED", "STAGING", json!({}));
    engine.response(frame, "exact_restore_begin_response", response, 201)
}

fn validate_begin_artifacts(
    payload: &Value,
) -> Result<Vec<RestoreBlobProgress>, ExactRestoreError> {
    let artifacts = payload["artifacts"]
        .as_array()
        .ok_or(ExactRestoreError::InvalidFrame)?;
    let reference_count =
        u64::try_from(artifacts.len()).map_err(|_| ExactRestoreError::InvalidFrame)?;
    if artifacts.len() < 2 || artifacts.len() > EXACT_RESTORE_MAX_REFERENCES {
        return Err(ExactRestoreError::InvalidFrame);
    }
    if payload["artifact_reference_count"].as_u64() != Some(reference_count) {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let manifest_size = payload["manifest_size_bytes"]
        .as_u64()
        .filter(|size| (1..=EXACT_RESTORE_MAX_BLOB_BYTES).contains(size))
        .ok_or(ExactRestoreError::InvalidFrame)?;
    let manifest_digest = payload["manifest_digest"]
        .as_str()
        .ok_or(ExactRestoreError::InvalidFrame)?;
    let mut unique = BTreeMap::<String, (u64, bool)>::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let role = artifact["role"]
            .as_str()
            .ok_or(ExactRestoreError::InvalidFrame)?;
        if (index == 0 && role != "canonical-state") || (index > 0 && role == "canonical-state") {
            return Err(ExactRestoreError::InvalidFrame);
        }
        let digest = artifact["digest"]
            .as_str()
            .ok_or(ExactRestoreError::InvalidFrame)?;
        let size = artifact["size_bytes"]
            .as_u64()
            .filter(|size| *size <= EXACT_RESTORE_MAX_BLOB_BYTES)
            .ok_or(ExactRestoreError::InvalidFrame)?;
        if digest == manifest_digest && size != manifest_size {
            return Err(ExactRestoreError::InvalidFrame);
        }
        if artifact["codec"].as_str().is_none_or(str::is_empty) {
            return Err(ExactRestoreError::InvalidFrame);
        }
        match unique.get(digest) {
            Some((existing_size, _)) if *existing_size != size => {
                return Err(ExactRestoreError::InvalidFrame);
            }
            Some(_) => {}
            None => {
                unique.insert(digest.to_owned(), (size, index == 0));
            }
        }
    }
    if artifacts[0]["role"].as_str() != Some("canonical-state") {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let distinct_count =
        u64::try_from(unique.len()).map_err(|_| ExactRestoreError::InvalidFrame)?;
    if payload["distinct_blob_count"].as_u64() != Some(distinct_count) {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let distinct_size_sum = unique.values().try_fold(0_u64, |sum, (size, _)| {
        sum.checked_add(*size)
            .ok_or(ExactRestoreError::InvalidFrame)
    })?;
    let aggregate = manifest_size
        .checked_add(distinct_size_sum)
        .ok_or(ExactRestoreError::InvalidFrame)?;
    if payload["aggregate_closure_bytes"].as_u64() != Some(aggregate)
        || aggregate > EXACT_RESTORE_MAX_CLOSURE_BYTES
    {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let mut blobs = Vec::with_capacity(unique.len() + 1);
    blobs.push(RestoreBlobProgress {
        digest: manifest_digest.to_owned(),
        total_bytes: manifest_size,
        next_offset: 0,
        verified: false,
        manifest: true,
    });
    for (digest, (total_bytes, _)) in unique {
        blobs.push(RestoreBlobProgress {
            digest,
            total_bytes,
            next_offset: 0,
            verified: false,
            manifest: false,
        });
    }
    Ok(blobs)
}

fn phase_name(state: RestorePhaseState) -> &'static str {
    match state {
        RestorePhaseState::Staging => "STAGING",
        RestorePhaseState::ClosureVerified => "CLOSURE_VERIFIED",
        RestorePhaseState::CommitIntent => "COMMIT_INTENT",
        RestorePhaseState::Unknown => "UNKNOWN",
        RestorePhaseState::RestoreVerified => "RESTORE_VERIFIED",
    }
}

fn remove_null_receipt(mut value: Value) -> Value {
    if value["receipt"].is_null()
        && let Some(object) = value.as_object_mut()
    {
        object.remove("receipt");
    }
    value
}
