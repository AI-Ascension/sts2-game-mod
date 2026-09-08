// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use super::{BuildManifest, check_identity, git_identity, repository, selected_loader};
use crate::common::{
    LOADER_MANIFEST, MANIFEST_SCHEMA, Record, bounded_size, fail, full_hex, parse_object,
    read_bounded, regular_file_path, safe_token, sha256_bytes, sha256_hex,
};
use crate::receipt::{payload_names, validate_receipt_metadata};

pub(crate) fn validate_build_manifest(
    path: &Path,
    platform: &str,
    package_version: &str,
    game_version: &str,
    source_revision: &str,
    expected_payload: &[&str],
) -> Result<BuildManifest, String> {
    let path = regular_file_path(path, "build manifest")?;
    let bytes = read_bounded(&path, "build manifest", 1_048_576)?;
    let digest = sha256_bytes(&bytes);
    let value = Value::Object(parse_object(&bytes, "build manifest")?);
    let object = value
        .as_object()
        .ok_or_else(|| "build manifest must be a JSON object".to_owned())?;
    let expected: BTreeSet<&str> = BTreeSet::from([
        "schema_version",
        "source_commit",
        "source_tree",
        "game_version",
        "package_version",
        "loader_manifest_version",
        "loader_manifest",
        "receipts",
        "artifacts",
    ]);
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if actual != expected {
        return fail("build manifest has unexpected or missing properties");
    }
    if object.get("schema_version").and_then(Value::as_str) != Some(MANIFEST_SCHEMA) {
        return fail("build manifest schema version is unsupported");
    }
    let source_commit = full_hex(
        object
            .get("source_commit")
            .and_then(Value::as_str)
            .ok_or("build source commit is missing")?,
        "build source commit",
    )?;
    let source_tree = full_hex(
        object
            .get("source_tree")
            .and_then(Value::as_str)
            .ok_or("build source tree is missing")?,
        "build source tree",
    )?;
    if source_revision != source_commit {
        return fail("source revision does not match build manifest source_commit");
    }
    if object.get("game_version").and_then(Value::as_str) != Some(game_version)
        || object.get("package_version").and_then(Value::as_str) != Some(package_version)
    {
        return fail("package/game version does not match build manifest");
    }
    safe_token(game_version, "game version", false, false)?;
    safe_token(package_version, "package version", false, false)?;
    let loader_version = object
        .get("loader_manifest_version")
        .and_then(Value::as_str)
        .ok_or("build loader manifest version is missing")?;
    safe_token(
        loader_version,
        "build loader manifest version",
        false,
        false,
    )?;
    if loader_version != package_version {
        return fail("build loader manifest version does not match package version");
    }
    let loader_record = record_from_value(
        object
            .get("loader_manifest")
            .ok_or("build loader manifest record is missing")?,
        LOADER_MANIFEST,
        "build loader manifest record",
    )?;
    let repo = repository()?;
    let (resolved_commit, resolved_tree) = git_identity(&repo, &source_commit)?;
    if resolved_commit != source_commit || resolved_tree != source_tree {
        return fail("build manifest source identity does not match the current Git checkout");
    }
    let (selected_loader, selected_record) = selected_loader(&repo, &source_commit)?;
    let selected_version = selected_loader
        .get("version")
        .and_then(Value::as_str)
        .ok_or("selected loader manifest version is missing")?;
    if selected_version != loader_version || selected_record != loader_record {
        return fail("build manifest loader metadata does not match the selected Git manifest");
    }
    let receipts = object
        .get("receipts")
        .and_then(Value::as_object)
        .ok_or("build manifest receipts are missing")?;
    if receipts.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != BTreeSet::from(["linux-x86_64", "windows-x86_64"])
    {
        return fail("build manifest must contain both Windows and Linux production receipts");
    }
    let mut validated_receipts = BTreeMap::new();
    for receipt_platform in ["linux-x86_64", "windows-x86_64"] {
        let names = payload_names(receipt_platform)?;
        let receipt = validate_receipt_metadata(
            receipts.get(receipt_platform).ok_or("receipt is missing")?,
            receipt_platform,
            &names,
        )?;
        check_identity(
            &receipt,
            &source_commit,
            &source_tree,
            game_version,
            package_version,
            loader_version,
        )?;
        if receipt
            .payload
            .get(1)
            .map(|record| (record.size_bytes, &record.sha256))
            != Some((selected_record.size_bytes, &selected_record.sha256))
        {
            return fail(format!(
                "{receipt_platform} receipt loader payload is not the selected Git manifest"
            ));
        }
        validated_receipts.insert(receipt_platform.to_owned(), receipt);
    }
    let artifacts = object
        .get("artifacts")
        .and_then(Value::as_object)
        .ok_or("build manifest artifacts are missing")?;
    if artifacts
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>()
        != BTreeSet::from(["linux-x86_64", "windows-x86_64"])
    {
        return fail("build manifest must contain exactly Windows and Linux artifacts");
    }
    if expected_payload != payload_names(platform)?.as_slice() {
        return fail("requested payload inventory does not match the platform");
    }
    let mut validated_artifacts = BTreeMap::new();
    for artifact_platform in ["linux-x86_64", "windows-x86_64"] {
        let names = payload_names(artifact_platform)?;
        let values = artifacts
            .get(artifact_platform)
            .and_then(Value::as_array)
            .ok_or("build manifest artifact inventory is invalid")?;
        if values.len() != names.len() {
            return fail(format!(
                "build manifest artifact inventory is invalid for {artifact_platform}"
            ));
        }
        let records: Vec<Record> = names
            .iter()
            .zip(values)
            .map(|(name, value)| record_from_value(value, name, "artifact inventory"))
            .collect::<Result<_, _>>()?;
        if Some(&records)
            != validated_receipts
                .get(artifact_platform)
                .map(|receipt| &receipt.payload)
        {
            return fail(format!(
                "{artifact_platform} artifacts do not match its receipt payload"
            ));
        }
        validated_artifacts.insert(artifact_platform.to_owned(), records);
    }
    Ok(BuildManifest {
        digest,
        artifacts: validated_artifacts,
    })
}

fn record_from_value(value: &Value, expected: &str, label: &str) -> Result<Record, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{label} is not an object"))?;
    if object.keys().map(String::as_str).collect::<BTreeSet<_>>()
        != BTreeSet::from(["path", "size_bytes", "sha256"])
    {
        return fail(format!("{label} has unexpected or missing properties"));
    }
    if object.get("path").and_then(Value::as_str) != Some(expected) {
        return fail(format!("{label} is not in the required payload order"));
    }
    let size = bounded_size(
        object.get("size_bytes").ok_or("record size is missing")?,
        &format!("{label} size"),
    )?;
    let sha256 = sha256_hex(
        object
            .get("sha256")
            .and_then(Value::as_str)
            .ok_or("record digest is missing")?,
        &format!("{label} digest"),
    )?;
    Ok(Record {
        path: expected.into(),
        size_bytes: size,
        sha256,
    })
}
