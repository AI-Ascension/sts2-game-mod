// SPDX-License-Identifier: MIT

use serde_json::{Value, json};

use super::super::wire::RequestFrame;
use super::super::{
    ExactRestoreCurrentOwner, ExactRestoreError, ExactRestoreStore, RestoreHostApplier,
    RestoreOwnerProvider,
};
use super::state::{ExactRestoreEngine, ExactRestoreResponse};

mod begin;
mod commit;
mod lookup;
mod transfer;

pub(super) fn dispatch<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    frame: RequestFrame,
    current: ExactRestoreCurrentOwner,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let stored_owner = engine
        .index()
        .operations
        .get(&frame.operation_id)
        .map(|operation| &operation.owner);
    let result = if stored_owner.is_some_and(|owner| owner != &current.fence) {
        Err(ExactRestoreError::StaleOwner)
    } else {
        match frame.kind.as_str() {
            "exact_restore_begin_request" => begin::begin(engine, &frame, &current),
            "exact_restore_chunk_request" => transfer::chunk(engine, &frame),
            "exact_restore_finish_blob_request" => transfer::finish(engine, &frame),
            "exact_restore_commit_request" => commit::commit(engine, &frame, &current),
            "exact_restore_lookup_request" => lookup::lookup(engine, &frame),
            _ => Err(ExactRestoreError::InvalidFrame),
        }
    };
    match result {
        Ok(response) => Ok(response),
        Err(error) => error_response(engine, &frame, error),
    }
}

pub(super) fn error_response<S, O, A>(
    engine: &ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    error: ExactRestoreError,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let (outcome, code, effect, status) = match error {
        ExactRestoreError::InvalidFrame
        | ExactRestoreError::NonCanonicalFrame
        | ExactRestoreError::FrameTooLarge => ("REJECTED", "invalid_frame", "not_started", 400),
        ExactRestoreError::Unauthorized => ("REJECTED", "owner_mismatch", "not_started", 403),
        ExactRestoreError::StaleOwner => ("STALE_OWNER", "owner_mismatch", "not_started", 409),
        ExactRestoreError::OwnerUnavailable | ExactRestoreError::StorageUnavailable => {
            ("UNAVAILABLE", "native_unavailable", "not_started", 503)
        }
        ExactRestoreError::NoRestoreAdapter => {
            ("REJECTED", "no_restore_adapter", "not_started", 409)
        }
        ExactRestoreError::OperationConflict => {
            ("REJECTED", "operation_conflict", "not_started", 409)
        }
        ExactRestoreError::InvalidPhase => ("REJECTED", "invalid_phase", "not_started", 409),
        ExactRestoreError::UnknownArtifact => ("REJECTED", "unknown_artifact", "not_started", 409),
        ExactRestoreError::ChunkConflict => ("REJECTED", "chunk_conflict", "not_started", 409),
        ExactRestoreError::DigestMismatch => ("REJECTED", "digest_mismatch", "not_started", 409),
        ExactRestoreError::CapacityFull => ("REJECTED", "capacity_full", "not_started", 409),
    };
    let payload = json!({
        "operation_id": frame.operation_id,
        "expected_owner": frame.expected_owner,
        "request_digest": frame.request_digest,
        "outcome": outcome,
        "error_code": code,
        "host_effect": effect
    });
    let response = engine.response(frame, "exact_restore_error_response", payload, status)?;
    Ok(response)
}

pub(super) fn phase_payload(
    frame: &RequestFrame,
    result: &str,
    state: &str,
    additional: Value,
) -> Value {
    let mut payload = json!({
        "operation_id": frame.operation_id,
        "result": result,
        "state": state,
        "expected_owner": frame.expected_owner,
        "request_digest": frame.request_digest
    });
    if let (Some(base), Some(extra)) = (payload.as_object_mut(), additional.as_object()) {
        for (key, value) in extra {
            base.insert(key.clone(), value.clone());
        }
    }
    payload
}

pub(super) fn parse_owner(frame: &RequestFrame) -> Result<Value, ExactRestoreError> {
    let owner: super::super::ExactRestoreOwnerFence =
        serde_json::from_value(frame.expected_owner.clone())
            .map_err(|_| ExactRestoreError::InvalidFrame)?;
    owner.validate()?;
    serde_json::to_value(owner).map_err(|_| ExactRestoreError::InvalidFrame)
}

pub(super) fn request_operation(frame: &RequestFrame) -> Result<String, ExactRestoreError> {
    if frame.payload["operation_id"].as_str() != Some(frame.operation_id.as_str()) {
        return Err(ExactRestoreError::InvalidFrame);
    }
    Ok(frame.operation_id.clone())
}
