// SPDX-License-Identifier: MIT

//! Canonical facts-v1 boundary for the production owner's copied live inputs.

use super::protocol_v1::responses;
use super::{LiveFactsRefusal, OwnedLiveFactsSnapshot, live_mapping};
use crate::ContentManifest;
use sts2_protocol::game_facts_reference_v1::{
    ErrorCode, ExpiryBehavior, GameFactsReferenceV1Codec as Codec,
    GameFactsReferenceV1Rejection as Rejection, Message, MessageKind, SnapshotInvalidator,
    SnapshotPolicy, validate_response_against_request,
};

/// Produces source-derived input observations, never a rule evaluation or native parity claim.
///
/// The production route supplies only owned host copies after authorization and freshness checks.
/// Arbitrary callers constructing those copies do not gain source authenticity from this API.
#[derive(Clone, Copy, Debug, Default)]
pub struct OwnedLiveGameFactsProducer;

impl OwnedLiveGameFactsProducer {
    /// Validates and normalizes an untrusted request before managed code reads its selectors.
    pub fn admit(request: &[u8], correlation_id: &str) -> Result<Vec<u8>, Rejection> {
        Codec::encode(&decode_request(request, correlation_id)?)
    }

    /// Advertises one retained generation; this declares implementation, not source readiness.
    pub fn capabilities(correlation_id: &str) -> Result<Vec<u8>, Rejection> {
        let mut response = responses::capabilities_response(correlation_id);
        if let Some(capabilities) = response.capabilities.as_mut() {
            capabilities.snapshot_policy = Some(SnapshotPolicy {
                supports_live: true,
                lifetime_generations: 1,
                max_retained_snapshots: 1,
                expiry_behavior: ExpiryBehavior::RejectStaleSnapshot,
                invalidated_by: vec![
                    SnapshotInvalidator::Restore,
                    SnapshotInvalidator::Restart,
                    SnapshotInvalidator::ProfileChange,
                    SnapshotInvalidator::ContentChange,
                    SnapshotInvalidator::RunChange,
                    SnapshotInvalidator::EpochChange,
                ],
            });
        }
        Codec::encode(&response)
    }

    /// Maps a retained capture and an independently produced current content manifest.
    ///
    /// The owner must revalidate the capture across the catalog read before invoking this method.
    /// No request value is used to fill a missing source field, build or mode.
    pub fn respond(
        request: &[u8],
        correlation_id: &str,
        source: &OwnedLiveFactsSnapshot,
        manifest: &ContentManifest,
    ) -> Result<Vec<u8>, Rejection> {
        let request = decode_request(request, correlation_id)?;
        let query = request.query.as_ref().ok_or(Rejection::Malformed)?;
        let response = match live_mapping::map_query(query, source, manifest) {
            Ok(result) => responses::query_response(correlation_id, query, result),
            Err(code) => refusal_response(&request, code)?,
        };
        validate_response_against_request(&request, &response)?;
        Codec::encode(&response)
    }

    /// Echoes an admitted request in a bounded canonical refusal without exposing host details.
    pub fn refuse(
        request: &[u8],
        correlation_id: &str,
        refusal: LiveFactsRefusal,
    ) -> Result<Vec<u8>, Rejection> {
        let request = decode_request(request, correlation_id)?;
        let response = refusal_response(&request, refusal.code())?;
        validate_response_against_request(&request, &response)?;
        Codec::encode(&response)
    }
}

fn decode_request(request: &[u8], correlation_id: &str) -> Result<Message, Rejection> {
    let request = Codec::decode(request)?;
    if request.kind != MessageKind::QueryRequest || request.correlation_id != correlation_id {
        return Err(Rejection::InvalidRequest);
    }
    Ok(request)
}

fn refusal_response(request: &Message, code: ErrorCode) -> Result<Message, Rejection> {
    let query = request.query.as_ref().ok_or(Rejection::Malformed)?;
    let mut response = responses::error_response(&request.correlation_id, query, code);
    if let Some(error) = response.error.as_mut() {
        error.reason = Some(
            match code {
                ErrorCode::DeniedScope => "The request does not match the admitted owner scope.",
                ErrorCode::StaleSnapshot => "The retained owner snapshot is no longer current.",
                ErrorCode::InvalidBinding => "The current owner content binding does not match.",
                ErrorCode::UnknownId => "The requested rule or card occurrence is unknown.",
                _ => "The owner cannot supply the requested live inputs.",
            }
            .to_owned(),
        );
    }
    Ok(response)
}
