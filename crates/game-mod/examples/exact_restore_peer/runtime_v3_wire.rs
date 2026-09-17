// SPDX-License-Identifier: MIT

use serde_json::{Value, json};

use super::config::TransportConfig;
use super::runtime_v3::{ACTION_ID, EFFECT_KIND, SCHEMA_DIGEST, STATE_ID};

#[allow(clippy::too_many_arguments)]
pub(super) fn envelope(
    transport: &TransportConfig,
    request: &Value,
    kind: &str,
    generation: u64,
    observation: Option<Value>,
    legal_actions: Option<Vec<Value>>,
    transition: Option<Value>,
    operation_id: Option<&str>,
    status: Option<&str>,
    error_code: Option<&str>,
) -> Value {
    json!({
        "protocol_version": "runtime-v3-gameplay",
        "schema_digest": SCHEMA_DIGEST,
        "provenance": {"artifact":"sts2-protocol/runtime-v3-gameplay","source":"schemas/runtime-v3-gameplay.schema.json","generator":"hand-authored"},
        "correlation_id": request["correlation_id"],
        "instance_id": transport.instance_id,
        "session_id": transport.session_id,
        "lease_id": transport.lease_id,
        "lease_epoch": transport.lease_epoch,
        "generation": generation,
        "kind": kind,
        "state_id": STATE_ID,
        "operation_id": operation_id,
        "observation": observation,
        "legal_actions": legal_actions,
        "action": null,
        "status": status,
        "transition": transition,
        "error_code": error_code,
        "wait_for_millis": null,
        "wait_outcome": null,
        "recovery": null
    })
}

pub(super) fn observation(generation: u64) -> Value {
    let state = if generation == 0 {
        json!({"state":"combat", "turn_index":1, "enemies":[]})
    } else {
        json!({"state":"victory"})
    };
    json!({
        "state_id": STATE_ID,
        "generation": generation,
        "visible_seed": "runtime-v3-peer-seed",
        "player": {"hp": 50, "max_hp": 50, "energy": 3, "gold": 99, "hand": [], "deck": [], "discard": [], "exhaust": []},
        "state": state
    })
}

pub(super) fn legal_actions(generation: u64) -> Vec<Value> {
    if generation == 0 {
        vec![json!({"action_id": ACTION_ID, "action": {"kind":"end_turn"}})]
    } else {
        Vec::new()
    }
}

pub(super) fn settled_response(
    correlation: &str,
    transport: &TransportConfig,
    operation_id: &str,
) -> Value {
    let request = json!({"correlation_id": correlation});
    envelope(
        transport,
        &request,
        "dispatch_action_response",
        1,
        Some(observation(1)),
        Some(Vec::new()),
        Some(
            json!({"from_generation":0,"to_generation":1,"state_id":STATE_ID,"effect_kind":EFFECT_KIND}),
        ),
        Some(operation_id),
        Some("settled"),
        None,
    )
}

pub(super) fn rejected_response(
    transport: &TransportConfig,
    request: &Value,
    operation_id: &str,
    code: &str,
) -> Value {
    envelope(
        transport,
        request,
        "dispatch_action_response",
        0,
        Some(observation(0)),
        Some(legal_actions(0)),
        None,
        Some(operation_id),
        Some("rejected"),
        Some(code),
    )
}

pub(super) fn unknown_response(
    transport: &TransportConfig,
    request: &Value,
    operation_id: &str,
) -> Value {
    envelope(
        transport,
        request,
        "wait_response",
        0,
        None,
        None,
        None,
        Some(operation_id),
        Some("unknown"),
        Some("operation_unknown"),
    )
}

pub(super) fn json_error(code: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"error_code": code})).unwrap_or_default()
}

pub(super) fn json_error_value(code: &str) -> Value {
    json!({"error_code": code})
}
