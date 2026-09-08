// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use super::{Args, ENTRYPOINT, LOADER_CONTRACT, MANAGED_NAME, PACKAGE_ID, PACKAGE_SCHEMA};
use crate::common::{
    Record, absolute, claim, copy_checked, digest, ensure_absent, fail, move_no_replace,
    owned_path, record_value, release_claim, remove_owned_dir, remove_owned_file, sha256_bytes,
    temp_suffix, write_new,
};

pub(super) struct StageInputs<'a> {
    pub(super) args: &'a Args,
    pub(super) source_revision: &'a str,
    pub(super) app_id: u64,
    pub(super) item_id: u64,
    pub(super) output: &'a Path,
    pub(super) vdf_path: &'a Path,
    pub(super) preview: &'a Path,
    pub(super) entries: &'a BTreeMap<String, std::path::PathBuf>,
    pub(super) expected_records: &'a [Record],
    pub(super) manifest_digest: Option<String>,
}

pub(super) fn stage_package(input: StageInputs<'_>) -> Result<(), String> {
    let StageInputs {
        args,
        source_revision,
        app_id,
        item_id,
        output,
        vdf_path,
        preview,
        entries,
        expected_records,
        manifest_digest,
    } = input;
    let parent = output.parent().ok_or("output parent is missing")?;
    fs::create_dir_all(parent).map_err(|error| format!("cannot create output parent: {error}"))?;
    crate::common::reject_symlink_components(parent, "output parent")?;
    let claim_path = std::path::PathBuf::from(format!("{}.claim", output.display()));
    let claim = claim(&claim_path)?;
    let temporary = parent.join(format!(
        ".{}.tmp.{}",
        output.file_name().unwrap_or_default().to_string_lossy(),
        temp_suffix()
    ));
    let vdf_temporary = parent.join(format!(
        ".{}.tmp.{}",
        vdf_path.file_name().unwrap_or_default().to_string_lossy(),
        temp_suffix()
    ));
    let mut temporary_owner: Option<crate::common::OwnedPath> = None;
    let mut vdf_owner: Option<crate::common::OwnedPath> = None;
    let mut committed_package = false;
    let mut committed_vdf = false;
    let result = (|| {
        fs::create_dir(&temporary)
            .map_err(|error| format!("cannot create package staging directory: {error}"))?;
        temporary_owner = Some(owned_path(&temporary, "package staging directory")?);
        let staged = copy_payload(entries, expected_records, &temporary)?;
        let mut package = Map::new();
        package.insert("schema_version".into(), PACKAGE_SCHEMA.into());
        package.insert("package_id".into(), PACKAGE_ID.into());
        package.insert(
            "package_version".into(),
            args.package_version.clone().into(),
        );
        package.insert("consumer_app_id".into(), app_id.into());
        package.insert("published_file_id".into(), item_id.into());
        package.insert("game_version".into(), args.game_version.clone().into());
        package.insert("platform".into(), args.platform.clone().into());
        package.insert("loader_contract".into(), LOADER_CONTRACT.into());
        package.insert("content_kind".into(), "first_party_executable".into());
        package.insert("entrypoint".into(), ENTRYPOINT.into());
        package.insert("files".into(), Value::Array(staged));
        package.insert(
            "content_digest".into(),
            content_digest(expected_records).into(),
        );
        package.insert("source_revision".into(), source_revision.into());
        let mut package_bytes = serde_json::to_vec_pretty(&Value::Object(package))
            .map_err(|error| error.to_string())?;
        package_bytes.push(b'\n');
        write_new(
            &temporary.join("sts2-workshop-manifest.json"),
            &package_bytes,
            "Workshop manifest",
            0o644,
        )?;
        write_checksums(&temporary, expected_records)?;
        let vdf = format_vdf(args, app_id, item_id, output, preview, source_revision);
        write_new(&vdf_temporary, vdf.as_bytes(), "Workshop VDF", 0o644)?;
        vdf_owner = Some(owned_path(&vdf_temporary, "Workshop VDF staging file")?);
        if std::env::var("STS2_RELEASE_TEST_INTERRUPT_AFTER_COPY").as_deref() == Ok("1") {
            return fail("interrupted after package copy");
        }
        verify_stable(entries, expected_records)?;
        verify_manifest_stable(manifest_digest, args)?;
        ensure_absent(output, "output directory")?;
        ensure_absent(vdf_path, "VDF output")?;
        move_no_replace(&temporary, output, "output directory")?;
        committed_package = true;
        move_no_replace(&vdf_temporary, vdf_path, "VDF output")?;
        committed_vdf = true;
        println!("Workshop package staged at {}", output.display());
        println!(
            "SteamCMD/ISteamUGC configuration staged at {}",
            vdf_path.display()
        );
        Ok(())
    })();
    if result.is_err() {
        if let Some(owner) = temporary_owner.as_ref() {
            remove_owned_dir(owner);
        }
        if let Some(owner) = vdf_owner.as_ref() {
            remove_owned_file(owner);
        }
    }
    if !committed_package || committed_vdf {
        release_claim(&claim);
    }
    if committed_package && !committed_vdf {
        result.map_err(|error| {
            format!(
                "package directory was published but VDF publication failed; claim retained: {error}"
            )
        })
    } else {
        result
    }
}

fn copy_payload(
    entries: &BTreeMap<String, std::path::PathBuf>,
    expected_records: &[Record],
    temporary: &Path,
) -> Result<Vec<Value>, String> {
    expected_records
        .iter()
        .map(|record| {
            copy_checked(
                entries
                    .get(&record.path)
                    .ok_or("payload entry disappeared")?,
                &temporary.join(&record.path),
                record.size_bytes,
                &record.sha256,
            )?;
            let mut value = record_value(record);
            if let Value::Object(object) = &mut value {
                let role = match record.path.as_str() {
                    MANAGED_NAME => "managed_assembly",
                    ENTRYPOINT => "loader_manifest",
                    _ => "native_library",
                };
                object.insert("role".into(), role.into());
            }
            Ok(value)
        })
        .collect()
}

fn content_digest(expected_records: &[Record]) -> String {
    sha256_bytes(
        expected_records
            .iter()
            .map(|record| {
                format!(
                    "{}\t{}\t{}\n",
                    record.path, record.size_bytes, record.sha256
                )
            })
            .collect::<String>()
            .as_bytes(),
    )
}

fn write_checksums(temporary: &Path, expected_records: &[Record]) -> Result<(), String> {
    let mut checksum = String::new();
    for name in expected_records
        .iter()
        .map(|record| record.path.as_str())
        .chain(["sts2-workshop-manifest.json"])
    {
        let (_, hash) = digest(&temporary.join(name), "package entry")?;
        checksum.push_str(&format!("{hash}  {name}\n"));
    }
    write_new(
        &temporary.join("SHA256SUMS"),
        checksum.as_bytes(),
        "package checksums",
        0o644,
    )
}

fn format_vdf(
    args: &Args,
    app_id: u64,
    item_id: u64,
    output: &Path,
    preview: &Path,
    source_revision: &str,
) -> String {
    format!(
        "\"workshopitem\"\n{{\n  \"appid\" \"{app_id}\"\n  \"publishedfileid\" \"{item_id}\"\n  \"contentfolder\" \"{}\"\n  \"previewfile\" \"{}\"\n  \"visibility\" \"0\"\n  \"title\" \"AI-Ascension STS2 Game Mod\"\n  \"description\" \"First-party AI-Ascension STS2 game-process adapter package.\"\n  \"changenote\" \"Package {} from {}\"\n}}\n",
        vdf_escape(&output.display().to_string()),
        vdf_escape(&preview.display().to_string()),
        vdf_escape(&args.package_version),
        vdf_escape(source_revision),
    )
}

fn verify_stable(
    entries: &BTreeMap<String, std::path::PathBuf>,
    expected_records: &[Record],
) -> Result<(), String> {
    for record in expected_records {
        let actual = digest(
            entries
                .get(&record.path)
                .ok_or("payload entry disappeared")?,
            "payload",
        )?;
        if actual != (record.size_bytes, record.sha256.clone()) {
            return fail(format!(
                "payload changed while it was being staged: {}",
                record.path
            ));
        }
    }
    Ok(())
}

fn verify_manifest_stable(manifest_digest: Option<String>, args: &Args) -> Result<(), String> {
    if let (Some(expected), Some(path)) = (manifest_digest, args.build_manifest.as_ref()) {
        let actual = sha256_bytes(&crate::common::read_bounded(
            &absolute(path, "build manifest")?,
            "build manifest",
            1_048_576,
        )?);
        if actual != expected {
            return fail("build manifest changed while the package was being staged");
        }
    }
    Ok(())
}

fn vdf_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
