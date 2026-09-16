// SPDX-License-Identifier: MIT

use serde_json::{Value, json};

use super::super::super::wire::RequestFrame;
use super::super::super::{
    ExactRestoreError, ExactRestoreStore, RestoreHostApplier, RestoreOwnerProvider,
};
use super::super::state::{ExactRestoreEngine, ExactRestoreResponse, RestorePhaseState};
use super::phase_payload;

pub(super) fn lookup<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let operation_id = super::request_operation(frame)?;
    let Some(operation) = engine.index().operations.get(&operation_id).cloned() else {
        return response(engine, frame, "NOT_FOUND", "NOT_FOUND", json!({}));
    };
    let mut additional = json!({"receipt": operation.receipt});
    if additional["receipt"].is_null() {
        additional
            .as_object_mut()
            .ok_or(ExactRestoreError::StorageUnavailable)?
            .remove("receipt");
    }
    if let Some(artifact_digest) = frame.payload["artifact_digest"].as_str() {
        let matching = operation
            .blobs
            .iter()
            .filter(|blob| blob.digest == artifact_digest)
            .collect::<Vec<_>>();
        if matching.is_empty() {
            return Err(ExactRestoreError::UnknownArtifact);
        }
        let first = matching[0];
        if matching.iter().any(|blob| {
            blob.total_bytes != first.total_bytes
                || blob.next_offset != first.next_offset
                || blob.verified != first.verified
        }) {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        additional["artifact_digest"] = Value::String(artifact_digest.to_owned());
        additional["total_bytes"] = json!(first.total_bytes);
        additional["next_offset"] = json!(first.next_offset);
        additional["verified"] = json!(first.verified);
    }
    let state = phase_name(operation.phase);
    response(engine, frame, state, state, additional)
}

fn response<S, O, A>(
    engine: &ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    result: &str,
    state: &str,
    additional: Value,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let payload = phase_payload(frame, result, state, additional);
    engine.response(frame, "exact_restore_lookup_response", payload, 200)
}

fn phase_name(phase: RestorePhaseState) -> &'static str {
    match phase {
        RestorePhaseState::Staging => "STAGING",
        RestorePhaseState::ClosureVerified => "CLOSURE_VERIFIED",
        RestorePhaseState::CommitIntent => "COMMIT_INTENT",
        RestorePhaseState::Unknown => "UNKNOWN",
        RestorePhaseState::RestoreVerified => "RESTORE_VERIFIED",
    }
}
