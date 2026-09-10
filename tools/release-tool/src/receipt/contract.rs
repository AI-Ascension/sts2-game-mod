// SPDX-License-Identifier: MIT

mod validation;

pub use validation::{metadata_value, stable, validate_receipt_metadata, validate_receipt_root};

use std::collections::BTreeSet;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::common::{
    ENTRYPOINT, LINUX_NATIVE, MANAGED_NAME, Record, WINDOWS_NATIVE, bounded_size, safe_text,
    safe_token, sha256_hex,
};

const RECEIPT_KEYS: &[&str] = &[
    "schema_version",
    "scope",
    "platform",
    "source_commit",
    "source_tree",
    "game_version",
    "package_version",
    "loader_manifest_version",
    "toolchain",
    "host_references",
    "fixture_flags",
    "payload",
];
const RECEIPT_METADATA_KEYS: &[&str] = &[
    "schema_version",
    "scope",
    "platform",
    "source_commit",
    "source_tree",
    "game_version",
    "package_version",
    "loader_manifest_version",
    "toolchain",
    "host_references",
    "fixture_flags",
    "payload",
    "receipt_size_bytes",
    "receipt_sha256",
];
const TOOLCHAIN_KEYS: &[&str] = &[
    "dotnet_executable",
    "dotnet_path_style",
    "dotnet_version",
    "rustc_version",
    "cargo_version",
];
const HOST_REFERENCE_NAMES: &[&str] = &["sts2.dll", "GodotSharp.dll"];

#[derive(Clone, Debug)]
pub struct ReceiptMetadata {
    pub schema_version: String,
    pub scope: String,
    pub platform: String,
    pub source_commit: String,
    pub source_tree: String,
    pub game_version: String,
    pub package_version: String,
    pub loader_manifest_version: String,
    pub toolchain: Map<String, Value>,
    pub host_references: Vec<Value>,
    pub fixture_flags: Vec<Value>,
    pub payload: Vec<Record>,
    pub receipt_size_bytes: u64,
    pub receipt_sha256: String,
}

#[derive(Clone, Debug)]
pub struct ReceiptRoot {
    pub root: PathBuf,
    pub receipt_path: PathBuf,
    pub payload: PathBuf,
    pub receipt_bytes: Vec<u8>,
    pub metadata: ReceiptMetadata,
}

pub fn native_name(platform: &str) -> Option<&'static str> {
    match platform {
        "windows-x86_64" => Some(WINDOWS_NATIVE),
        "linux-x86_64" => Some(LINUX_NATIVE),
        _ => None,
    }
}

pub fn payload_names(platform: &str) -> Result<Vec<&'static str>, String> {
    let native =
        native_name(platform).ok_or_else(|| format!("unsupported platform: {platform}"))?;
    Ok(vec![MANAGED_NAME, ENTRYPOINT, native])
}

fn exact_keys(object: &Map<String, Value>, expected: &[&str], label: &str) -> Result<(), String> {
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let required: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != required {
        return Err(format!("{label} has unexpected or missing properties"));
    }
    Ok(())
}

fn string(object: &Map<String, Value>, key: &str, label: &str) -> Result<String, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{label} is missing or not a string"))
}

fn record(value: &Value, expected: &str, label: &str) -> Result<Record, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} is not an object"))?;
    exact_keys(object, &["path", "size_bytes", "sha256"], label)?;
    let path = string(object, "path", label)?;
    if path != expected {
        return Err(format!("{label} is not in the required payload order"));
    }
    let size = bounded_size(
        object
            .get("size_bytes")
            .ok_or_else(|| format!("{label} size is missing"))?,
        &format!("{label} size"),
    )?;
    let sha256 = sha256_hex(
        &string(object, "sha256", label)?,
        &format!("{label} digest"),
    )?;
    Ok(Record {
        path,
        size_bytes: size,
        sha256,
    })
}

fn records(value: &Value, names: &[&str], label: &str) -> Result<Vec<Record>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{label} is not an array"))?;
    if values.len() != names.len() {
        return Err(format!("{label} is incomplete"));
    }
    names
        .iter()
        .zip(values)
        .map(|(name, value)| record(value, name, label))
        .collect()
}

fn toolchain(value: &Value, platform: &str) -> Result<Map<String, Value>, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{platform} receipt toolchain is incomplete"))?;
    exact_keys(
        object,
        TOOLCHAIN_KEYS,
        &format!("{platform} receipt toolchain"),
    )?;
    let executable = string(object, "dotnet_executable", platform)?;
    safe_token(
        &executable,
        &format!("{platform} dotnet executable"),
        false,
        false,
    )?;
    if executable.contains('/') || executable.contains('\\') {
        return Err(format!("{platform} dotnet executable is path-like"));
    }
    let style = string(object, "dotnet_path_style", platform)?;
    if !matches!(style.as_str(), "posix" | "windows") {
        return Err(format!(
            "{platform} receipt dotnet path style is unsupported"
        ));
    }
    for key in ["dotnet_version", "rustc_version", "cargo_version"] {
        let value = string(object, key, platform)?;
        safe_text(&value, &format!("{platform} {key}"))?;
        if value.eq_ignore_ascii_case("fixture") {
            return Err(format!(
                "{platform} receipt toolchain cannot use fixture identities"
            ));
        }
    }
    Ok(object.clone())
}

fn host_references(value: &Value, platform: &str) -> Result<Vec<Value>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{platform} receipt host references are incomplete"))?;
    if values.len() != HOST_REFERENCE_NAMES.len() {
        return Err(format!("{platform} receipt host references are incomplete"));
    }
    let mut output = Vec::new();
    for (expected, value) in HOST_REFERENCE_NAMES.iter().zip(values) {
        let object = value
            .as_object()
            .ok_or_else(|| format!("{platform} receipt host reference is not an object"))?;
        exact_keys(
            object,
            &["name", "size_bytes", "sha256"],
            &format!("{platform} host reference"),
        )?;
        if string(object, "name", platform)? != *expected {
            return Err(format!(
                "{platform} receipt host references are not in the required order"
            ));
        }
        bounded_size(
            object
                .get("size_bytes")
                .ok_or_else(|| "host size is missing".to_owned())?,
            &format!("{platform} host reference size"),
        )?;
        sha256_hex(
            &string(object, "sha256", platform)?,
            &format!("{platform} host reference digest"),
        )?;
        output.push(value.clone());
    }
    Ok(output)
}
