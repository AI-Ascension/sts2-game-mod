// SPDX-License-Identifier: MIT

use sts2_protocol::game_facts_reference_v1::{
    Capabilities, ErrorBody, ErrorCode, GAME_FACTS_REFERENCE_V1_ARTIFACT,
    GAME_FACTS_REFERENCE_V1_MAX_MESSAGE_BYTES, GAME_FACTS_REFERENCE_V1_PROFILE,
    GAME_FACTS_REFERENCE_V1_SCHEMA_DIGEST, GameFactsProvenance, Message, MessageKind, Query,
    QueryResult,
};

const MAX_RULE_IDS: u16 = 16;
const MAX_INPUTS_PER_RULE: u16 = 64;
const MAX_UNSUPPORTED_COMBINATIONS: u16 = 256;

pub(super) fn capabilities_response(correlation_id: &str) -> Message {
    Message {
        protocol_version: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
        schema_digest: GAME_FACTS_REFERENCE_V1_SCHEMA_DIGEST.to_owned(),
        provenance: provenance(),
        correlation_id: correlation_id.to_owned(),
        kind: MessageKind::CapabilitiesResponse,
        query: None,
        result: None,
        capabilities: Some(Capabilities {
            profile: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
            rules_reference_version: 2,
            max_rule_ids: MAX_RULE_IDS,
            max_inputs_per_rule: MAX_INPUTS_PER_RULE,
            max_unsupported_combinations: MAX_UNSUPPORTED_COMBINATIONS,
            max_message_bytes: GAME_FACTS_REFERENCE_V1_MAX_MESSAGE_BYTES as u32,
            snapshot_policy: None,
        }),
        error: None,
    }
}

pub(super) fn query_response(correlation_id: &str, query: &Query, result: QueryResult) -> Message {
    Message {
        protocol_version: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
        schema_digest: GAME_FACTS_REFERENCE_V1_SCHEMA_DIGEST.to_owned(),
        provenance: provenance(),
        correlation_id: correlation_id.to_owned(),
        kind: MessageKind::QueryResponse,
        query: Some(query.clone()),
        result: Some(result),
        capabilities: None,
        error: None,
    }
}

pub(super) fn error_response(correlation_id: &str, query: &Query, code: ErrorCode) -> Message {
    let reason = match code {
        ErrorCode::InvalidBinding => "The supplied content binding does not match this request.",
        _ => "No supported static inventory is available for this request.",
    };
    Message {
        protocol_version: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
        schema_digest: GAME_FACTS_REFERENCE_V1_SCHEMA_DIGEST.to_owned(),
        provenance: provenance(),
        correlation_id: correlation_id.to_owned(),
        kind: MessageKind::ErrorResponse,
        query: Some(query.clone()),
        result: None,
        capabilities: None,
        error: Some(ErrorBody {
            code,
            field: None,
            reason: Some(reason.to_owned()),
            retryable: false,
        }),
    }
}

fn provenance() -> GameFactsProvenance {
    GameFactsProvenance {
        artifact: GAME_FACTS_REFERENCE_V1_ARTIFACT.to_owned(),
        source: "schemas/game-facts-reference-v1.schema.json".to_owned(),
        generator: "hand-authored".to_owned(),
    }
}
