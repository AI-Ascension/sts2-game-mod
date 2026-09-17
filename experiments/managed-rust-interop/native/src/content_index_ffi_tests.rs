// SPDX-License-Identifier: MIT

use super::abi::run;
use super::{InputSnapshot, Output, STATUS_OK, build_index};

fn must<T, E: std::fmt::Debug>(value: Result<T, E>, label: &str) -> T {
    match value {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{label}: {error:?}");
            std::process::abort();
        }
    }
}

fn must_some<T>(value: Option<T>, label: &str) -> T {
    match value {
        Some(value) => value,
        None => {
            eprintln!("{label}");
            std::process::abort();
        }
    }
}

fn snapshot() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
            "generation_before": 7,
            "generation_after": 7,
            "game_build": "build:probe",
            "locale": "en-US",
            "packages": [
                {"package_id": "base", "package_version": "1", "order": 0}
            ],
            "available_entity_kinds": ["card", "relic"],
            "registry_definition_counts": {"card": 5, "relic": 0},
            "definitions": [
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:bash",
                    "semantic_inputs": "{\"rarity\":\"attack\",\"unlock_state\":\"unlocked\"}",
                    "localized_text": "{\"title\":\"Bash\",\"description\":\"Deal damage\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:defend",
                    "semantic_inputs": "{\"unlock_state\":\"unlocked\"}",
                    "localized_text": "{\"title\":\"Defend\",\"aliases\":[\"Guard\"]}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:strike",
                    "semantic_inputs": "{\"rarity\":\"attack\",\"unlock_state\":\"unlocked\"}",
                    "localized_text": "{\"title\":\"Strike\",\"aliases\":[\"Hit\"],\"description\":\"Deal damage\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:hidden",
                    "semantic_inputs": "{\"unlock_state\":\"unknown\"}",
                    "localized_text": "{\"title\":\"Hidden\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                },
                {
                    "entity_kind": "card", "namespaced_id": "ironclad:locked",
                    "semantic_inputs": "{\"unlock_state\":\"locked\"}",
                    "localized_text": "{\"title\":\"Locked\"}",
                    "origin": {"package_id":"base","package_version":"1"},
                    "override_chain": []
                }
            ],
            "adapter_compatibility": "content-index-v1"
        }))
        .unwrap_or_else(|error| {
            eprintln!("fixture serializes: {error}");
            std::process::abort();
        })
}

fn query(operation: &str, limit: usize, cursor: Option<&str>) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "operation": operation,
        "literal": null,
        "entity_kind": "card",
        "namespaced_id": null,
        "limit": limit,
        "cursor": cursor,
        "binding_key": "binding:fixture"
    }))
    .unwrap_or_else(|error| {
        eprintln!("query serializes: {error}");
        std::process::abort();
    })
}

#[test]
fn reader_controls_visibility_order_and_single_use_cursor() {
    let fixture: InputSnapshot = must(serde_json::from_slice(&snapshot()), "snapshot");
    assert!(build_index(fixture).is_ok(), "index build failed");
    let (status, first_bytes) = run(&snapshot(), &query("list", 1, None));
    assert_eq!(status, STATUS_OK);
    let first: Output = must(serde_json::from_slice(&first_bytes), "first output");
    assert_eq!(first.total_count, 3);
    assert!(!first.final_page);
    assert_eq!(first.items[0].namespaced_id, "ironclad:bash");
    let cursor = must_some(first.next_cursor, "continuation");

    let (status, second_bytes) = run(&[], &query("list", 1, Some(&cursor)));
    assert_eq!(status, STATUS_OK);
    let second: Output = must(serde_json::from_slice(&second_bytes), "second output");
    assert!(!second.final_page);
    assert_eq!(second.items[0].namespaced_id, "ironclad:defend");
    let third_cursor = must_some(second.next_cursor, "third continuation");

    let (status, third_bytes) = run(&[], &query("list", 1, Some(&third_cursor)));
    assert_eq!(status, STATUS_OK);
    let third: Output = must(serde_json::from_slice(&third_bytes), "third output");
    assert!(third.final_page);
    assert_eq!(third.items[0].namespaced_id, "ironclad:strike");

    let (status, _) = run(&[], &query("list", 1, Some(&cursor)));
    assert_eq!(status, 409);
}

#[test]
fn reader_uses_literal_folded_search_and_unknown_is_excluded() {
    let (status, bytes) = run(
        &snapshot(),
        &serde_json::to_vec(&serde_json::json!({
            "operation": "search",
            "literal": "guard",
            "entity_kind": "card",
            "namespaced_id": null,
            "limit": 8,
            "cursor": null,
            "binding_key": "binding:search"
        }))
        .unwrap_or_else(|error| {
            eprintln!("query serializes: {error}");
            std::process::abort();
        }),
    );
    assert_eq!(status, STATUS_OK);
    let output: Output = must(serde_json::from_slice(&bytes), "search output");
    assert_eq!(output.total_count, 1);
    assert_eq!(output.items[0].namespaced_id, "ironclad:defend");
}

#[test]
fn exact_unknown_returns_not_found_status() {
    let unknown = serde_json::to_vec(&serde_json::json!({
        "operation": "get", "literal": null, "entity_kind": "card",
        "namespaced_id": "ironclad:missing", "limit": 1, "cursor": null,
        "binding_key": "binding:unknown"
    }))
    .unwrap_or_else(|error| {
        eprintln!("query serializes: {error}");
        std::process::abort();
    });
    let (status, _) = run(&snapshot(), &unknown);
    assert_eq!(status, 404);
}

#[test]
fn identity_filters_refuse_before_pagination() {
    let filtered = serde_json::to_vec(&serde_json::json!({
        "operation": "list", "literal": null, "entity_kind": "card",
        "namespaced_id": null, "namespaced_ids": ["ironclad:locked"],
        "definition_refs": [], "limit": 1, "cursor": null,
        "binding_key": "binding:filter"
    }))
    .unwrap_or_else(|error| {
        eprintln!("query serializes: {error}");
        std::process::abort();
    });
    let (status, body) = run(&snapshot(), &filtered);
    assert_eq!(status, 400);
    let error_code = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("error_code")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    assert_eq!(error_code.as_deref(), Some("unsupported_filter"));
}

#[test]
fn locked_definition_has_typed_scope_behavior() {
    let public = must(
        serde_json::to_vec(&serde_json::json!({
            "operation": "get", "literal": null, "entity_kind": "card",
            "namespaced_id": "ironclad:locked", "limit": 1, "cursor": null,
            "scope": "public", "binding_key": "binding:locked"
        })),
        "public query serializes",
    );
    let (status, body) = run(&snapshot(), &public);
    assert_eq!(status, 403);
    let error_code = serde_json::from_slice::<serde_json::Value>(&body)
        .ok()
        .and_then(|value| {
            value
                .get("error_code")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        });
    assert_eq!(error_code.as_deref(), Some("denied_scope"));
    let reference =
        must(String::from_utf8(public), "query utf8").replace("\"public\"", "\"reference\"");
    let (status, body) = run(&snapshot(), reference.as_bytes());
    assert_eq!(status, STATUS_OK);
    let output: Output = must(serde_json::from_slice(&body), "reference output");
    assert_eq!(output.items[0].namespaced_id, "ironclad:locked");
}
