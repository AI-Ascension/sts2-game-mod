// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sts2_game_mod::ExactRestoreCurrentOwner;

use super::recovery::{RECOVERY_CONTRACT, RECOVERY_SCHEMA, hex, timestamp, uuid};
use super::runtime_v3::{RuntimeV3State, STATE_ID};

pub(crate) fn runtime_operation_response(
    request: &Value,
    owner: &ExactRestoreCurrentOwner,
    runtime: &mut RuntimeV3State,
) -> (u16, Vec<u8>) {
    let Some(operation) = request["payload"]["operation"].as_object() else {
        return (400, Vec::new());
    };
    let Some(operation_id) = operation["operation_id"].as_str() else {
        return (400, Vec::new());
    };
    if !operation_owner_matches(operation, owner) {
        return (409, error_json("recovery_operation_context_mismatch"));
    }
    let kind = request["kind"].as_str().unwrap_or_default();
    if kind == "operation_intent_request" {
        runtime.remember_pending(operation_id, Value::Object(operation.clone()));
        return (
            200,
            operation_frame(request, "INTENT_RECORDED", Value::Object(operation.clone())),
        );
    }
    let Some(pending) = runtime.pending_operation(operation_id) else {
        return (404, error_json("recovery_operation_not_found"));
    };
    let expected = pending["expected_boundary"].clone();
    let Some(generation) = expected["generation"].as_u64() else {
        return (400, Vec::new());
    };
    let Some(state_id) = expected["state_id"].as_str() else {
        return (400, Vec::new());
    };
    let Some(action_bytes) = pending["action"]["canonical_json_b64"]
        .as_str()
        .and_then(decode_base64_no_pad)
    else {
        return (400, error_json("recovery_action_invalid"));
    };
    let Ok(action) = serde_json::from_slice::<Value>(&action_bytes) else {
        return (400, error_json("recovery_action_invalid"));
    };
    let dispatch = json!({
        "kind": "dispatch_action_request",
        "correlation_id": request["correlation_id"],
        "operation_id": operation_id,
        "generation": generation,
        "state_id": state_id,
        "action": action,
    });
    let result = runtime.dispatch_operation(dispatch);
    let status = result["status"].as_str().unwrap_or_default();
    if status != "settled" {
        let host_status = if status == "rejected" {
            "REJECTED"
        } else {
            "UNKNOWN"
        };
        return (
            if host_status == "UNKNOWN" { 503 } else { 200 },
            operation_frame(
                request,
                host_status,
                with_host_operation(&pending, None, None),
            ),
        );
    }
    let effect_digest = digest(
        pending["action"]["canonical_json_b64"]
            .as_str()
            .unwrap_or_default()
            .as_bytes(),
    );
    let ticket = json!({
        "ticket_id": uuid(),
        "operation_id": operation_id,
        "payload_digest": pending["payload_digest"],
        "boot_id": pending["original_context"]["boot_id"],
        "instance_incarnation": pending["original_context"]["instance_incarnation"],
        "lease_epoch": pending["original_context"]["lease_epoch"],
        "host_fence_id": owner.fence.host_fence_id(),
        "state": "SETTLED",
        "issued_at": timestamp(),
        "expires_at": timestamp(),
    });
    let witness = json!({
        "witness_id": uuid(),
        "operation_id": operation_id,
        "payload_digest": pending["payload_digest"],
        "boot_id": pending["original_context"]["boot_id"],
        "instance_incarnation": pending["original_context"]["instance_incarnation"],
        "host_fence_id": owner.fence.host_fence_id(),
        "source": "host_game_thread",
        "state_id": STATE_ID,
        "generation": 1,
        "effect_digest": effect_digest,
        "observed_at": timestamp(),
    });
    let host_operation = with_host_operation(&pending, Some(ticket), Some(witness));
    runtime.remember_pending(operation_id, host_operation.clone());
    (200, operation_frame(request, "SETTLED", host_operation))
}

fn operation_owner_matches(
    operation: &serde_json::Map<String, Value>,
    owner: &ExactRestoreCurrentOwner,
) -> bool {
    let context = &operation["original_context"];
    context["deployment_id"].as_str() == Some(owner.fence.deployment_id())
        && context["instance_id"].as_str() == Some(owner.fence.instance_id())
        && context["instance_incarnation"].as_str() == Some(owner.fence.instance_incarnation())
        && context["boot_id"].as_str() == Some(owner.fence.boot_id())
        && context["authority_generation"].as_u64() == Some(owner.fence.authority_generation())
        && context["lease_id"].as_str() == Some(owner.fence.lease_id())
        && context["lease_epoch"].as_u64() == Some(owner.fence.lease_epoch())
}

fn with_host_operation(pending: &Value, ticket: Option<Value>, witness: Option<Value>) -> Value {
    let mut operation = pending.clone();
    operation["state"] = Value::String(if witness.is_some() {
        "SETTLED".to_owned()
    } else if ticket.is_some() {
        "ACCEPTED".to_owned()
    } else {
        "REJECTED".to_owned()
    });
    operation["ticket"] = ticket.unwrap_or(Value::Null);
    operation["witness"] = witness.unwrap_or(Value::Null);
    operation
}

fn operation_frame(request: &Value, status: &str, operation: Value) -> Vec<u8> {
    let principal = request["actor"]["principal_id"].clone();
    let body = json!({
        "contract": RECOVERY_CONTRACT,
        "schema_digest": RECOVERY_SCHEMA,
        "message_id": uuid(),
        "correlation_id": request["correlation_id"],
        "sent_at": timestamp(),
        "actor": {"principal_id": principal, "role": "host"},
        "auth": {"principal_id": request["actor"]["principal_id"], "capability": "operation_submit", "proof": Value::Null},
        "kind": if request["kind"] == "operation_intent_request" {
            "operation_intent_response"
        } else {
            "operation_dispatch_response"
        },
        "payload": {
            "result": {
                "status": status,
                "retryable": status == "UNKNOWN",
                "retry_after_seconds": if status == "UNKNOWN" { json!(1) } else { Value::Null },
            },
            "operation": operation,
        },
    });
    serde_json::to_vec(&body).unwrap_or_default()
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex(&Sha256::digest(bytes)))
}

fn decode_base64_no_pad(value: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in value.bytes() {
        let digit = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | u32::from(digit);
        bits = bits.saturating_add(6);
        if bits >= 8 {
            bits -= 8;
            output.push((buffer >> bits) as u8);
        }
    }
    if bits >= 6 || (bits > 0 && (buffer & ((1_u32 << bits) - 1)) != 0) {
        return None;
    }
    Some(output)
}

fn error_json(code: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({"error_code": code})).unwrap_or_default()
}
