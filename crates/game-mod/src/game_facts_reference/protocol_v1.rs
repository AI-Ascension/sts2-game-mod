// SPDX-License-Identifier: MIT

//! Static metadata mapping for the accepted typed game-facts profile.
//!
//! This adapter checks values supplied by its caller. It does not authenticate their origin,
//! prove coherent extraction, evaluate rules, or read host state.

mod mapping;
mod responses;

use super::FactsInventory;
use crate::ContentManifest;
use sts2_protocol::game_facts_reference_v1::{
    GameFactsReferenceV1Codec, GameFactsReferenceV1Rejection as Rejection, Message, MessageKind,
    validate_response_against_request,
};

/// Stateless mapper for static GameMod facts metadata.
#[derive(Clone, Copy, Debug, Default)]
pub struct GameFactsReferenceV1Adapter;

impl GameFactsReferenceV1Adapter {
    /// Returns the accepted schema limits without advertising any supported rule or live policy.
    pub fn capabilities(correlation_id: &str) -> Result<Message, Rejection> {
        let response = responses::capabilities_response(correlation_id);
        GameFactsReferenceV1Codec::validate(&response)?;
        Ok(response)
    }

    /// Maps one typed request using caller-supplied inventory and manifest evidence.
    ///
    /// A matching manifest value and inventory binding prove consistency only. The caller must
    /// independently establish source authenticity and coherent extraction; public manifest fields
    /// and shape-valid provenance tokens do not provide that proof.
    pub fn respond(
        request: &Message,
        inventory: Option<&FactsInventory>,
        manifest: Option<&ContentManifest>,
    ) -> Result<Message, Rejection> {
        GameFactsReferenceV1Codec::validate(request)?;
        if request.kind != MessageKind::QueryRequest {
            return Err(Rejection::InvalidRequest);
        }
        let query = request.query.as_ref().ok_or(Rejection::Malformed)?;
        let response = match mapping::map_query(query, inventory, manifest) {
            Ok(result) => responses::query_response(&request.correlation_id, query, result),
            Err(code) => responses::error_response(&request.correlation_id, query, code),
        };
        GameFactsReferenceV1Codec::validate(&response)?;
        validate_response_against_request(request, &response)?;
        Ok(response)
    }
}
