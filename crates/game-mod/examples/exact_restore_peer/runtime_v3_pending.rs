// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

const MAX_PENDING_OPERATIONS: usize = 16;
const MAX_PENDING_BYTES: u64 = 64 * 1024;

pub(crate) fn load(
    path: &Path,
    settled_operation_ids: &std::collections::BTreeSet<String>,
) -> Result<BTreeMap<String, Value>, String> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let size = std::fs::metadata(path)
        .map_err(|error| format!("stat runtime-v3 pending ledger: {error}"))?
        .len();
    if size > MAX_PENDING_BYTES {
        return Err(String::from(
            "runtime-v3 pending ledger exceeds its byte bound",
        ));
    }
    let bytes =
        std::fs::read(path).map_err(|error| format!("read runtime-v3 pending ledger: {error}"))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse runtime-v3 pending ledger: {error}"))?;
    if value["version"] != 1 {
        return Err(String::from("runtime-v3 pending ledger version is invalid"));
    }
    let items = value["operations"]
        .as_array()
        .ok_or_else(|| String::from("runtime-v3 pending operations must be an array"))?;
    if items.len() > MAX_PENDING_OPERATIONS {
        return Err(String::from(
            "runtime-v3 pending ledger exceeds operation bound",
        ));
    }
    let mut pending = BTreeMap::new();
    for item in items {
        let operation_id = item["operation_id"]
            .as_str()
            .ok_or_else(|| String::from("runtime-v3 pending operation_id is missing"))?;
        match item["state"].as_str() {
            None => {}
            Some("REJECTED" | "UNKNOWN")
                if item["ticket"].is_null() && item["witness"].is_null() => {}
            Some("SETTLED")
                if item["ticket"].is_object()
                    && item["witness"].is_object()
                    && settled_operation_ids.contains(operation_id)
                    && receipt_identity_matches(item) => {}
            _ => {
                return Err(String::from(
                    "runtime-v3 pending operation state is invalid",
                ));
            }
        }
        if pending
            .insert(operation_id.to_owned(), item.clone())
            .is_some()
        {
            return Err(String::from(
                "runtime-v3 pending ledger has duplicate operation IDs",
            ));
        }
    }
    Ok(pending)
}

pub(crate) fn remember(
    pending: &mut BTreeMap<String, Value>,
    path: &Path,
    operation_id: &str,
    operation: Value,
) -> Result<bool, String> {
    let mut next = pending.clone();
    if next.len() >= MAX_PENDING_OPERATIONS && !next.contains_key(operation_id) {
        return Err(String::from("runtime-v3 pending ledger is full"));
    }
    if let Some(existing) = next.get(operation_id) {
        if !identity_matches(existing, &operation) {
            return Err(String::from("runtime-v3 pending operation conflict"));
        }
        if operation["state"].is_null() && existing["state"].is_string() {
            return Ok(false);
        }
    }
    let inserted = next.insert(operation_id.to_owned(), operation).is_none();
    persist(&next, path)?;
    *pending = next;
    Ok(inserted)
}

fn persist(pending: &BTreeMap<String, Value>, path: &Path) -> Result<(), String> {
    let operations: Vec<&Value> = pending.values().collect();
    let bytes = serde_json::to_vec(&json!({
        "version": 1,
        "operations": operations,
    }))
    .map_err(|error| format!("encode runtime-v3 pending ledger: {error}"))?;
    if bytes.len() as u64 > MAX_PENDING_BYTES {
        return Err(String::from(
            "runtime-v3 pending ledger exceeds its byte bound",
        ));
    }
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes)
        .map_err(|error| format!("write runtime-v3 pending ledger: {error}"))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| format!("replace runtime-v3 pending ledger: {error}"))
}

fn identity_matches(existing: &Value, candidate: &Value) -> bool {
    existing["operation_id"] == candidate["operation_id"]
        && existing["payload_digest"] == candidate["payload_digest"]
        && existing["original_context"] == candidate["original_context"]
        && existing["expected_boundary"] == candidate["expected_boundary"]
        && existing["action"] == candidate["action"]
}

fn receipt_identity_matches(operation: &Value) -> bool {
    let context = &operation["original_context"];
    let boundary = &operation["expected_boundary"];
    let ticket = &operation["ticket"];
    let witness = &operation["witness"];
    let matches_common = |value: &Value, require_epoch: bool| {
        value["operation_id"].as_str() == operation["operation_id"].as_str()
            && value["payload_digest"].as_str() == operation["payload_digest"].as_str()
            && value["boot_id"].as_str() == context["boot_id"].as_str()
            && value["instance_incarnation"].as_str() == context["instance_incarnation"].as_str()
            && (!require_epoch || value["lease_epoch"].as_u64() == context["lease_epoch"].as_u64())
            && value["host_fence_id"]
                .as_str()
                .is_some_and(|id| !id.is_empty())
    };
    matches_common(ticket, true)
        && matches_common(witness, false)
        && witness["state_id"] == boundary["state_id"]
        && witness["generation"] == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_persist_does_not_mutate_pending_map() {
        let path = std::env::temp_dir().join(format!(
            "sts2-runtime-v3-pending-dir-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("directory");
        let mut pending = BTreeMap::new();
        let operation = json!({"operation_id": "op-1"});
        let result = remember(&mut pending, &path, "op-1", operation);
        assert!(result.is_err());
        assert!(pending.is_empty());
        let _ = std::fs::remove_file(path.with_extension("json.tmp"));
        std::fs::remove_dir_all(path).expect("cleanup");
    }
}
