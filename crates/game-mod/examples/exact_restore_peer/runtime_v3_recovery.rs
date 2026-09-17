// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sts2_game_mod::ExactRestoreCurrentOwner;

use super::recovery::{
    RECOVERY_CONTRACT, RECOVERY_SCHEMA, digest_value, timestamp, timestamp_after_seconds, uuid,
};
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
        let pending = Value::Object(operation.clone());
        let inserted = match runtime.remember_pending(operation_id, pending.clone()) {
            Ok(inserted) => inserted,
            Err(error) => return (409, error_json(&error)),
        };
        return (
            200,
            operation_frame(
                request,
                if inserted {
                    "INTENT_RECORDED"
                } else {
                    "DUPLICATE"
                },
                runtime.pending_operation(operation_id).unwrap_or(pending),
            ),
        );
    }
    let Some(pending) = runtime.pending_operation(operation_id) else {
        return (404, error_json("recovery_operation_not_found"));
    };
    if !operation_reference_matches(&pending, operation) {
        return (409, error_json("recovery_operation_conflict"));
    }
    if pending["state"].as_str() == Some("SETTLED")
        && pending["ticket"].is_object()
        && pending["witness"].is_object()
    {
        return (200, operation_frame(request, "DUPLICATE", pending));
    }
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
        let host_operation = with_host_operation(&pending, host_status, None, None);
        if let Err(error) = runtime.remember_pending(operation_id, host_operation.clone()) {
            return (
                503,
                error_json(&format!("recovery_pending_persist_failed:{error}")),
            );
        }
        return (
            if host_status == "UNKNOWN" { 503 } else { 200 },
            operation_frame(request, host_status, host_operation),
        );
    }
    let effect_digest = digest_value(&pending["action"]["canonical_json_b64"]);
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
        "expires_at": timestamp_after_seconds(5),
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
    let host_operation = with_host_operation(&pending, "SETTLED", Some(ticket), Some(witness));
    if let Err(error) = runtime.remember_pending(operation_id, host_operation.clone()) {
        return (
            503,
            error_json(&format!("recovery_pending_persist_failed:{error}")),
        );
    }
    (200, operation_frame(request, "SETTLED", host_operation))
}

fn operation_reference_matches(
    pending: &Value,
    reference: &serde_json::Map<String, Value>,
) -> bool {
    pending["operation_id"] == reference["operation_id"]
        && pending["payload_digest"] == reference["payload_digest"]
        && pending["original_context"] == reference["original_context"]
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

fn with_host_operation(
    pending: &Value,
    state: &str,
    ticket: Option<Value>,
    witness: Option<Value>,
) -> Value {
    let mut operation = pending.clone();
    operation["state"] = Value::String(state.to_owned());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TransportConfig;
    use crate::recovery::{valid_effect_digest, valid_ticket_window};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn state() -> (RuntimeV3State, TransportConfig, std::path::PathBuf) {
        let transport = TransportConfig {
            principal: "harness".into(),
            token: "token".into(),
            instance_id: "instance-1".into(),
            session_id: "session-1".into(),
            lease_id: "lease-1".into(),
            lease_epoch: 1,
        };
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("sts2-runtime-v3-pending-{suffix}"));
        std::fs::create_dir_all(&path).expect("store");
        (
            RuntimeV3State::open(transport.clone(), path.clone()).expect("state"),
            transport,
            path,
        )
    }

    fn operation() -> Value {
        json!({
            "operation_id": "operation-1",
            "payload_digest": "sha256:payload",
            "original_context": {
                "deployment_id": "deployment-1",
                "instance_id": "instance-1",
                "instance_incarnation": "incarnation-1",
                "boot_id": "boot-1",
                "authority_generation": 1,
                "lease_id": "lease-1",
                "lease_epoch": 1
            },
            "expected_boundary": {
                "state_id": STATE_ID,
                "generation": 0,
                "catalog_digest": "catalog-1"
            },
            "action": {
                "schema_digest": "schema-1",
                "canonical_json_b64": "eyJhY3Rpb24iOnsia2luZCI6ImVuZF90dXJuIn0sImFjdGlvbl9pZCI6ImNvbWJhdC5lbmQtdHVybiJ9",
                "payload_digest": "sha256:payload"
            }
        })
    }

    #[test]
    fn pending_operation_replays_after_restart_and_rejects_mutation() {
        let (mut runtime, transport, path) = state();
        let pending = operation();
        assert!(
            runtime
                .remember_pending("operation-1", pending.clone())
                .expect("persist pending")
        );
        assert!(
            !runtime
                .remember_pending("operation-1", pending.clone())
                .expect("duplicate pending")
        );

        let mut reopened = RuntimeV3State::open(transport.clone(), path.clone()).expect("reopen");
        assert_eq!(
            reopened.pending_operation("operation-1"),
            Some(pending.clone())
        );
        let dispatch = json!({
            "kind": "dispatch_action_request",
            "operation_id": "operation-1",
            "generation": 0,
            "state_id": STATE_ID,
            "action": {
                "action_id": "combat.end-turn",
                "action": {"kind": "end_turn"}
            }
        });
        assert_eq!(reopened.dispatch_operation(dispatch)["status"], "settled");
        let ledger: Value = serde_json::from_slice(
            &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
        )
        .expect("ledger json");
        assert_eq!(ledger["action_count"], 1);

        let mut conflict = pending;
        conflict["payload_digest"] = json!("sha256:changed");
        assert!(
            reopened
                .remember_pending("operation-1", conflict)
                .expect_err("conflict")
                .contains("conflict")
        );
    }

    #[test]
    fn host_mux_replays_settled_witness_after_restart_without_second_effect() {
        let (mut runtime, transport, path) = state();
        let fence = sts2_game_mod::ExactRestoreOwnerFence::new(
            "11111111-1111-4111-8111-111111111111".into(),
            "22222222-2222-4222-8222-222222222222".into(),
            "33333333-3333-4333-8333-333333333333".into(),
            "44444444-4444-4444-8444-444444444444".into(),
            3,
            "55555555-5555-4555-8555-555555555555".into(),
            7,
            "66666666-6666-4666-8666-666666666666".into(),
            8,
            "77777777-7777-4777-8777-777777777777".into(),
            1_900_000_000_000,
        )
        .expect("owner");
        let owner = sts2_game_mod::ExactRestoreCurrentOwner {
            fence,
            observed_at_millis: 1_800_000_000_000,
        };
        let mut pending = operation();
        pending["original_context"]["deployment_id"] = json!(owner.fence.deployment_id());
        pending["original_context"]["instance_id"] = json!(owner.fence.instance_id());
        pending["original_context"]["instance_incarnation"] =
            json!(owner.fence.instance_incarnation());
        pending["original_context"]["boot_id"] = json!(owner.fence.boot_id());
        pending["original_context"]["authority_generation"] =
            json!(owner.fence.authority_generation());
        pending["original_context"]["lease_id"] = json!(owner.fence.lease_id());
        pending["original_context"]["lease_epoch"] = json!(owner.fence.lease_epoch());
        let request = json!({
            "kind": "operation_intent_request",
            "correlation_id": "88888888-8888-4888-8888-888888888888",
            "actor": {"principal_id": "77777777-7777-4777-8777-777777777777"},
            "payload": {"operation": pending}
        });
        let (status, _) = runtime_operation_response(&request, &owner, &mut runtime);
        assert_eq!(status, 200);
        let dispatch = json!({
            "kind": "operation_dispatch_request",
            "correlation_id": "99999999-9999-4999-8999-999999999999",
            "actor": {"principal_id": "77777777-7777-4777-8777-777777777777"},
            "payload": {"operation": {
                "operation_id": "operation-1",
                "payload_digest": "sha256:payload",
                "original_context": request["payload"]["operation"]["original_context"]
            }}
        });
        let (status, body) = runtime_operation_response(&dispatch, &owner, &mut runtime);
        assert_eq!(status, 200);
        let first: Value = serde_json::from_slice(&body).expect("settled response");
        assert_eq!(first["contract"], RECOVERY_CONTRACT);
        assert_eq!(first["schema_digest"], RECOVERY_SCHEMA);
        assert_eq!(first["kind"], "operation_dispatch_response");
        assert_eq!(first["correlation_id"], dispatch["correlation_id"]);
        assert_eq!(first["actor"]["role"], "host");
        assert_eq!(first["auth"]["capability"], "operation_submit");
        assert_eq!(first["payload"]["result"]["status"], "SETTLED");
        let ticket = first["payload"]["operation"]["ticket"].clone();
        let witness = first["payload"]["operation"]["witness"].clone();
        assert!(valid_ticket_window(&ticket));
        assert!(valid_effect_digest(&witness));

        let mut reopened = RuntimeV3State::open(transport, path.clone()).expect("reopen");
        let (status, body) = runtime_operation_response(&dispatch, &owner, &mut reopened);
        assert_eq!(status, 200);
        let duplicate: Value = serde_json::from_slice(&body).expect("duplicate response");
        assert_eq!(duplicate["payload"]["result"]["status"], "DUPLICATE");
        assert_eq!(duplicate["payload"]["operation"]["ticket"], ticket);
        assert_eq!(duplicate["payload"]["operation"]["witness"], witness);

        let mut conflict = dispatch;
        conflict["payload"]["operation"]["payload_digest"] = json!("sha256:changed");
        let (status, _) = runtime_operation_response(&conflict, &owner, &mut reopened);
        assert_eq!(status, 409);
        let ledger: Value = serde_json::from_slice(
            &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
        )
        .expect("ledger json");
        assert_eq!(ledger["action_count"], 1);
    }
}
