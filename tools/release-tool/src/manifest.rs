// SPDX-License-Identifier: MIT

mod validation;

pub(crate) use validation::validate_build_manifest;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Map, Value};

use crate::common::{
    LOADER_MANIFEST, MANIFEST_SCHEMA, Record, ensure_disjoint, fail, full_hex, move_no_replace,
    parse_object, record_value, regular_dir_path, sha256_bytes, write_new,
};
use crate::receipt::{ReceiptRoot, metadata_value, stable, validate_receipt_root};

#[derive(Clone, Debug)]
pub struct BuildManifest {
    pub digest: String,
    pub artifacts: BTreeMap<String, Vec<Record>>,
}

pub fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 6 {
        return fail(
            "usage: build-artifact-manifest.sh <source-commit> <game-version> <package-version> \
             <windows-receipt-root> <linux-receipt-root> <output-manifest>",
        );
    }
    let source_revision = &args[0];
    let game_version = &args[1];
    let package_version = &args[2];
    crate::common::safe_token(game_version, "game version", false, false)?;
    crate::common::safe_token(package_version, "package version", false, false)?;
    let repo = repository()?;
    check_clean(&repo)?;
    let (source_commit, source_tree) = git_identity(&repo, source_revision)?;
    let output = crate::common::absolute(&args[5], "output manifest")?;
    let windows = validate_receipt_root(&args[3], "windows-x86_64")?;
    let linux = validate_receipt_root(&args[4], "linux-x86_64")?;
    ensure_disjoint(
        &output,
        "output manifest",
        &windows.root,
        "Windows receipt root",
    )?;
    ensure_disjoint(
        &output,
        "output manifest",
        &linux.root,
        "Linux receipt root",
    )?;
    ensure_disjoint(&output, "output manifest", &repo, "Git checkout")?;
    let (loader, loader_record) = selected_loader(&repo, &source_commit)?;
    let loader_version = loader
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| "selected loader manifest version is missing".to_owned())?;
    crate::common::safe_token(
        loader_version,
        "selected loader manifest version",
        false,
        false,
    )?;
    if loader_version != package_version {
        return fail("package version does not match selected source loader manifest version");
    }
    for root in [&windows, &linux] {
        check_identity(
            &root.metadata,
            &source_commit,
            &source_tree,
            game_version,
            package_version,
            loader_version,
        )?;
        if root
            .metadata
            .payload
            .get(1)
            .map(|record| (record.size_bytes, &record.sha256))
            != Some((loader_record.size_bytes, &loader_record.sha256))
        {
            return fail(format!(
                "{} receipt loader manifest is not the selected Git manifest",
                root.metadata.platform
            ));
        }
        stable(root)?;
    }
    let value = build_manifest(
        &source_commit,
        &source_tree,
        game_version,
        package_version,
        loader_version,
        &loader_record,
        [&windows, &linux],
    )?;
    write_manifest(
        &output,
        serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?,
    )?;
    println!("build artifact manifest: {}", output.display());
    println!("source commit: {source_commit}");
    println!("source tree: {source_tree}");
    Ok(())
}

pub(crate) fn repository() -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("STS2_RELEASE_REPO_ROOT") {
        return regular_dir_path(Path::new(&value), "Git checkout");
    }
    regular_dir_path(
        &std::env::current_dir().map_err(|error| format!("cannot locate Git checkout: {error}"))?,
        "Git checkout",
    )
}

fn check_clean(repo: &Path) -> Result<(), String> {
    if std::env::var("STS2_RELEASE_ALLOW_DIRTY").as_deref() == Ok("1") {
        return Ok(());
    }
    let status = Command::new("git")
        .args([
            "-C",
            &repo.to_string_lossy(),
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
        ])
        .output()
        .map_err(|error| format!("could not inspect source checkout cleanliness: {error}"))?;
    if !status.status.success() {
        return fail("could not inspect source checkout cleanliness");
    }
    if !status.stdout.is_empty() {
        return fail(
            "source checkout must be clean (set STS2_RELEASE_ALLOW_DIRTY=1 only for fixtures)",
        );
    }
    Ok(())
}

fn git_output(repo: &Path, args: &[&str], label: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("{label} could not start: {error}"))?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return fail(format!(
            "{label} failed{}",
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        ));
    }
    Ok(output.stdout)
}

pub(crate) fn git_identity(repo: &Path, revision: &str) -> Result<(String, String), String> {
    let expression = format!("{revision}^{{commit}}");
    let commit = String::from_utf8(git_output(
        repo,
        &["rev-parse", "--verify", "--end-of-options", &expression],
        "source revision resolution",
    )?)
    .map_err(|_| "resolved source commit is not ASCII".to_owned())?
    .trim()
    .to_ascii_lowercase();
    let commit = full_hex(&commit, "resolved source commit")?;
    let tree_expression = format!("{commit}^{{tree}}");
    let tree = String::from_utf8(git_output(
        repo,
        &["rev-parse", "--verify", &tree_expression],
        "source tree resolution",
    )?)
    .map_err(|_| "resolved source tree is not ASCII".to_owned())?
    .trim()
    .to_ascii_lowercase();
    Ok((commit, full_hex(&tree, "resolved source tree")?))
}

pub(crate) fn selected_loader(
    repo: &Path,
    commit: &str,
) -> Result<(Map<String, Value>, Record), String> {
    let expression = format!("{commit}:{LOADER_MANIFEST}");
    let bytes = git_output(repo, &["show", &expression], "selected loader manifest")?;
    let object = parse_object(&bytes, "selected loader manifest")?;
    Ok((
        object,
        Record {
            path: LOADER_MANIFEST.into(),
            size_bytes: bytes.len() as u64,
            sha256: sha256_bytes(&bytes),
        },
    ))
}

pub(crate) fn check_identity(
    metadata: &crate::receipt::ReceiptMetadata,
    commit: &str,
    tree: &str,
    game: &str,
    package: &str,
    loader: &str,
) -> Result<(), String> {
    for (actual, expected, label) in [
        (&metadata.source_commit, commit, "source_commit"),
        (&metadata.source_tree, tree, "source_tree"),
        (&metadata.game_version, game, "game_version"),
        (&metadata.package_version, package, "package_version"),
        (
            &metadata.loader_manifest_version,
            loader,
            "loader_manifest_version",
        ),
    ] {
        if actual != expected {
            return fail(format!(
                "{} receipt {label} does not match the paired release identity",
                metadata.platform
            ));
        }
    }
    Ok(())
}

fn build_manifest(
    commit: &str,
    tree: &str,
    game: &str,
    package: &str,
    loader_version: &str,
    loader_record: &Record,
    roots: [&ReceiptRoot; 2],
) -> Result<Value, String> {
    let mut receipts = Map::new();
    let mut artifacts = Map::new();
    for root in roots {
        receipts.insert(
            root.metadata.platform.clone(),
            metadata_value(&root.metadata),
        );
        artifacts.insert(
            root.metadata.platform.clone(),
            Value::Array(root.metadata.payload.iter().map(record_value).collect()),
        );
    }
    let mut object = Map::new();
    object.insert("schema_version".into(), MANIFEST_SCHEMA.into());
    object.insert("source_commit".into(), commit.into());
    object.insert("source_tree".into(), tree.into());
    object.insert("game_version".into(), game.into());
    object.insert("package_version".into(), package.into());
    object.insert("loader_manifest_version".into(), loader_version.into());
    object.insert("loader_manifest".into(), record_value(loader_record));
    object.insert("receipts".into(), Value::Object(receipts));
    object.insert("artifacts".into(), Value::Object(artifacts));
    Ok(Value::Object(object))
}

fn write_manifest(output: &Path, mut data: Vec<u8>) -> Result<(), String> {
    data.push(b'\n');
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create output parent: {error}"))?;
        regular_dir_path(parent, "output manifest parent")?;
    }
    crate::common::ensure_absent(output, "output manifest")?;
    let claim_path = PathBuf::from(format!("{}.claim", output.display()));
    let claim = crate::common::claim(&claim_path)?;
    let temporary = output.with_file_name(format!(
        ".{}.tmp.{}",
        output.file_name().unwrap_or_default().to_string_lossy(),
        crate::common::temp_suffix()
    ));
    let result = write_new(&temporary, &data, "output manifest", 0o644)
        .and_then(|()| move_no_replace(&temporary, output, "output manifest"));
    crate::common::release_claim(&claim);
    result
}
