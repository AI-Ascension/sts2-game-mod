// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

const MAX_PENDING_OPERATIONS: usize = 16;
const MAX_PENDING_BYTES: u64 = 64 * 1024;

pub(crate) fn load(path: &Path) -> Result<BTreeMap<String, Value>, String> {
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
    if pending.len() >= MAX_PENDING_OPERATIONS && !pending.contains_key(operation_id) {
        return Err(String::from("runtime-v3 pending ledger is full"));
    }
    if let Some(existing) = pending.get(operation_id) {
        if !identity_matches(existing, &operation) {
            return Err(String::from("runtime-v3 pending operation conflict"));
        }
        if operation["state"].is_null() && existing["state"].is_string() {
            return Ok(false);
        }
    }
    let inserted = pending.insert(operation_id.to_owned(), operation).is_none();
    persist(pending, path)?;
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
