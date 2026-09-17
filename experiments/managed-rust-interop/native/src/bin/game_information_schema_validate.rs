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
    let page = value
        .get("result")
        .and_then(|result| result.get("page"))
        .ok_or_else(|| "query response missing result.page".to_owned())?;
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
