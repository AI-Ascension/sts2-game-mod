// SPDX-License-Identifier: MIT

use serde_json::Value;

use super::super::wire::{canonical_json, digest};
use super::super::{
    EXACT_RESTORE_MAX_BLOB_BYTES, EXACT_RESTORE_MAX_CLOSURE_BYTES, EXACT_RESTORE_MAX_REFERENCES,
    EXACT_RESTORE_MAX_TERMINAL_RECEIPTS,
};
use super::api::ExactRestoreError;
use super::owner::{MAX_SAFE_INTEGER, valid_uuid};
use super::state;

pub(super) fn validate_index(index: &state::RestoreIndex) -> Result<(), ExactRestoreError> {
    if index.version != 1 || index.operations.len() > EXACT_RESTORE_MAX_TERMINAL_RECEIPTS + 1 {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let mut unfinished = 0_usize;
    let mut terminal = 0_usize;
    for (operation_id, operation) in &index.operations {
        if operation_id != &operation.operation_id
            || !valid_uuid(operation_id, true)
            || operation.immutable_begin["operation_id"].as_str() != Some(operation_id)
            || operation.immutable_begin["expected_owner"]
                != serde_json::to_value(&operation.owner)
                    .map_err(|_| ExactRestoreError::StorageUnavailable)?
            || operation.blobs.len() > EXACT_RESTORE_MAX_REFERENCES + 1
        {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        operation
            .owner
            .validate()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        if operation.created_at_millis == 0 || operation.created_at_millis > MAX_SAFE_INTEGER {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        validate_operation_artifacts(operation)?;
        let mut manifest_count = 0;
        let mut seen = std::collections::BTreeSet::new();
        for blob in &operation.blobs {
            if !valid_digest(&blob.digest)
                || blob.total_bytes > EXACT_RESTORE_MAX_BLOB_BYTES
                || blob.next_offset > blob.total_bytes
                || (blob.verified && blob.next_offset != blob.total_bytes)
            {
                return Err(ExactRestoreError::StorageUnavailable);
            }
            if blob.manifest {
                manifest_count += 1;
            } else if !seen.insert(blob.digest.as_str()) {
                return Err(ExactRestoreError::StorageUnavailable);
            }
        }
        if manifest_count != 1 {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        if operation.phase.is_unfinished() {
            unfinished += 1;
        } else {
            terminal += 1;
        }
        if (operation.phase == state::RestorePhaseState::RestoreVerified)
            != operation.receipt.is_some()
        {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let has_commit_intent = matches!(
            operation.phase,
            state::RestorePhaseState::CommitIntent
                | state::RestorePhaseState::Unknown
                | state::RestorePhaseState::RestoreVerified
        );
        if has_commit_intent != operation.commit_request_digest.is_some()
            || operation
                .commit_request_digest
                .as_deref()
                .is_some_and(|value| !valid_digest(value))
            || (matches!(
                operation.phase,
                state::RestorePhaseState::ClosureVerified
                    | state::RestorePhaseState::CommitIntent
                    | state::RestorePhaseState::Unknown
                    | state::RestorePhaseState::RestoreVerified
            ) && operation.blobs.iter().any(|blob| !blob.verified))
        {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        if let Some(receipt) = &operation.receipt {
            validate_receipt(operation, receipt)?;
        }
    }
    if unfinished > 1 || terminal > EXACT_RESTORE_MAX_TERMINAL_RECEIPTS {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

fn validate_operation_artifacts(
    operation: &state::RestoreOperation,
) -> Result<(), ExactRestoreError> {
    let begin = &operation.immutable_begin;
    let artifacts = begin["artifacts"]
        .as_array()
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    if artifacts.len() < 2 || artifacts.len() > EXACT_RESTORE_MAX_REFERENCES {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let manifest_digest = begin["manifest_digest"]
        .as_str()
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    let manifest_size = begin["manifest_size_bytes"]
        .as_u64()
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    let manifest = operation
        .blobs
        .iter()
        .find(|blob| blob.manifest)
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    if manifest.digest != manifest_digest || manifest.total_bytes != manifest_size {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let mut distinct = std::collections::BTreeMap::<&str, u64>::new();
    for artifact in artifacts {
        let digest = artifact["digest"]
            .as_str()
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        let size = artifact["size_bytes"]
            .as_u64()
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        if !valid_digest(digest)
            || size > EXACT_RESTORE_MAX_BLOB_BYTES
            || distinct
                .insert(digest, size)
                .is_some_and(|existing| existing != size)
            || (digest == manifest_digest && size != manifest_size)
        {
            return Err(ExactRestoreError::StorageUnavailable);
        }
    }
    if begin["artifact_reference_count"].as_u64() != Some(artifacts.len() as u64)
        || begin["distinct_blob_count"].as_u64() != Some(distinct.len() as u64)
    {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let distinct_bytes = distinct.values().try_fold(0_u64, |sum, size| {
        sum.checked_add(*size)
            .ok_or(ExactRestoreError::StorageUnavailable)
    })?;
    let aggregate = manifest_size
        .checked_add(distinct_bytes)
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    if aggregate > EXACT_RESTORE_MAX_CLOSURE_BYTES
        || begin["aggregate_closure_bytes"].as_u64() != Some(aggregate)
    {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let stored = operation
        .blobs
        .iter()
        .filter(|blob| !blob.manifest)
        .map(|blob| (blob.digest.as_str(), blob.total_bytes))
        .collect::<std::collections::BTreeMap<_, _>>();
    if stored != distinct {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

fn validate_receipt(
    operation: &state::RestoreOperation,
    receipt: &Value,
) -> Result<(), ExactRestoreError> {
    let begin = &operation.immutable_begin;
    if receipt["operation_id"].as_str() != Some(operation.operation_id.as_str())
        || receipt["destination_owner"]
            != serde_json::to_value(&operation.owner)
                .map_err(|_| ExactRestoreError::StorageUnavailable)?
        || receipt["branch"] != begin["branch"]
        || receipt["checkpoint_id"] != begin["checkpoint_id"]
        || receipt["exact_state_digest"] != begin["exact_state_digest"]
        || receipt["manifest_digest"] != begin["manifest_digest"]
        || receipt["closure_digest"] != begin["closure_digest"]
        || receipt["compatibility_digest"] != begin["compatibility_digest"]
        || receipt["coverage_contract_digest"] != begin["coverage_contract_digest"]
        || receipt["aggregate_closure_bytes"] != begin["aggregate_closure_bytes"]
        || receipt["artifact_reference_count"] != begin["artifact_reference_count"]
        || receipt["distinct_blob_count"] != begin["distinct_blob_count"]
        || receipt["boundary"] != begin["boundary"]
        || receipt["recaptured_exact_state_digest"] != begin["exact_state_digest"]
    {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let mut without_digest = receipt.clone();
    without_digest
        .as_object_mut()
        .ok_or(ExactRestoreError::StorageUnavailable)?
        .remove("receipt_digest");
    let expected = digest(&canonical_json(&without_digest).map_err(ExactRestoreError::from)?);
    if receipt["receipt_digest"].as_str() != Some(expected.as_str()) {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}
