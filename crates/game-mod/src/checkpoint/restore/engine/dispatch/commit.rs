// SPDX-License-Identifier: MIT

use serde_json::{Value, json};

use super::super::super::wire::{RequestFrame, canonical_json, digest};
use super::super::super::{
    ExactRestoreCapability, ExactRestoreCurrentOwner, ExactRestoreError, ExactRestoreStore,
    RestoreApplyOutcome, RestoreClosureView, RestoreHostApplier, RestoreOwnerProvider,
};
use super::super::state::{
    ExactRestoreEngine, ExactRestoreResponse, RestoreOperation, RestorePhaseState,
};
use super::phase_payload;

type StagedBlob = (String, Vec<u8>);
type RestoreClosureBytes = (Vec<u8>, Vec<StagedBlob>);

pub(super) fn commit<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    current: &ExactRestoreCurrentOwner,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let operation_id = super::request_operation(frame)?;
    let operation = engine
        .index()
        .operations
        .get(&operation_id)
        .cloned()
        .ok_or(ExactRestoreError::InvalidPhase)?;
    validate_commit_payload(&operation, &frame.payload)?;
    match operation.phase {
        RestorePhaseState::RestoreVerified => {
            return commit_response(engine, frame, "RESTORE_VERIFIED", operation.receipt);
        }
        RestorePhaseState::Unknown => {
            return commit_response(engine, frame, "UNKNOWN", None);
        }
        RestorePhaseState::CommitIntent => {
            if let Some(stored) = engine.index_mut().operations.get_mut(&operation_id) {
                stored.phase = RestorePhaseState::Unknown;
            }
            engine.persist_index()?;
            return commit_response(engine, frame, "UNKNOWN", None);
        }
        RestorePhaseState::Staging => return Err(ExactRestoreError::InvalidPhase),
        RestorePhaseState::ClosureVerified => {}
    }
    if engine.applier().capability() != ExactRestoreCapability::Available {
        return Err(ExactRestoreError::NoRestoreAdapter);
    }
    let (manifest_bytes, blobs) = collect_closure(engine, &operation)?;
    let before = engine.index().clone();
    let stored = engine
        .index_mut()
        .operations
        .get_mut(&operation_id)
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    stored.phase = RestorePhaseState::CommitIntent;
    stored.commit_request_digest = Some(frame.request_digest.clone());
    if let Err(error) = engine.persist_index() {
        *engine.index_mut() = before;
        return Err(error);
    }

    let request = &operation.immutable_begin;
    let views = blobs
        .iter()
        .map(|(digest_value, bytes)| (digest_value.as_str(), bytes.as_slice()))
        .collect::<Vec<_>>();
    let outcome = engine.applier_mut().restore(
        request,
        RestoreClosureView {
            manifest: &manifest_bytes,
            blobs: views,
        },
    );
    match outcome {
        RestoreApplyOutcome::Unknown => {
            if persist_unknown(engine, &operation_id).is_ok() {
                commit_response(engine, frame, "UNKNOWN", None)
            } else {
                persist_unknown_best_effort(engine, &operation_id);
                may_have_started_response(engine, frame)
            }
        }
        RestoreApplyOutcome::Verified {
            recaptured_exact_state_digest,
        } if request["exact_state_digest"].as_str()
            == Some(recaptured_exact_state_digest.as_str()) =>
        {
            let receipt = make_receipt(&operation, &current.fence, &recaptured_exact_state_digest)?;
            if let Some(stored) = engine.index_mut().operations.get_mut(&operation_id) {
                stored.phase = RestorePhaseState::RestoreVerified;
                stored.receipt = Some(receipt.clone());
            }
            match engine.persist_index() {
                Ok(()) => commit_response(engine, frame, "RESTORE_VERIFIED", Some(receipt)),
                Err(_) => {
                    persist_unknown_best_effort(engine, &operation_id);
                    may_have_started_response(engine, frame)
                }
            }
        }
        RestoreApplyOutcome::Verified { .. } => {
            if persist_unknown(engine, &operation_id).is_ok() {
                commit_response(engine, frame, "UNKNOWN", None)
            } else {
                persist_unknown_best_effort(engine, &operation_id);
                may_have_started_response(engine, frame)
            }
        }
    }
}

fn validate_commit_payload(
    operation: &RestoreOperation,
    payload: &Value,
) -> Result<(), ExactRestoreError> {
    let begin = &operation.immutable_begin;
    for key in [
        "operation_id",
        "expected_owner",
        "branch",
        "checkpoint_id",
        "closure_digest",
        "exact_state_digest",
        "manifest_digest",
    ] {
        if begin[key] != payload[key] {
            return Err(ExactRestoreError::OperationConflict);
        }
    }
    Ok(())
}

fn collect_closure<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    operation: &RestoreOperation,
) -> Result<RestoreClosureBytes, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let manifest = operation
        .blobs
        .iter()
        .find(|blob| blob.manifest)
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    let manifest_bytes = engine
        .store_mut()
        .read_blob(&manifest.key(&operation.operation_id), manifest.total_bytes)?;
    let mut items = Vec::new();
    let artifacts = operation.immutable_begin["artifacts"]
        .as_array()
        .ok_or(ExactRestoreError::InvalidFrame)?;
    let mut seen = std::collections::BTreeSet::new();
    for artifact in artifacts {
        let artifact_digest = artifact["digest"]
            .as_str()
            .ok_or(ExactRestoreError::InvalidFrame)?;
        if !seen.insert(artifact_digest.to_owned()) {
            continue;
        }
        let progress = operation
            .blobs
            .iter()
            .find(|blob| !blob.manifest && blob.digest == artifact_digest)
            .ok_or(ExactRestoreError::UnknownArtifact)?;
        items.push((
            artifact_digest.to_owned(),
            engine
                .store_mut()
                .read_blob(&progress.key(&operation.operation_id), progress.total_bytes)?,
        ));
    }
    Ok((manifest_bytes, items))
}

fn make_receipt(
    operation: &RestoreOperation,
    current_owner: &super::super::super::ExactRestoreOwnerFence,
    recaptured_digest: &str,
) -> Result<Value, ExactRestoreError> {
    let begin = &operation.immutable_begin;
    let mut receipt = json!({
        "operation_id": operation.operation_id,
        "branch": begin["branch"],
        "destination_owner": current_owner,
        "checkpoint_id": begin["checkpoint_id"],
        "exact_state_digest": begin["exact_state_digest"],
        "manifest_digest": begin["manifest_digest"],
        "closure_digest": begin["closure_digest"],
        "compatibility_digest": begin["compatibility_digest"],
        "coverage_contract_digest": begin["coverage_contract_digest"],
        "aggregate_closure_bytes": begin["aggregate_closure_bytes"],
        "artifact_reference_count": begin["artifact_reference_count"],
        "distinct_blob_count": begin["distinct_blob_count"],
        "boundary": begin["boundary"],
        "recaptured_exact_state_digest": recaptured_digest
    });
    let canonical = canonical_json(&receipt).map_err(ExactRestoreError::from)?;
    let receipt_digest = digest(&canonical);
    receipt
        .as_object_mut()
        .ok_or(ExactRestoreError::StorageUnavailable)?
        .insert(
            String::from("receipt_digest"),
            Value::String(receipt_digest),
        );
    Ok(receipt)
}

fn commit_response<S, O, A>(
    engine: &ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    state: &str,
    receipt: Option<Value>,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let mut additional = json!({});
    if let Some(receipt) = receipt {
        additional["receipt"] = receipt;
    }
    let payload = phase_payload(frame, state, state, additional);
    engine.response(frame, "exact_restore_commit_response", payload, 200)
}

fn persist_unknown<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    operation_id: &str,
) -> Result<(), ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    if let Some(operation) = engine.index_mut().operations.get_mut(operation_id) {
        operation.phase = RestorePhaseState::Unknown;
        operation.receipt = None;
    }
    engine.persist_index()
}

fn persist_unknown_best_effort<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    operation_id: &str,
) where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    if let Some(operation) = engine.index_mut().operations.get_mut(operation_id) {
        operation.phase = RestorePhaseState::Unknown;
        operation.receipt = None;
    }
    let _ = engine.persist_index();
}

fn may_have_started_response<S, O, A>(
    engine: &ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let payload = json!({
        "operation_id": frame.operation_id,
        "expected_owner": frame.expected_owner,
        "request_digest": frame.request_digest,
        "outcome": "UNAVAILABLE",
        "error_code": "native_unavailable",
        "host_effect": "may_have_started"
    });
    engine.response(frame, "exact_restore_error_response", payload, 503)
}
