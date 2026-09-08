// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::common::{fail, parse_object};

const SCHEMA_VERSION: &str = "ai-ascension-source-distribution-policy-v1";
const POLICY_ID: &str = "source-distribution-v1";
const ARTIFACT_KIND: &str = "source_bundle";
const ARTIFACT_SCOPE: &str = "production_source_only";
const SELECTION_MODE: &str = "exact_tracked_path_allowlist";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Policy {
    pub(crate) allowed: Vec<String>,
    pub(crate) excluded: Vec<String>,
    pub(crate) required: Vec<String>,
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Policy, String> {
    let object = parse_object(bytes, "source policy")?;
    require_string(&object, "schema_version", SCHEMA_VERSION)?;
    require_string(&object, "policy_id", POLICY_ID)?;
    require_string(&object, "artifact_kind", ARTIFACT_KIND)?;
    require_string(&object, "artifact_scope", ARTIFACT_SCOPE)?;
    require_string(&object, "selection_mode", SELECTION_MODE)?;
    if object
        .get("allow_only_regular_files")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return fail("source policy must require regular files");
    }

    let allowed = paths(&object, "allowed_paths")?;
    let excluded = paths(&object, "excluded_paths")?;
    let required = paths(&object, "required_paths")?;
    let allowed_set: BTreeSet<&str> = allowed.iter().map(String::as_str).collect();
    if allowed
        .iter()
        .any(|path| excluded.binary_search(path).is_ok())
    {
        return fail("source policy allowlist and exclusion list overlap");
    }
    if required
        .iter()
        .any(|path| !allowed_set.contains(path.as_str()))
    {
        return fail("source policy required paths must be allowlisted");
    }
    Ok(Policy {
        allowed,
        excluded,
        required,
    })
}

fn require_string(object: &Map<String, Value>, name: &str, expected: &str) -> Result<(), String> {
    if object.get(name).and_then(Value::as_str) != Some(expected) {
        return fail(format!(
            "source policy field {name} is unsupported or missing"
        ));
    }
    Ok(())
}

fn paths(object: &Map<String, Value>, name: &str) -> Result<Vec<String>, String> {
    let values = object
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("source policy field {name} must be a string array"))?;
    let mut paths = Vec::with_capacity(values.len());
    for value in values {
        let path = value
            .as_str()
            .ok_or_else(|| format!("source policy field {name} must be a string array"))?;
        if unsafe_relative_path(path) {
            return fail(format!(
                "source policy field {name} contains an unsafe relative path"
            ));
        }
        paths.push(path.to_owned());
    }
    if paths.windows(2).any(|pair| pair[0] >= pair[1]) {
        return fail(format!(
            "source policy field {name} must be sorted and unique"
        ));
    }
    Ok(paths)
}

fn unsafe_relative_path(path: &str) -> bool {
    path.is_empty()
        || path.starts_with('/')
        || path.starts_with('\\')
        || windows_absolute(path)
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
}

fn windows_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use serde_json::json;

    use super::parse;

    fn policy(
        allowed: serde_json::Value,
        excluded: serde_json::Value,
        required: serde_json::Value,
    ) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "schema_version": "ai-ascension-source-distribution-policy-v1",
            "policy_id": "source-distribution-v1",
            "artifact_kind": "source_bundle",
            "artifact_scope": "production_source_only",
            "selection_mode": "exact_tracked_path_allowlist",
            "allow_only_regular_files": true,
            "allowed_paths": allowed,
            "excluded_paths": excluded,
            "required_paths": required
        }))
        .expect("policy JSON")
    }

    #[test]
    fn accepts_metadata_and_ignores_unrelated_properties() {
        let mut value: serde_json::Value = serde_json::from_slice(&policy(
            json!(["a.txt"]),
            json!(["x.txt"]),
            json!(["a.txt"]),
        ))
        .expect("policy value");
        value
            .as_object_mut()
            .expect("object")
            .insert("review_note".into(), json!("allowed"));
        let parsed =
            parse(&serde_json::to_vec(&value).expect("policy JSON")).expect("valid policy");
        assert_eq!(parsed.allowed, vec!["a.txt"]);
    }

    #[test]
    fn rejects_duplicate_keys() {
        let bytes = br#"{"schema_version":"ai-ascension-source-distribution-policy-v1","schema_version":"ai-ascension-source-distribution-policy-v1"}"#;
        assert!(parse(bytes).is_err());
    }

    #[test]
    fn rejects_unsafe_and_unsorted_paths() {
        for paths in [
            json!(["../escape"]),
            json!(["C:/escape"]),
            json!(["b", "a"]),
            json!(["a", "a"]),
        ] {
            assert!(parse(&policy(paths, json!(["x"]), json!([]))).is_err());
        }
    }

    #[test]
    fn rejects_overlap_and_required_subset_violation() {
        assert!(parse(&policy(json!(["a"]), json!(["a"]), json!([]))).is_err());
        assert!(parse(&policy(json!(["a"]), json!(["x"]), json!(["x"]))).is_err());
    }
}
