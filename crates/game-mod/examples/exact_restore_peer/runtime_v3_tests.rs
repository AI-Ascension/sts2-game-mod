// SPDX-License-Identifier: MIT

use super::{ACTION_ID, EFFECT_KIND, RuntimeV3State, observation};
use crate::config::TransportConfig;
use crate::http_wire::Request;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture() -> (RuntimeV3State, TransportConfig, std::path::PathBuf) {
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
    let path = std::env::temp_dir().join(format!("sts2-runtime-v3-{suffix}"));
    std::fs::create_dir_all(&path).expect("store");
    let state = RuntimeV3State::open(transport.clone(), path.clone()).expect("state");
    (state, transport, path)
}

fn request(
    transport: &TransportConfig,
    path: &str,
    body: serde_json::Value,
    stale: bool,
) -> Request {
    let mut headers = BTreeMap::from([
        ("x-sts2-instance-id".into(), transport.instance_id.clone()),
        ("x-sts2-session-id".into(), transport.session_id.clone()),
        ("x-sts2-lease-id".into(), transport.lease_id.clone()),
        (
            "x-sts2-lease-epoch".into(),
            transport.lease_epoch.to_string(),
        ),
    ]);
    if stale {
        headers.insert("x-sts2-lease-epoch".into(), "99".into());
    }
    Request {
        method: if path.ends_with("/state") || path.ends_with("legal-actions") {
            "GET".into()
        } else {
            "POST".into()
        },
        path: path.into(),
        headers,
        body: serde_json::to_vec(&body).expect("body"),
    }
}

#[test]
fn fixture_has_one_legal_action_and_settled_successor() {
    assert_eq!(observation(0)["state"]["state"], "combat");
    assert_eq!(observation(1)["state"]["state"], "victory");
    assert_eq!(ACTION_ID, "combat.end-turn");
    assert_eq!(EFFECT_KIND, "combat.end-turn_settled");
}

#[test]
fn observe_dispatch_wait_persists_one_settled_effect() {
    let (mut state, transport, path) = fixture();
    let state_body = json!({
        "kind":"state_request","correlation_id":"corr-1"
    });
    let (status, body) = state.handle(&request(
        &transport,
        "/api/v3/runtime/state",
        state_body,
        false,
    ));
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("response")["generation"],
        0
    );
    let dispatch = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-2",
        "operation_id":"op-1","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    let (status, body) = state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        dispatch,
        false,
    ));
    assert_eq!(status, 200);
    let response: serde_json::Value = serde_json::from_slice(&body).expect("response");
    assert_eq!(response["status"], "settled");
    assert_eq!(response["generation"], 1);
    assert_eq!(response["transition"]["effect_kind"], EFFECT_KIND);
    let wait = json!({
        "kind":"wait_request","correlation_id":"corr-3",
        "operation_id":"op-1","wait_for_millis":1000
    });
    let (status, body) = state.handle(&request(&transport, "/api/v3/runtime/wait", wait, false));
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("response")["wait_outcome"],
        "successor"
    );
    let ledger: serde_json::Value = serde_json::from_slice(
        &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
    )
    .expect("ledger json");
    assert_eq!(ledger["action_count"], 1);
    assert_eq!(ledger["settled_count"], 1);
}

#[test]
fn unknown_wait_requires_recovery_without_echoing_request_timeout() {
    let (mut state, transport, _path) = fixture();
    let wait = json!({
        "kind":"wait_request","correlation_id":"corr-unknown",
        "operation_id":"missing","wait_for_millis":1000
    });
    let (status, body) = state.handle(&request(&transport, "/api/v3/runtime/wait", wait, false));
    assert_eq!(status, 200);
    let response: Value = serde_json::from_slice(&body).expect("response");
    assert_eq!(response["status"], "unknown");
    assert_eq!(response["wait_outcome"], "recovery_required");
    assert!(response["wait_for_millis"].is_null());
}

#[test]
fn stale_fence_is_rejected_without_effect() {
    let (mut state, transport, path) = fixture();
    let dispatch = json!({
        "kind":"dispatch_action_request","correlation_id":"corr",
        "operation_id":"op-stale","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    let (status, body) = state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        dispatch,
        true,
    ));
    assert_eq!(status, 409);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&body).expect("error")["error_code"],
        "stale_fence"
    );
    assert!(!path.join("runtime-v3-effects.json").exists());
}

#[test]
fn duplicate_operation_is_idempotent_and_binds_payload_and_generation() {
    let (mut state, transport, path) = fixture();
    let dispatch = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-1",
        "operation_id":"op-1","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    let (status, _) = state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        dispatch.clone(),
        false,
    ));
    assert_eq!(status, 200);
    let mut retry = dispatch;
    retry["correlation_id"] = json!("corr-retry");
    let (status, body) = state.handle(&request(&transport, "/api/v3/runtime/action", retry, false));
    assert_eq!(status, 200);
    let response: Value = serde_json::from_slice(&body).expect("response");
    assert_eq!(response["correlation_id"], "corr-retry");
    let ledger: Value = serde_json::from_slice(
        &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
    )
    .expect("ledger json");
    assert_eq!(ledger["action_count"], 1);

    let mut changed = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-changed",
        "operation_id":"op-1","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":"combat.other","action":{"kind":"end_turn"}}
    });
    changed["action"]["action_id"] = json!("combat.other");
    let (_, body) = state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        changed,
        false,
    ));
    assert_eq!(
        serde_json::from_slice::<Value>(&body).expect("error")["error_code"],
        "runtime_v3_operation_conflict"
    );
}

#[test]
fn stale_generation_is_rejected_without_second_effect() {
    let (mut state, transport, path) = fixture();
    let dispatch = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-1",
        "operation_id":"op-1","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        dispatch,
        false,
    ));
    let stale = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-2",
        "operation_id":"op-2","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    let (status, body) = state.handle(&request(&transport, "/api/v3/runtime/action", stale, false));
    assert_eq!(status, 200);
    let response: Value = serde_json::from_slice(&body).expect("response");
    assert_eq!(response["status"], "rejected");
    assert_eq!(response["error_code"], "stale_generation");
    let ledger: Value = serde_json::from_slice(
        &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
    )
    .expect("ledger json");
    assert_eq!(ledger["action_count"], 1);
}

#[test]
fn reopened_operation_replays_original_generation_without_second_effect() {
    let (mut state, transport, path) = fixture();
    let dispatch = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-1",
        "operation_id":"op-reopen","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    let (status, _) = state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        dispatch.clone(),
        false,
    ));
    assert_eq!(status, 200);

    let mut reopened = RuntimeV3State::open(transport.clone(), path.clone()).expect("reopen");
    let mut retry = dispatch;
    retry["correlation_id"] = json!("corr-after-restart");
    let (status, body) =
        reopened.handle(&request(&transport, "/api/v3/runtime/action", retry, false));
    assert_eq!(status, 200);
    let response: Value = serde_json::from_slice(&body).expect("response");
    assert_eq!(response["correlation_id"], "corr-after-restart");
    assert_eq!(response["status"], "settled");
    let ledger: Value = serde_json::from_slice(
        &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
    )
    .expect("ledger json");
    assert_eq!(ledger["action_count"], 1);
    assert_eq!(ledger["operations"][0]["generation"], 0);
}

#[test]
fn terminal_successor_rejects_fresh_operation_without_second_effect() {
    let (mut state, transport, path) = fixture();
    let dispatch = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-1",
        "operation_id":"op-terminal","generation":0,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    state.handle(&request(
        &transport,
        "/api/v3/runtime/action",
        dispatch,
        false,
    ));
    let fresh = json!({
        "kind":"dispatch_action_request","correlation_id":"corr-2",
        "operation_id":"op-fresh","generation":1,"state_id":super::STATE_ID,
        "action":{"action_id":ACTION_ID,"action":{"kind":"end_turn"}}
    });
    let (status, body) = state.handle(&request(&transport, "/api/v3/runtime/action", fresh, false));
    assert_eq!(status, 200);
    let response: Value = serde_json::from_slice(&body).expect("response");
    assert_eq!(response["status"], "rejected");
    assert_eq!(response["error_code"], "stale_generation");
    let ledger: Value = serde_json::from_slice(
        &std::fs::read(path.join("runtime-v3-effects.json")).expect("ledger"),
    )
    .expect("ledger json");
    assert_eq!(ledger["action_count"], 1);
    assert_eq!(ledger["settled_count"], 1);
}

#[test]
fn malformed_persisted_ledger_fails_closed() {
    let transport = TransportConfig {
        principal: "harness".into(),
        token: "token".into(),
        instance_id: "instance-1".into(),
        session_id: "session-1".into(),
        lease_id: "lease-1".into(),
        lease_epoch: 1,
    };
    let path = std::env::temp_dir().join("sts2-runtime-v3-malformed-ledger");
    std::fs::create_dir_all(&path).expect("store");
    std::fs::write(
        path.join("runtime-v3-effects.json"),
        br#"{"action_count":1,"settled_count":0,"operations":[]}"#,
    )
    .expect("ledger");
    assert!(RuntimeV3State::open(transport, path).is_err());
}
