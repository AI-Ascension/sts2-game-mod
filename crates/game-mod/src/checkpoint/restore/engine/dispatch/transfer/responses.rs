// SPDX-License-Identifier: MIT

use serde_json::json;

use super::super::super::super::wire::RequestFrame;
use super::super::super::super::{
    ExactRestoreError, ExactRestoreStore, RestoreHostApplier, RestoreOwnerProvider,
};
use super::super::super::state::{ExactRestoreEngine, ExactRestoreResponse, RestoreBlobProgress};
use super::super::phase_payload;

pub(super) fn finish_response<S, O, A>(
    engine: &ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    artifact_digest: &str,
    total_bytes: u64,
    state: &str,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let payload = phase_payload(
        frame,
        "BLOB_VERIFIED",
        state,
        json!({
            "artifact_digest": artifact_digest,
            "total_bytes": total_bytes,
            "closure_state": state
        }),
    );
    engine.response(frame, "exact_restore_finish_blob_response", payload, 200)
}

pub(super) fn matching_progress(
    operation: &super::super::super::state::RestoreOperation,
    artifact_digest: &str,
) -> Result<Vec<RestoreBlobProgress>, ExactRestoreError> {
    let matches = operation
        .blobs
        .iter()
        .filter(|blob| blob.digest == artifact_digest)
        .cloned()
        .collect::<Vec<_>>();
    if matches.is_empty() {
        Err(ExactRestoreError::UnknownArtifact)
    } else {
        Ok(matches)
    }
}

pub(super) fn chunk_response<S, O, A>(
    engine: &ExactRestoreEngine<S, O, A>,
    frame: &RequestFrame,
    artifact_digest: &str,
    next_offset: u64,
) -> Result<ExactRestoreResponse, ExactRestoreError>
where
    S: ExactRestoreStore,
    O: RestoreOwnerProvider,
    A: RestoreHostApplier,
{
    let payload = phase_payload(
        frame,
        "CHUNK_ACCEPTED",
        "STAGING",
        json!({
            "artifact_digest": artifact_digest,
            "next_offset": next_offset
        }),
    );
    engine.response(frame, "exact_restore_chunk_response", payload, 200)
}
