// SPDX-License-Identifier: MIT

use serde_json::Value;

use super::super::super::wire::{RequestFrame, digest};
use super::super::super::{
    EXACT_RESTORE_MAX_BLOB_BYTES, EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES,
    EXACT_RESTORE_MAX_CHUNK_BYTES, ExactRestoreError, ExactRestoreStore, RestoreHostApplier,
    RestoreOwnerProvider,
};
use super::super::state::{ExactRestoreEngine, ExactRestoreResponse, RestorePhaseState};

mod closure;
mod decode;
mod responses;
use decode::decode_base64_canonical;
use responses::{chunk_response, finish_response, matching_progress};

pub(super) fn chunk<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let operation_id = super::request_operation(frame)?;
    let artifact_digest = string_field(&frame.payload, "artifact_digest")?;
    let offset = number_field(&frame.payload, "offset")?;
    let total_bytes = number_field(&frame.payload, "total_bytes")?;
    let chunk_digest = string_field(&frame.payload, "chunk_digest")?;
    let encoded = string_field(&frame.payload, "data_base64")?;
    if encoded.len() > EXACT_RESTORE_MAX_CHUNK_BASE64_BYTES {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let bytes = decode_base64_canonical(encoded)?;
    if bytes.is_empty() || bytes.len() > EXACT_RESTORE_MAX_CHUNK_BYTES {
        return Err(ExactRestoreError::InvalidFrame);
    }
    if digest(&bytes) != chunk_digest {
        return Err(ExactRestoreError::DigestMismatch);
    }
    let end = offset
        .checked_add(bytes.len() as u64)
        .filter(|end| *end <= total_bytes)
        .ok_or(ExactRestoreError::ChunkConflict)?;
    let operation = engine
        .index()
        .operations
        .get(&operation_id)
        .ok_or(ExactRestoreError::InvalidPhase)?;
    let progress = matching_progress(operation, artifact_digest)?;
    if operation.phase == RestorePhaseState::ClosureVerified
        && progress.iter().all(|blob| {
            blob.total_bytes == total_bytes && blob.verified && blob.next_offset == total_bytes
        })
    {
        return finish_response(
            engine,
            frame,
            artifact_digest,
            total_bytes,
            "CLOSURE_VERIFIED",
        );
    }
    if operation.phase != RestorePhaseState::Staging {
        return Err(ExactRestoreError::InvalidPhase);
    }
    if progress
        .iter()
        .all(|blob| blob.total_bytes == total_bytes && blob.verified)
    {
        return finish_response(engine, frame, artifact_digest, total_bytes, "STAGING");
    }
    if progress.iter().any(|blob| blob.total_bytes != total_bytes) {
        return Err(ExactRestoreError::ChunkConflict);
    }
    let mut duplicate = false;
    for blob in &progress {
        let key = blob.key(&operation_id);
        if offset < blob.next_offset {
            if offset != blob.next_offset
                && engine
                    .store_mut()
                    .has_chunk_start(&key, total_bytes, offset)?
            {
                let next = engine.store_mut().next_chunk_start(
                    &key,
                    total_bytes,
                    offset,
                    blob.next_offset,
                )?;
                if next != end {
                    return Err(ExactRestoreError::ChunkConflict);
                }
                let existing = engine
                    .store_mut()
                    .read_blob_range(&key, offset, bytes.len())?;
                if existing != bytes {
                    return Err(ExactRestoreError::ChunkConflict);
                }
                duplicate = true;
                continue;
            }
            return Err(ExactRestoreError::ChunkConflict);
        }
        if offset != blob.next_offset {
            return Err(ExactRestoreError::ChunkConflict);
        }
    }
    if duplicate {
        if progress.iter().any(|blob| blob.next_offset < end) {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        return chunk_response(engine, frame, artifact_digest, progress[0].next_offset);
    }

    let before = engine.index().clone();
    for blob in &progress {
        engine
            .store_mut()
            .append_chunk(&blob.key(&operation_id), total_bytes, offset, &bytes)?;
    }
    if let Some(operation) = engine.index_mut().operations.get_mut(&operation_id) {
        for blob in operation
            .blobs
            .iter_mut()
            .filter(|blob| blob.digest == artifact_digest)
        {
            blob.next_offset = end;
        }
    }
    if let Err(error) = engine.persist_index() {
        *engine.index_mut() = before;
        for blob in &progress {
            let _ = engine.store_mut().reconcile_blob(
                &blob.key(&operation_id),
                blob.total_bytes,
                blob.next_offset,
            );
        }
        return Err(error);
    }
    chunk_response(engine, frame, artifact_digest, end)
}

pub(super) fn finish<S, O, A>(
    engine: &mut ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let operation_id = super::request_operation(frame)?;
    let artifact_digest = string_field(&frame.payload, "artifact_digest")?;
    let total_bytes = number_field(&frame.payload, "total_bytes")?;
    if total_bytes > EXACT_RESTORE_MAX_BLOB_BYTES {
        return Err(ExactRestoreError::InvalidFrame);
    }
    let operation = engine
        .index()
        .operations
        .get(&operation_id)
        .ok_or(ExactRestoreError::InvalidPhase)?;
    if operation.phase != RestorePhaseState::Staging {
        return Err(ExactRestoreError::InvalidPhase);
    }
    let progress = matching_progress(operation, artifact_digest)?;
    if progress.iter().any(|blob| {
        blob.total_bytes != total_bytes || blob.next_offset != blob.total_bytes || blob.verified
    }) {
        return Err(ExactRestoreError::InvalidPhase);
    }
    let mut artifacts = Vec::with_capacity(progress.len());
    for blob in &progress {
        let data = engine
            .store_mut()
            .read_blob(&blob.key(&operation_id), blob.total_bytes)?;
        if data.len() as u64 != blob.total_bytes || digest(&data) != artifact_digest {
            return Err(ExactRestoreError::DigestMismatch);
        }
        artifacts.push((blob.clone(), data));
    }
    let before = engine.index().clone();
    let mut newly_verified = Vec::new();
    if let Some(operation) = engine.index_mut().operations.get_mut(&operation_id) {
        for (blob, _) in &artifacts {
            if let Some(current) = operation
                .blobs
                .iter_mut()
                .find(|candidate| candidate.key(&operation_id) == blob.key(&operation_id))
            {
                current.verified = true;
                newly_verified.push(current.clone());
            }
        }
    }
    let all_verified = engine
        .index()
        .operations
        .get(&operation_id)
        .is_some_and(|operation| operation.blobs.iter().all(|blob| blob.verified));
    if all_verified {
        let operation = engine
            .index()
            .operations
            .get(&operation_id)
            .cloned()
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        let manifest_blob = operation
            .blobs
            .iter()
            .find(|blob| blob.manifest)
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        let manifest_bytes = engine
            .store_mut()
            .read_blob(&manifest_blob.key(&operation_id), manifest_blob.total_bytes)?;
        if let Err(error) = closure::verify_closure(&operation, &manifest_bytes, engine.store_mut())
        {
            *engine.index_mut() = before;
            return Err(error);
        }
        if let Some(operation) = engine.index_mut().operations.get_mut(&operation_id) {
            operation.phase = RestorePhaseState::ClosureVerified;
        }
    }
    if let Err(error) = engine.persist_index() {
        *engine.index_mut() = before;
        for blob in newly_verified {
            let _ = engine.store_mut().reconcile_blob(
                &blob.key(&operation_id),
                blob.total_bytes,
                blob.next_offset,
            );
        }
        return Err(error);
    }
    let state = engine
        .index()
        .operations
        .get(&operation_id)
        .map(|operation| {
            if operation.phase == RestorePhaseState::ClosureVerified {
                "CLOSURE_VERIFIED"
            } else {
                "STAGING"
            }
        })
        .ok_or(ExactRestoreError::StorageUnavailable)?;
    finish_response(engine, frame, artifact_digest, total_bytes, state)
}

fn string_field<'a>(payload: &'a Value, key: &str) -> Result<&'a str, ExactRestoreError> {
    payload[key].as_str().ok_or(ExactRestoreError::InvalidFrame)
}

fn number_field(payload: &Value, key: &str) -> Result<u64, ExactRestoreError> {
    payload[key].as_u64().ok_or(ExactRestoreError::InvalidFrame)
}
