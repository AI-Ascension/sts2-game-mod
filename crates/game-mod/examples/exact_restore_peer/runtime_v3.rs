// SPDX-License-Identifier: MIT
//
//! Bounded test-only Runtime-v3 peer used by the actual Gateway/MCP fixture.
//! It records one legal end-turn effect and persists host operation intent.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::{Value, json};

use super::config::TransportConfig;
use super::http_wire::Request;
use super::runtime_v3_wire::*;

pub(crate) const SCHEMA_DIGEST: &str =
    "8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63";
pub(super) const STATE_ID: &str = "00000000-0000-4000-8000-000000000001";
pub(super) const ACTION_ID: &str = "combat.end-turn";
pub(super) const EFFECT_KIND: &str = "combat.end-turn_settled";

pub(crate) struct RuntimeV3State {
    transport: TransportConfig,
    store_path: PathBuf,
    pending_path: PathBuf,
    generation: u64,
    operations: BTreeMap<String, Operation>,
    pending_operations: BTreeMap<String, Value>,
}

#[derive(Clone)]
struct Operation {
    action_id: String,
    action: Value,
    generation: u64,
    response: Value,
}

impl RuntimeV3State {
    pub(crate) fn open(transport: TransportConfig, store_dir: PathBuf) -> Result<Self, String> {
        let store_path = store_dir.join("runtime-v3-effects.json");
        let pending_path = store_dir.join("runtime-v3-pending.json");
        let operations: BTreeMap<String, Operation> =
            if store_path.exists() {
                let size = std::fs::metadata(&store_path)
                    .map_err(|error| format!("stat runtime-v3 ledger: {error}"))?
                    .len();
                if size > 64 * 1024 {
                    return Err(String::from("runtime-v3 ledger exceeds its byte bound"));
                }
                let bytes = std::fs::read(&store_path)
                    .map_err(|error| format!("read runtime-v3 ledger: {error}"))?;
                let value: Value = serde_json::from_slice(&bytes)
                    .map_err(|error| format!("decode runtime-v3 ledger: {error}"))?;
                let items = value["operations"]
                    .as_array()
                    .ok_or_else(|| String::from("runtime-v3 ledger operations must be an array"))?;
                let action_count = value["action_count"]
                    .as_u64()
                    .ok_or_else(|| String::from("runtime-v3 ledger action_count is missing"))?;
                let settled_count = value["settled_count"]
                    .as_u64()
                    .ok_or_else(|| String::from("runtime-v3 ledger settled_count is missing"))?;
                let top_action_id = value["action_id"]
                    .as_str()
                    .ok_or_else(|| String::from("runtime-v3 ledger action_id is missing"))?;
                if items.len() > 1
                    || action_count != items.len() as u64
                    || settled_count != action_count
                {
                    return Err(String::from(
                        "runtime-v3 ledger counters do not match operations",
                    ));
                }
                let mut operations = BTreeMap::new();
                for item in items {
                    let operation_id = item["operation_id"]
                        .as_str()
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| String::from("runtime-v3 ledger operation_id is missing"))?
                        .to_owned();
                    let action_id = item["action_id"]
                        .as_str()
                        .filter(|value| !value.is_empty())
                        .ok_or_else(|| String::from("runtime-v3 ledger action_id is missing"))?
                        .to_owned();
                    let generation = item["generation"]
                        .as_u64()
                        .ok_or_else(|| String::from("runtime-v3 ledger generation is missing"))?;
                    if item["status"].as_str() != Some("settled")
                        || item["effect_kind"].as_str() != Some(EFFECT_KIND)
                        || action_id != ACTION_ID
                        || generation != 0
                        || top_action_id != action_id
                    {
                        return Err(String::from("runtime-v3 ledger operation is invalid"));
                    }
                    if operations.insert(
                        operation_id.clone(),
                        Operation {
                            action: json!({"action_id": action_id, "action": {"kind":"end_turn"}}),
                            action_id,
                            generation,
                            response: settled_response("replayed", &transport, &operation_id),
                        },
                    ).is_some() {
                        return Err(String::from("runtime-v3 ledger has duplicate operation IDs"));
                    }
                }
                operations
            } else {
                BTreeMap::new()
            };
        let pending_operations = super::runtime_v3_pending::load(&pending_path)?;
        let generation = operations
            .values()
            .next()
            .map_or(0, |operation| operation.generation.saturating_add(1));
        Ok(Self {
            transport,
            store_path,
            pending_path,
            generation,
            operations,
            pending_operations,
        })
    }

    pub(crate) fn remember_pending(
        &mut self,
        operation_id: &str,
        operation: Value,
    ) -> Result<bool, String> {
        super::runtime_v3_pending::remember(
            &mut self.pending_operations,
            &self.pending_path,
            operation_id,
            operation,
        )
    }

    pub(crate) fn pending_operation(&self, operation_id: &str) -> Option<Value> {
        self.pending_operations.get(operation_id).cloned()
    }

    pub(crate) fn dispatch_operation(&mut self, request: Value) -> Value {
        self.dispatch_response(&request)
    }

    pub(crate) fn update_transport(
        &mut self,
        instance_id: String,
        session_id: String,
        lease_id: String,
        lease_epoch: u64,
    ) {
        self.transport.instance_id = instance_id;
        self.transport.session_id = session_id;
        self.transport.lease_id = lease_id;
        self.transport.lease_epoch = lease_epoch;
        for operation in self.operations.values_mut() {
            operation.response["instance_id"] = Value::String(self.transport.instance_id.clone());
            operation.response["session_id"] = Value::String(self.transport.session_id.clone());
            operation.response["lease_id"] = Value::String(self.transport.lease_id.clone());
            operation.response["lease_epoch"] = json!(self.transport.lease_epoch);
        }
    }

    pub(crate) fn handle(&mut self, request: &Request) -> (u16, Vec<u8>) {
        self.log_request(request);
        if !self.fence_matches(request) {
            return (409, json_error("stale_fence"));
        }
        let Ok(frame) = serde_json::from_slice::<Value>(&request.body) else {
            return (400, json_error("runtime_v3_body_invalid"));
        };
        let expected = match request.path.as_str() {
            "/api/v3/runtime/state" => "state_request",
            "/api/v3/runtime/legal-actions" => "legal_actions_request",
            "/api/v3/runtime/action" => "dispatch_action_request",
            "/api/v3/runtime/wait" => "wait_request",
            "/api/v3/runtime/reobserve" => "reobserve_request",
            "/api/v3/runtime/recover" => "recover_request",
            _ => return (404, Vec::new()),
        };
        if frame["kind"].as_str() != Some(expected) {
            return (400, json_error("runtime_v3_kind_invalid"));
        }
        let response = match expected {
            "state_request" => self.state_response(&frame, "state_response"),
            "reobserve_request" => self.state_response(&frame, "reobserve_response"),
            "legal_actions_request" => self.legal_actions_response(&frame),
            "dispatch_action_request" => self.dispatch_response(&frame),
            "wait_request" => self.wait_response(&frame),
            "recover_request" => self.recover_response(&frame),
            _ => unreachable!(),
        };
        let body = serde_json::to_vec(&response).unwrap_or_default();
        self.log_response(request, 200, &body);
        (200, body)
    }

    fn log_request(&self, request: &Request) {
        let path = self
            .store_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("runtime-v3-requests.log");
        let line = format!(
            "{} {} {}\n",
            request.method,
            request.path,
            String::from_utf8_lossy(&request.body)
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
    }

    fn log_response(&self, request: &Request, status: u16, body: &[u8]) {
        let path = self
            .store_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("runtime-v3-responses.log");
        let line = format!(
            "{} {} {}\n",
            status,
            request.path,
            String::from_utf8_lossy(body)
        );
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| std::io::Write::write_all(&mut file, line.as_bytes()));
    }

    fn fence_matches(&self, request: &Request) -> bool {
        let header = |name: &str| request.headers.get(name).map(String::as_str);
        let epoch = header("x-sts2-lease-epoch").and_then(|value| value.parse().ok());
        header("x-sts2-instance-id") == Some(self.transport.instance_id.as_str())
            && header("x-sts2-session-id") == Some(self.transport.session_id.as_str())
            && header("x-sts2-lease-id") == Some(self.transport.lease_id.as_str())
            && epoch == Some(self.transport.lease_epoch)
    }

    fn state_response(&self, request: &Value, kind: &str) -> Value {
        let generation = self.generation;
        envelope(
            &self.transport,
            request,
            kind,
            generation,
            Some(observation(generation)),
            Some(legal_actions(generation)),
            None,
            None,
            None,
            None,
        )
    }

    fn legal_actions_response(&self, request: &Value) -> Value {
        let generation = self.generation;
        let actions = if generation == 0 {
            Some(legal_actions(generation))
        } else {
            Some(Vec::new())
        };
        envelope(
            &self.transport,
            request,
            "legal_actions_response",
            generation,
            None,
            actions,
            None,
            None,
            None,
            None,
        )
    }

    fn dispatch_response(&mut self, request: &Value) -> Value {
        let Some(operation_id) = request["operation_id"]
            .as_str()
            .filter(|value| !value.is_empty())
        else {
            return json_error_value("runtime_v3_operation_id_required");
        };
        let Some(action) = request["action"].as_object() else {
            return json_error_value("runtime_v3_action_required");
        };
        let Some(action_id) = action
            .get("action_id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            return json_error_value("runtime_v3_action_id_required");
        };
        if request["state_id"].as_str() != Some(STATE_ID)
            || action.get("action") != Some(&json!({"kind": "end_turn"}))
        {
            return json_error_value("runtime_v3_action_invalid");
        }
        if let Some(existing) = self.operations.get(operation_id) {
            if existing.action_id != action_id
                || existing.action != request["action"]
                || existing.generation != request["generation"].as_u64().unwrap_or(u64::MAX)
            {
                return json_error_value("runtime_v3_operation_conflict");
            }
            let mut response = existing.response.clone();
            response["correlation_id"] = request["correlation_id"].clone();
            return response;
        }
        let Some(generation) = request["generation"].as_u64() else {
            return json_error_value("runtime_v3_generation_required");
        };
        if action_id != ACTION_ID || generation != self.generation || self.generation != 0 {
            return rejected_response(&self.transport, request, operation_id, "stale_generation");
        }
        let response = settled_response(
            request["correlation_id"].as_str().unwrap_or("corr"),
            &self.transport,
            operation_id,
        );
        let old_generation = self.generation;
        self.generation = old_generation.saturating_add(1);
        self.operations.insert(
            operation_id.to_owned(),
            Operation {
                action_id: action_id.to_owned(),
                action: request["action"].clone(),
                generation,
                response: response.clone(),
            },
        );
        if let Err(error) = self.persist() {
            self.operations.remove(operation_id);
            self.generation = old_generation;
            return json_error_value(&format!("runtime_v3_persist_failed:{error}"));
        }
        response
    }

    fn wait_response(&self, request: &Value) -> Value {
        let operation_id = request["operation_id"]
            .as_str()
            .unwrap_or("operation-missing");
        if self.operations.contains_key(operation_id) {
            let mut response = settled_response(
                request["correlation_id"].as_str().unwrap_or("corr"),
                &self.transport,
                operation_id,
            );
            response["kind"] = Value::String("wait_response".to_owned());
            response["wait_for_millis"] = request["wait_for_millis"].clone();
            response["wait_outcome"] = Value::String("successor".to_owned());
            return response;
        }
        unknown_response(&self.transport, request, operation_id)
    }

    fn recover_response(&self, request: &Value) -> Value {
        let operation_id = request["recovery"]["operation_id"].clone();
        let mut response = envelope(
            &self.transport,
            request,
            "recover_response",
            self.generation,
            Some(observation(self.generation)),
            Some(if self.generation == 0 {
                legal_actions(self.generation)
            } else {
                Vec::new()
            }),
            None,
            None,
            Some("accepted"),
            None,
        );
        response["operation_id"] = operation_id;
        response
    }

    fn persist(&self) -> Result<(), String> {
        let operations: Vec<Value> = self
            .operations
            .iter()
            .map(|(operation_id, operation)| {
                json!({
                    "operation_id": operation_id,
                    "action_id": operation.action_id,
                    "status": "settled",
                    "generation": operation.generation,
                    "effect_kind": EFFECT_KIND
                })
            })
            .collect();
        let body = json!({
            "action_count": operations.len(),
            "settled_count": operations.len(),
            "action_id": operations.first().and_then(|item| item["action_id"].as_str()),
            "operations": operations
        });
        std::fs::write(
            &self.store_path,
            serde_json::to_vec_pretty(&body).unwrap_or_default(),
        )
        .map_err(|error| format!("write runtime-v3 ledger: {error}"))
    }
}

#[cfg(test)]
#[path = "runtime_v3_tests.rs"]
mod tests;
