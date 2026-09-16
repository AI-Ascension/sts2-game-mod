// SPDX-License-Identifier: MIT

use super::wire::{decode_request, encode_frame};

mod api;
mod dispatch;
mod owner;
mod state;
mod validation;

pub use api::{
    ExactRestoreCapability, ExactRestoreError, ExactRestoreStore, NoExactRestoreApplier,
    RestoreApplyOutcome, RestoreClosureView, RestoreHostApplier,
};
pub use owner::{
    ExactRestoreAuthorization, ExactRestoreCurrentOwner, ExactRestoreOwnerFence,
    ExactRestoreUnavailableOwner, RestoreOwnerProvider,
};
pub use state::{ExactRestoreEngine, ExactRestoreResponse};

pub(super) const MAX_INDEX_BYTES: usize = 16 * 1024 * 1024;
pub(super) const STAGING_TTL_MILLIS: u64 = 24 * 60 * 60 * 1000;

/// Returns the typed fail-closed response used when no production restore owner is installed.
///
/// The authenticated gateway has already verified the complete live owner tuple. This endpoint
/// still binds the frame correlation and the instance/session/lease subset carried by the native
/// callback. It never opens storage or accepts bytes.
pub fn exact_restore_unavailable_response(
    body: &[u8],
    authorization: &ExactRestoreAuthorization,
    configured_principal: &str,
    expected_kind: &str,
) -> Result<ExactRestoreResponse, ExactRestoreError> {
    if authorization.principal != configured_principal {
        return Err(ExactRestoreError::Unauthorized);
    }
    let frame = decode_request(body)?;
    if frame.kind != expected_kind {
        return Err(ExactRestoreError::InvalidFrame);
    }
    if frame.correlation_id != authorization.correlation_id {
        return Err(ExactRestoreError::Unauthorized);
    }
    let owner: ExactRestoreOwnerFence = serde_json::from_value(frame.expected_owner.clone())
        .map_err(|_| ExactRestoreError::InvalidFrame)?;
    owner.validate()?;
    if !owner.matches_transport(authorization) {
        return Err(ExactRestoreError::StaleOwner);
    }
    let (outcome, error_code, status) = if frame.kind == "exact_restore_begin_request" {
        ("REJECTED", "no_restore_adapter", 409)
    } else {
        ("UNAVAILABLE", "native_unavailable", 503)
    };
    let payload = serde_json::json!({
        "operation_id": frame.operation_id,
        "expected_owner": frame.expected_owner,
        "request_digest": frame.request_digest,
        "outcome": outcome,
        "error_code": error_code,
        "host_effect": "not_started"
    });
    let value =
        super::wire::response_base("exact_restore_error_response", &frame.message_id, payload)?;
    let encoded = encode_frame(&value)?;
    Ok(ExactRestoreResponse {
        status,
        body: encoded,
    })
}
