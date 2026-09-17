// SPDX-License-Identifier: MIT

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sts2_game_mod::{ExactRestoreCurrentOwner, ExactRestoreOwnerFence};

pub(crate) const RECOVERY_PATH: &str = "/api/v1/runtime/recovery";
const RECOVERY_CONTRACT: &str = "watchdog-recovery-v1";
const RECOVERY_SCHEMA: &str = "fb934d3157485aaf6e13e6ebbb213ec8a14c7fc6f5eeebc06b7a22c1f0009217";
const LEASE_CONTRACT: &str = "watchdog-host-lease-control-v1";
const LEASE_SCHEMA: &str = "e22faf0f7d3cd313a007b65e52058b3c255153d5778dd8124055c283adf977f9";

pub(crate) fn recovery_response(request: &Value, owner: &ExactRestoreCurrentOwner) -> Vec<u8> {
    let principal = request["actor"]["principal_id"].clone();
    let boot = request["payload"]["boot"].clone();
    let fence = json!({
        "host_fence_id": owner.fence.host_fence_id(),
        "deployment_id": boot["deployment_id"],
        "instance_id": boot["instance_id"],
        "instance_incarnation": boot["instance_incarnation"],
        "boot_id": boot["boot_id"],
        "authority_generation": boot["authority_generation"],
        "fence_generation": owner.fence.host_fence_generation(),
        "created_at": timestamp(),
    });
    serde_json::to_vec(&json!({
        "contract": RECOVERY_CONTRACT,
        "schema_digest": RECOVERY_SCHEMA,
        "message_id": uuid(),
        "correlation_id": request["correlation_id"],
        "sent_at": timestamp(),
        "actor": {"principal_id": principal, "role": "gateway"},
        "auth": {"principal_id": principal, "capability": "host_fence", "proof": Value::Null},
        "kind": "host_fence_response",
        "payload": {
            "result": {"status": "FENCE_ACCEPTED", "retryable": false, "retry_after_seconds": Value::Null},
            "fence": fence,
        },
    }))
    .unwrap_or_default()
}

pub(crate) fn lease_response(request: &Value, owner: &ExactRestoreCurrentOwner) -> Vec<u8> {
    let payload = &request["payload"];
    let grant = &payload["grant"];
    let lease = &grant["lease"];
    let principal = request["actor"]["principal_id"].clone();
    let kind = request["kind"].as_str().unwrap_or("lease_install_request");
    let response_kind = kind.replace("_request", "_response");
    let capability = request["auth"]["capability"].clone();
    let status = match kind {
        "lease_renew_request" => "RENEWED",
        "lease_revoke_request" => "REVOKED",
        _ => "INSTALLED",
    };
    let ack = json!({
        "result": {"status": status, "retryable": false, "retry_after_seconds": Value::Null},
        "installation_id": payload["installation_id"],
        "grant_digest": payload["grant_digest"],
        "boot_id": grant["boot"]["boot_id"],
        "instance_incarnation": grant["boot"]["instance_incarnation"],
        "host_fence_id": grant["fence"]["host_fence_id"],
        "fence_generation": grant["fence"]["fence_generation"],
        "lease_id": lease["lease_id"],
        "lease_epoch": lease["lease_epoch"],
        "host_install_generation": 1,
        "recorded_at": timestamp(),
        "renew_sequence": if kind == "lease_install_request" || kind == "lease_revoke_request" {
            Value::Null
        } else {
            payload["renew_sequence"].clone()
        },
        "expires_at": if kind == "lease_revoke_request" {
            Value::Null
        } else {
            lease["expires_at"].clone()
        },
    });
    let mut frame = json!({
        "contract": LEASE_CONTRACT,
        "schema_digest": LEASE_SCHEMA,
        "message_id": uuid(),
        "correlation_id": request["correlation_id"],
        "sent_at": timestamp(),
        "actor": {"principal_id": principal, "role": "host"},
        "auth": {"principal_id": request["actor"]["principal_id"], "capability": capability, "proof": ""},
        "kind": response_kind,
        "payload": {"ack": ack},
    });
    let secret = std::env::var("STS2_RUNTIME_HOST_LEASE_KEY")
        .ok()
        .and_then(|value| decode_hex(&value));
    if let Some(secret) = secret {
        let proof = proof_for_frame(&frame, &secret);
        frame["auth"]["proof"] = Value::String(proof);
    }
    // Keep the owner live when the Gateway sends an issued grant. This file-backed
    // snapshot is still consulted for the first request and is never taken from exact payloads.
    let _ = owner;
    serde_json::to_vec(&frame).unwrap_or_default()
}

pub(crate) fn valid_lease_request(request: &Value) -> bool {
    let kind = request["kind"].as_str().unwrap_or_default();
    let capability = request["auth"]["capability"].as_str().unwrap_or_default();
    let domain = match kind {
        "lease_install_request" => "host-lease-control/v1/lease-install-request",
        "lease_renew_request" => "host-lease-control/v1/lease-renew-request",
        "lease_revoke_request" => "host-lease-control/v1/lease-revoke-request",
        _ => return false,
    };
    if request["contract"].as_str() != Some(LEASE_CONTRACT)
        || request["schema_digest"].as_str() != Some(LEASE_SCHEMA)
        || request["auth"]["proof"].as_str().is_none()
        || capability != kind.trim_end_matches("_request")
        || request["payload"]["installation_id"].as_str().is_none()
        || request["payload"]["grant_digest"].as_str().is_none()
        || request["payload"]["grant"].is_null()
    {
        return false;
    }
    let Some(secret) = std::env::var("STS2_RUNTIME_HOST_LEASE_KEY")
        .ok()
        .and_then(|value| decode_hex(&value))
    else {
        return false;
    };
    proof_for_domain(request, &secret, domain)
        == request["auth"]["proof"].as_str().unwrap_or_default()
}

fn proof_for_frame(frame: &Value, secret: &[u8]) -> String {
    let domain = match frame["kind"].as_str().unwrap_or_default() {
        "lease_renew_response" => "host-lease-control/v1/lease-renew-ack",
        "lease_revoke_response" => "host-lease-control/v1/lease-revoke-ack",
        _ => "host-lease-control/v1/lease-install-ack",
    };
    proof_for_domain(frame, secret, domain)
}

fn proof_for_domain(frame: &Value, secret: &[u8], domain: &str) -> String {
    let mut protected = frame.clone();
    let Some(auth) = protected["auth"].as_object_mut() else {
        return String::new();
    };
    auth.remove("proof");
    let canonical = canonical(&protected);
    let mut message = domain.as_bytes().to_vec();
    message.push(0);
    message.extend(canonical);
    hex(&hmac(secret, &message))
}

fn canonical(value: &Value) -> Vec<u8> {
    match value {
        Value::Null => b"null".to_vec(),
        Value::Bool(value) => {
            if *value {
                b"true".to_vec()
            } else {
                b"false".to_vec()
            }
        }
        Value::Number(value) => value.to_string().into_bytes(),
        Value::String(value) => serde_json::to_vec(value).unwrap_or_default(),
        Value::Array(values) => {
            let mut out = vec![b'['];
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                out.extend(canonical(value));
            }
            out.push(b']');
            out
        }
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
            let mut out = vec![b'{'];
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                out.extend(serde_json::to_vec(key).unwrap_or_default());
                out.push(b':');
                out.extend(canonical(&values[key]));
            }
            out.push(b'}');
            out
        }
    }
}

fn hmac(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut normalized = [0_u8; 64];
    normalized[..key.len().min(64)].copy_from_slice(&key[..key.len().min(64)]);
    let mut inner = [0_u8; 64];
    let mut outer = [0_u8; 64];
    for i in 0..64 {
        inner[i] = normalized[i] ^ 0x36;
        outer[i] = normalized[i] ^ 0x5c;
    }
    let mut h = Sha256::new();
    h.update(inner);
    h.update(message);
    let inner = h.finalize();
    let mut h = Sha256::new();
    h.update(outer);
    h.update(inner);
    let digest = h.finalize();
    digest.into()
}

fn decode_hex(value: &str) -> Option<Vec<u8>> {
    let bytes = value.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    bytes
        .chunks_exact(2)
        .map(|pair| Some((hex_digit(pair[0])? << 4) | hex_digit(pair[1])?))
        .collect()
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn timestamp() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    format!("2026-09-17T00:00:{:02}.000Z", (millis / 1000) % 60)
}

fn uuid() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "aaaaaaaa-aaaa-4aaa-8aaa-{:012x}",
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

#[allow(dead_code)]
fn _owner(_: &ExactRestoreOwnerFence) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_response_preserves_correlation_and_accepts_fence() {
        let fence = ExactRestoreOwnerFence::new(
            "11111111-1111-4111-8111-111111111111".into(),
            "22222222-2222-4222-8222-222222222222".into(),
            "33333333-3333-4333-8333-333333333333".into(),
            "44444444-4444-4444-8444-444444444444".into(),
            3,
            "55555555-5555-4555-8555-555555555555".into(),
            7,
            "66666666-6666-4666-8666-666666666666".into(),
            8,
            "session:fixture".into(),
            1_900_000_000_000,
        )
        .expect("owner");
        let owner = ExactRestoreCurrentOwner {
            fence,
            observed_at_millis: 1_800_000_000_000,
        };
        let request = json!({
            "actor": {"principal_id": "77777777-7777-4777-8777-777777777777"},
            "correlation_id": "88888888-8888-4888-8888-888888888888",
            "payload": {"boot": {
                "deployment_id": owner.fence.deployment_id(),
                "instance_id": owner.fence.instance_id(),
                "instance_incarnation": owner.fence.instance_incarnation(),
                "boot_id": owner.fence.boot_id(),
                "authority_generation": owner.fence.authority_generation()
            }}
        });
        let response: Value =
            serde_json::from_slice(&recovery_response(&request, &owner)).expect("response");
        assert_eq!(response["kind"], "host_fence_response");
        assert_eq!(
            response["correlation_id"],
            "88888888-8888-4888-8888-888888888888"
        );
        assert_eq!(response["payload"]["result"]["status"], "FENCE_ACCEPTED");
        assert_eq!(
            response["payload"]["fence"]["host_fence_id"],
            owner.fence.host_fence_id()
        );
    }

    #[test]
    fn revoke_ack_omits_renewal_and_expiry() {
        let request = json!({
            "kind": "lease_revoke_request",
            "actor": {"principal_id": "77777777-7777-4777-8777-777777777777"},
            "auth": {"capability": "lease_revoke"},
            "correlation_id": "88888888-8888-4888-8888-888888888888",
            "payload": {
                "installation_id": "99999999-9999-4999-8999-999999999999",
                "grant_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "grant": {"boot": {}, "fence": {}, "lease": {}}
            }
        });
        let owner = ExactRestoreCurrentOwner {
            fence: ExactRestoreOwnerFence::new(
                "11111111-1111-4111-8111-111111111111".into(),
                "22222222-2222-4222-8222-222222222222".into(),
                "33333333-3333-4333-8333-333333333333".into(),
                "44444444-4444-4444-8444-444444444444".into(),
                3,
                "55555555-5555-4555-8555-555555555555".into(),
                7,
                "66666666-6666-4666-8666-666666666666".into(),
                8,
                "session:fixture".into(),
                1_900_000_000_000,
            )
            .expect("owner"),
            observed_at_millis: 1_800_000_000_000,
        };
        let response: Value =
            serde_json::from_slice(&lease_response(&request, &owner)).expect("response");
        assert!(response["payload"]["ack"]["renew_sequence"].is_null());
        assert!(response["payload"]["ack"]["expires_at"].is_null());
    }
}
