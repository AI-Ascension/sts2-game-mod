// SPDX-License-Identifier: MIT

//! Validate one game-information-query-v1 JSON envelope against the pinned
//! protocol artifact. The validator reads a single JSON value from stdin and
//! exits non-zero with each schema violation on stderr.

use std::io::{self, Read};

const SCHEMA_DIGEST: &str = "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";
const SCHEMA: &str =
    include_str!("../../../../../protocol-artifact/game-information-query-v1/schema.json");

fn main() -> Result<(), String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("read stdin: {error}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&input).map_err(|error| format!("invalid JSON: {error}"))?;
    let schema: serde_json::Value =
        serde_json::from_str(SCHEMA).map_err(|error| format!("invalid pinned schema: {error}"))?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| format!("schema compile failed: {error}"))?;
    let mut errors = validator.iter_errors(&value).peekable();
    if errors.peek().is_some() {
        for error in errors {
            eprintln!("{error}");
        }
        return Err(format!("schema digest {SCHEMA_DIGEST} rejected response"));
    }
    semantic_checks(&value)?;
    println!("valid game-information-query-v1 ({SCHEMA_DIGEST})");
    Ok(())
}

fn semantic_checks(value: &serde_json::Value) -> Result<(), String> {
    if value.get("kind").and_then(serde_json::Value::as_str) != Some("query_response") {
        return Ok(());
    }
    let query = value
        .get("query")
        .ok_or_else(|| "query response missing query".to_owned())?;
    let result = value
        .get("result")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "query response missing result".to_owned())?;
    if result.get("read_only") != Some(&serde_json::Value::Bool(true)) {
        return Err("query result is not read-only".to_owned());
    }
    let page = value
        .get("result")
        .and_then(|result| result.get("page"))
        .ok_or_else(|| "query response missing result.page".to_owned())?;
    if page.get("limits") != query.get("limits") {
        return Err("response limits do not echo query limits".to_owned());
    }
    validate_generation(result, query)?;
    let items = page
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "result.page.items is not an array".to_owned())?;
    let accounting = page
        .get("accounting")
        .ok_or_else(|| "result.page.accounting missing".to_owned())?;
    let item_count = accounting
        .get("item_count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "accounting.item_count missing".to_owned())?;
    if item_count != items.len() as u64 {
        return Err("accounting.item_count does not match emitted items".to_owned());
    }
    validate_bounds(page, query, items, accounting)?;
    validate_items(page, query, items)?;
    let final_page = page
        .get("final_page")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| "final_page missing".to_owned())?;
    let has_cursor = !page
        .get("next_cursor")
        .is_some_and(serde_json::Value::is_null);
    if final_page == has_cursor {
        return Err("final_page and next_cursor disagree".to_owned());
    }
    if page
        .get("total_count_known")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && page
            .get("total_count")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|total| total < item_count)
    {
        return Err("total_count is smaller than emitted item count".to_owned());
    }
    if page.get("coverage").and_then(serde_json::Value::as_str) == Some("complete")
        && page
            .get("total_count_known")
            .and_then(serde_json::Value::as_bool)
            != Some(true)
    {
        return Err("complete coverage requires known total_count".to_owned());
    }
    Ok(())
}

fn validate_generation(
    result: &serde_json::Map<String, serde_json::Value>,
    query: &serde_json::Value,
) -> Result<(), String> {
    let mode = query
        .get("binding")
        .and_then(|binding| binding.get("mode"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "query binding mode missing".to_owned())?;
    match mode {
        "static" => {
            if result.get("result_generation") != Some(&serde_json::Value::Null)
                || result.get("parent_observation") != Some(&serde_json::Value::Null)
            {
                return Err("static response carries live generation".to_owned());
            }
        }
        "live" => {
            let generation = query
                .get("binding")
                .and_then(|binding| binding.get("snapshot_ref"))
                .and_then(|snapshot| snapshot.get("state_generation"));
            if result.get("result_generation") != generation
                || result.get("parent_observation") != query.get("parent_observation")
            {
                return Err("live response generation fence mismatch".to_owned());
            }
        }
        _ => return Err("unknown response binding mode".to_owned()),
    }
    Ok(())
}

fn validate_bounds(
    page: &serde_json::Value,
    query: &serde_json::Value,
    items: &[serde_json::Value],
    accounting: &serde_json::Value,
) -> Result<(), String> {
    let limits = query
        .get("limits")
        .ok_or_else(|| "query limits missing".to_owned())?;
    let page_items = limits
        .get("page_items")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "page_items missing".to_owned())?;
    let item_limit = limits
        .get("item_bytes")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "item_bytes missing".to_owned())?;
    let page_limit = limits
        .get("page_bytes")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "page_bytes missing".to_owned())?;
    let text_limit = limits
        .get("text_bytes")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "text_bytes missing".to_owned())?;
    let measured_payload = serde_json::to_vec(items)
        .map_err(|_| "items cannot be serialized".to_owned())?
        .len() as u64;
    let measured_item = items
        .iter()
        .map(|item| serde_json::to_vec(item).map(|bytes| bytes.len() as u64))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "item cannot be serialized".to_owned())?
        .into_iter()
        .max()
        .unwrap_or(0);
    let Some(page_object) = page.as_object() else {
        return Err("page is not an object".to_owned());
    };
    let mut without_accounting = page_object.clone();
    without_accounting.remove("accounting");
    let measured_page = serde_json::to_vec(&without_accounting)
        .map_err(|_| "page cannot be serialized".to_owned())?
        .len() as u64;
    let declared = |name: &str| {
        accounting
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("accounting.{name} missing"))
    };
    if items.len() as u64 > page_items
        || measured_item > item_limit
        || measured_payload > page_limit
        || measured_page > page_limit
        || declared("item_bytes")? != measured_item
        || declared("payload_bytes")? != measured_payload
        || declared("page_bytes")? != measured_page
    {
        return Err(format!(
            "page byte accounting or bounds mismatch (declared item={} payload={} page={}, measured item={} payload={} page={})",
            declared("item_bytes")?,
            declared("payload_bytes")?,
            declared("page_bytes")?,
            measured_item,
            measured_payload,
            measured_page
        ));
    }
    let text_bytes = page_text_bytes(items)?;
    if declared("text_bytes")? != text_bytes || text_bytes > text_limit {
        return Err("text byte accounting or bounds mismatch".to_owned());
    }
    Ok(())
}

fn page_text_bytes(items: &[serde_json::Value]) -> Result<u64, String> {
    let mut total = 0_u64;
    for item in items {
        let fields = item
            .get("fields")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "item fields missing".to_owned())?;
        for field in fields {
            if field
                .get("availability")
                .and_then(serde_json::Value::as_str)
                != Some("available")
            {
                continue;
            }
            match field.get("value") {
                Some(serde_json::Value::String(value)) => total += value.len() as u64,
                Some(serde_json::Value::Array(values)) => {
                    total += values
                        .iter()
                        .filter_map(serde_json::Value::as_str)
                        .map(|value| value.len() as u64)
                        .sum::<u64>();
                }
                _ => {}
            }
        }
    }
    Ok(total)
}

fn validate_items(
    page: &serde_json::Value,
    query: &serde_json::Value,
    items: &[serde_json::Value],
) -> Result<(), String> {
    let mode = query
        .get("binding")
        .and_then(|binding| binding.get("mode"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "binding mode missing".to_owned())?;
    let manifest = query
        .get("binding")
        .and_then(|binding| binding.get("content_manifest_id"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "content manifest missing".to_owned())?;
    let entity_kind = query
        .get("entity_kind")
        .ok_or_else(|| "entity kind missing".to_owned())?;
    for item in items {
        let definition = item
            .get("definition_ref")
            .ok_or_else(|| "definition_ref missing".to_owned())?;
        if definition
            .get("content_manifest_id")
            .and_then(serde_json::Value::as_str)
            != Some(manifest)
            || definition.get("entity_kind") != Some(entity_kind)
        {
            return Err("item identity does not match query binding".to_owned());
        }
        let instance = item
            .get("instance_ref")
            .ok_or_else(|| "instance_ref missing".to_owned())?;
        if (mode == "static" && !instance.is_null()) || (mode == "live" && instance.is_null()) {
            return Err("item instance scope does not match query mode".to_owned());
        }
        for field in item
            .get("fields")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| "item fields missing".to_owned())?
        {
            let available = field
                .get("availability")
                .and_then(serde_json::Value::as_str)
                == Some("available");
            let value_present = field.get("value").is_some_and(|value| !value.is_null());
            let reason_present = field.get("reason").is_some_and(|value| !value.is_null());
            if value_present != available || reason_present == available {
                return Err("field availability/value/reason contradiction".to_owned());
            }
        }
    }
    let _ = page;
    Ok(())
}
