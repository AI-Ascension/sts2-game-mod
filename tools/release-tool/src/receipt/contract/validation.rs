// SPDX-License-Identifier: MIT

mod receipt;

use std::collections::BTreeSet;
use std::fs;

use serde_json::{Map, Value};

use super::{
    RECEIPT_METADATA_KEYS, ReceiptMetadata, ReceiptRoot, exact_keys, host_references, records,
    string, toolchain,
};
use crate::common::{
    RECEIPT_SCHEMA, bounded_size, digest, full_hex, parse_object, read_bounded, record_value,
    regular_dir, regular_dir_path, regular_file_path, safe_token, sha256_hex,
};
use receipt::metadata_from_receipt;

pub fn validate_receipt_metadata(
    value: &Value,
    platform: &str,
    expected_payload: &[&str],
) -> Result<ReceiptMetadata, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{platform} receipt metadata is not an object"))?;
    exact_keys(
        object,
        RECEIPT_METADATA_KEYS,
        &format!("{platform} receipt metadata"),
    )?;
    if string(object, "schema_version", platform)? != RECEIPT_SCHEMA
        || string(object, "scope", platform)? != "production"
    {
        return Err(format!(
            "{platform} receipt metadata is not a production receipt"
        ));
    }
    if string(object, "platform", platform)? != platform {
        return Err(format!(
            "{platform} receipt metadata platform does not match"
        ));
    }
    let source_commit = full_hex(
        &string(object, "source_commit", platform)?,
        &format!("{platform} source commit"),
    )?;
    let source_tree = full_hex(
        &string(object, "source_tree", platform)?,
        &format!("{platform} source tree"),
    )?;
    let game_version = string(object, "game_version", platform)?;
    let package_version = string(object, "package_version", platform)?;
    safe_token(
        &game_version,
        &format!("{platform} game version"),
        false,
        false,
    )?;
    safe_token(
        &package_version,
        &format!("{platform} package version"),
        false,
        false,
    )?;
    let loader_manifest_version = string(object, "loader_manifest_version", platform)?;
    safe_token(
        &loader_manifest_version,
        &format!("{platform} loader version"),
        false,
        false,
    )?;
    if loader_manifest_version != package_version {
        return Err(format!(
            "{platform} receipt loader manifest version does not match package version"
        ));
    }
    let toolchain = toolchain(
        object
            .get("toolchain")
            .ok_or_else(|| "toolchain is missing".to_owned())?,
        platform,
    )?;
    let host_references = host_references(
        object
            .get("host_references")
            .ok_or_else(|| "host references are missing".to_owned())?,
        platform,
    )?;
    let fixture_flags = object
        .get("fixture_flags")
        .and_then(Value::as_array)
        .cloned()
        .ok_or_else(|| format!("{platform} fixture flags are invalid"))?;
    if !fixture_flags.is_empty() {
        return Err(format!(
            "{platform} production receipt contains fixture flags"
        ));
    }
    let payload = records(
        object
            .get("payload")
            .ok_or_else(|| "payload is missing".to_owned())?,
        expected_payload,
        &format!("{platform} receipt payload"),
    )?;
    let receipt_size_bytes = bounded_size(
        object
            .get("receipt_size_bytes")
            .ok_or_else(|| "receipt size is missing".to_owned())?,
        &format!("{platform} receipt size"),
    )?;
    let receipt_sha256 = sha256_hex(
        &string(object, "receipt_sha256", platform)?,
        &format!("{platform} receipt digest"),
    )?;
    Ok(ReceiptMetadata {
        schema_version: RECEIPT_SCHEMA.into(),
        scope: "production".into(),
        platform: platform.into(),
        source_commit,
        source_tree,
        game_version,
        package_version,
        loader_manifest_version,
        toolchain,
        host_references,
        fixture_flags,
        payload,
        receipt_size_bytes,
        receipt_sha256,
    })
}

pub fn validate_receipt_root(value: &str, platform: &str) -> Result<ReceiptRoot, String> {
    let root = regular_dir(value, &format!("{platform} receipt root"))?;
    let names: BTreeSet<String> = fs::read_dir(&root)
        .map_err(|error| format!("cannot inspect {platform} receipt root: {error}"))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .map_err(|error| format!("cannot inspect {platform} receipt root: {error}"))?;
    if names != BTreeSet::from(["build-receipt.json".into(), "payload".into()]) {
        return Err(format!(
            "{platform} receipt root must contain only build-receipt.json and payload"
        ));
    }
    let receipt_path = regular_file_path(
        &root.join("build-receipt.json"),
        &format!("{platform} build receipt"),
    )?;
    let payload = regular_dir_path(
        &root.join("payload"),
        &format!("{platform} receipt payload"),
    )?;
    let bytes = read_bounded(
        &receipt_path,
        &format!("{platform} build receipt"),
        1_048_576,
    )?;
    let object = parse_object(&bytes, &format!("{platform} build receipt"))?;
    let names = super::payload_names(platform)?;
    let entries: BTreeSet<String> = fs::read_dir(&payload)
        .map_err(|error| format!("cannot inspect {platform} payload: {error}"))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .map_err(|error| format!("cannot inspect {platform} payload: {error}"))?;
    if entries != names.iter().map(|name| (*name).to_owned()).collect() {
        return Err(format!(
            "{platform} receipt payload must contain exactly the three runtime files"
        ));
    }
    let metadata = metadata_from_receipt(&object, platform, &names, &bytes)?;
    for (name, record) in names.iter().zip(&metadata.payload) {
        let actual = digest(&payload.join(name), &format!("{platform} payload artifact"))?;
        if actual != (record.size_bytes, record.sha256.clone()) {
            return Err(format!(
                "{platform} payload does not match its receipt: {name}"
            ));
        }
    }
    Ok(ReceiptRoot {
        root,
        receipt_path,
        payload,
        receipt_bytes: bytes,
        metadata,
    })
}

pub fn stable(root: &ReceiptRoot) -> Result<(), String> {
    let bytes = read_bounded(&root.receipt_path, "receipt stability check", 1_048_576)?;
    if bytes != root.receipt_bytes {
        return Err(format!(
            "{} build receipt changed while the manifest was being built",
            root.metadata.platform
        ));
    }
    for record in &root.metadata.payload {
        let actual = digest(
            &root.payload.join(&record.path),
            "receipt stability payload",
        )?;
        if actual != (record.size_bytes, record.sha256.clone()) {
            return Err(format!(
                "{} payload changed while the manifest was being built",
                root.metadata.platform
            ));
        }
    }
    Ok(())
}

pub fn metadata_value(metadata: &ReceiptMetadata) -> Value {
    let mut object = Map::new();
    object.insert(
        "schema_version".into(),
        metadata.schema_version.clone().into(),
    );
    object.insert("scope".into(), metadata.scope.clone().into());
    object.insert("platform".into(), metadata.platform.clone().into());
    object.insert(
        "source_commit".into(),
        metadata.source_commit.clone().into(),
    );
    object.insert("source_tree".into(), metadata.source_tree.clone().into());
    object.insert("game_version".into(), metadata.game_version.clone().into());
    object.insert(
        "package_version".into(),
        metadata.package_version.clone().into(),
    );
    object.insert(
        "loader_manifest_version".into(),
        metadata.loader_manifest_version.clone().into(),
    );
    object.insert(
        "toolchain".into(),
        Value::Object(metadata.toolchain.clone()),
    );
    object.insert(
        "host_references".into(),
        Value::Array(metadata.host_references.clone()),
    );
    object.insert(
        "fixture_flags".into(),
        Value::Array(metadata.fixture_flags.clone()),
    );
    object.insert(
        "payload".into(),
        Value::Array(metadata.payload.iter().map(record_value).collect()),
    );
    object.insert(
        "receipt_size_bytes".into(),
        metadata.receipt_size_bytes.into(),
    );
    object.insert(
        "receipt_sha256".into(),
        metadata.receipt_sha256.clone().into(),
    );
    Value::Object(object)
}
