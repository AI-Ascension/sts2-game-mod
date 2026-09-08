// SPDX-License-Identifier: MIT

mod args;
mod stage;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{
    ENTRYPOINT, MANAGED_NAME, Record, absolute, digest, ensure_absent, fail, regular_dir_path,
    regular_file_path, safe_token,
};
use crate::manifest::validate_build_manifest;
use crate::receipt::payload_names;
use args::{Args, bounded_decimal, parse_args};

const PACKAGE_SCHEMA: &str = "sts2-workshop-manifest-v1";
const PACKAGE_ID: &str = "ai-ascension.sts2-game-mod";
const LOADER_CONTRACT: &str = "sts2-managed-loader-v1";
const MAX_APP_ID: u64 = u32::MAX as u64;
const MAX_ITEM_ID: u64 = u64::MAX;

pub fn run(arguments: &[String]) -> Result<(), String> {
    let args = parse_args(arguments)?;
    let payload_names = payload_names(&args.platform)?;
    let app_id = bounded_decimal(&args.app_id, MAX_APP_ID, "consumer app ID")?;
    let item_id = bounded_decimal(&args.item_id, MAX_ITEM_ID, "published file ID")?;
    if app_id == 0 {
        return fail("consumer app ID must be a positive uint32 decimal");
    }
    safe_token(&args.game_version, "game version", false, false)?;
    safe_token(&args.package_version, "package version", false, false)?;
    let source_revision = if args.build_manifest.is_some() {
        let source = args.source_revision.to_ascii_lowercase();
        crate::common::full_hex(&source, "source revision")?
    } else {
        safe_token(&args.source_revision, "source revision", true, false)?;
        if !args.legacy {
            return fail(
                "a build manifest is required; use --legacy-unbound only for synthetic fixtures",
            );
        }
        args.source_revision.clone()
    };
    let payload = regular_dir_path(
        &crate::common::absolute(&args.payload_dir, "payload directory")?,
        "payload directory",
    )?;
    let output = absolute(&args.output_dir, "output directory")?;
    let preview = regular_file_path(
        &absolute(&args.preview_file, "preview file")?,
        "preview file",
    )?;
    if output == payload || output.starts_with(&payload) {
        return fail("output directory must not be inside the payload directory");
    }
    if payload.starts_with(&output) {
        return fail("payload directory must not be inside the output directory");
    }
    ensure_absent(&output, "output directory")?;
    let vdf_path = PathBuf::from(format!("{}.vdf", output.display()));
    ensure_absent(&vdf_path, "VDF output")?;
    let entries = payload_entries(&payload, &payload_names)?;
    let (manifest_digest, expected_records) = if let Some(path) = &args.build_manifest {
        let path = regular_file_path(&absolute(path, "build manifest")?, "build manifest")?;
        let validated = validate_build_manifest(
            &path,
            &args.platform,
            &args.package_version,
            &args.game_version,
            &source_revision,
            &payload_names,
        )?;
        let records = validated
            .artifacts
            .get(&args.platform)
            .cloned()
            .ok_or("build manifest platform artifacts are missing")?;
        verify_payload(&entries, &records)?;
        (Some(validated.digest), records)
    } else {
        let records = payload_names
            .iter()
            .map(|name| {
                let (size_bytes, sha256) = digest(
                    entries.get(*name).ok_or("payload entry disappeared")?,
                    "payload",
                )?;
                Ok(Record {
                    path: (*name).into(),
                    size_bytes,
                    sha256,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        (None, records)
    };
    stage::stage_package(stage::StageInputs {
        args: &args,
        source_revision: &source_revision,
        app_id,
        item_id,
        output: &output,
        vdf_path: &vdf_path,
        preview: &preview,
        entries: &entries,
        expected_records: &expected_records,
        manifest_digest,
    })
}

fn payload_entries(payload: &Path, names: &[&str]) -> Result<BTreeMap<String, PathBuf>, String> {
    let mut entries = BTreeMap::new();
    for entry in fs::read_dir(payload)
        .map_err(|error| format!("cannot inspect payload directory: {error}"))?
    {
        let entry = entry.map_err(|error| format!("cannot inspect payload directory: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if !names.contains(&name.as_str()) {
            return fail(format!("unexpected payload entry: {name}"));
        }
        let path = regular_file_path(&entry.path(), "payload entry")?;
        if entries.insert(name.clone(), path).is_some() {
            return fail(format!("duplicate payload entry: {name}"));
        }
    }
    for name in names {
        if !entries.contains_key(*name) {
            return fail(format!("required payload file is missing: {name}"));
        }
        digest(
            entries.get(*name).ok_or("payload entry disappeared")?,
            "payload",
        )?;
    }
    Ok(entries)
}

fn verify_payload(entries: &BTreeMap<String, PathBuf>, records: &[Record]) -> Result<(), String> {
    for record in records {
        let path = entries
            .get(&record.path)
            .ok_or("build manifest payload file is missing")?;
        let actual = digest(path, "payload")?;
        if actual != (record.size_bytes, record.sha256.clone()) {
            return fail(format!(
                "payload does not match build manifest: {}",
                record.path
            ));
        }
    }
    Ok(())
}
