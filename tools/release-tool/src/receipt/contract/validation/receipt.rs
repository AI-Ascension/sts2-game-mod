// SPDX-License-Identifier: MIT

use serde_json::{Map, Value};

use super::super::{
    RECEIPT_KEYS, ReceiptMetadata, exact_keys, host_references, records, string, toolchain,
};
use crate::common::{RECEIPT_SCHEMA, full_hex, safe_token, sha256_bytes};

pub(super) fn metadata_from_receipt(
    object: &Map<String, Value>,
    platform: &str,
    payload: &[&str],
    bytes: &[u8],
) -> Result<ReceiptMetadata, String> {
    exact_keys(object, RECEIPT_KEYS, &format!("{platform} build receipt"))?;
    if string(object, "schema_version", platform)? != RECEIPT_SCHEMA
        || string(object, "scope", platform)? != "production"
    {
        return Err(format!(
            "{platform} build receipt must have production scope"
        ));
    }
    if string(object, "platform", platform)? != platform {
        return Err(format!("{platform} build receipt platform does not match"));
    }
    let source_commit = full_hex(
        &string(object, "source_commit", platform)?,
        &format!("{platform} receipt source commit"),
    )?;
    let source_tree = full_hex(
        &string(object, "source_tree", platform)?,
        &format!("{platform} receipt source tree"),
    )?;
    let game_version = string(object, "game_version", platform)?;
    let package_version = string(object, "package_version", platform)?;
    let loader_manifest_version = string(object, "loader_manifest_version", platform)?;
    safe_token(
        &game_version,
        &format!("{platform} receipt game version"),
        false,
        false,
    )?;
    safe_token(
        &package_version,
        &format!("{platform} receipt package version"),
        false,
        false,
    )?;
    safe_token(
        &loader_manifest_version,
        &format!("{platform} receipt loader version"),
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
        payload: records(
            object
                .get("payload")
                .ok_or_else(|| "payload is missing".to_owned())?,
            payload,
            &format!("{platform} receipt payload"),
        )?,
        receipt_size_bytes: bytes.len() as u64,
        receipt_sha256: sha256_bytes(bytes),
    })
}
