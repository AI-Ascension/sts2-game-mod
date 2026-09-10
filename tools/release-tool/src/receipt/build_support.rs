// SPDX-License-Identifier: MIT

mod fixtures;
mod production;

pub(crate) use fixtures::{fixture_inputs, host_records, payload_records};
pub(crate) use production::production_inputs;

use std::collections::BTreeMap;
use std::env;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{Map, Value};

use crate::common::{
    ensure_absent, ensure_disjoint, fail, full_hex, parse_object, regular_dir_path, safe_token,
};

#[derive(Debug)]
pub(crate) struct Args {
    pub(crate) platform: String,
    pub(crate) source_commit: String,
    pub(crate) game_version: String,
    pub(crate) package_version: String,
    pub(crate) host_data_dir: Option<String>,
    pub(crate) output_dir: String,
    pub(crate) evidence_dir: Option<String>,
    pub(crate) dotnet: String,
    pub(crate) scope: String,
    pub(crate) fixture_payload_dir: Option<String>,
}

#[derive(Debug)]
pub(crate) struct Tool {
    pub(crate) path: PathBuf,
    pub(crate) style: &'static str,
}

#[derive(Debug)]
pub(crate) struct Inputs {
    pub(crate) managed: PathBuf,
    pub(crate) native: PathBuf,
    pub(crate) toolchain: Map<String, Value>,
    pub(crate) hosts: Vec<Value>,
    pub(crate) host_values: BTreeMap<String, (u64, String)>,
}

#[derive(Debug)]
pub(crate) struct HostData {
    pub(crate) records: Vec<Value>,
    pub(crate) values: BTreeMap<String, (u64, String)>,
}

pub(crate) fn parse_args(arguments: &[String]) -> Result<Args, String> {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    let mut scope = "production".to_owned();
    let mut dotnet = env::var("DOTNET_BIN").unwrap_or_else(|_| "dotnet".into());
    let mut index = 0;
    while index < arguments.len() {
        let key = &arguments[index];
        if key == "--help" || key == "-h" {
            println!(
                "build-runtime-receipt.sh --platform PLATFORM --source-commit COMMIT --game-version VERSION --package-version VERSION --output-dir DIR [--host-data-dir DIR] [--evidence-dir DIR] [--dotnet EXE] [--scope production|fixture --fixture-payload-dir DIR]"
            );
            return fail("help requested");
        }
        if key == "--scope" {
            scope = arguments
                .get(index + 1)
                .ok_or("--scope requires a value")?
                .clone();
            index += 2;
            continue;
        }
        if key == "--dotnet" {
            dotnet = arguments
                .get(index + 1)
                .ok_or("--dotnet requires a value")?
                .clone();
            index += 2;
            continue;
        }
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("{key} requires a value"))?;
        if !matches!(
            key.as_str(),
            "--platform"
                | "--source-commit"
                | "--game-version"
                | "--package-version"
                | "--host-data-dir"
                | "--output-dir"
                | "--evidence-dir"
                | "--fixture-payload-dir"
        ) {
            return fail(format!("unknown option: {key}"));
        }
        if values.insert(key.clone(), value.clone()).is_some() {
            return fail(format!("duplicate option: {key}"));
        }
        index += 2;
    }
    let get = |key: &str| {
        values
            .get(key)
            .cloned()
            .ok_or_else(|| format!("{key} is required"))
    };
    let game = get("--game-version")?;
    let package = get("--package-version")?;
    safe_token(&game, "game version", false, false)?;
    safe_token(&package, "package version", false, false)?;
    if !matches!(scope.as_str(), "production" | "fixture") {
        return fail("scope must be production or fixture");
    }
    let fixture = values.get("--fixture-payload-dir").cloned();
    let host = values.get("--host-data-dir").cloned();
    if scope == "production" && fixture.is_some() {
        return fail("fixture payload requires --scope fixture");
    }
    if scope == "fixture" && fixture.is_none() {
        return fail("fixture scope requires --fixture-payload-dir");
    }
    if scope == "production" && host.is_none() {
        return fail("production scope requires --host-data-dir");
    }
    Ok(Args {
        platform: get("--platform")?,
        source_commit: get("--source-commit")?,
        game_version: game,
        package_version: package,
        host_data_dir: host,
        output_dir: get("--output-dir")?,
        evidence_dir: values.get("--evidence-dir").cloned(),
        dotnet,
        scope,
        fixture_payload_dir: fixture,
    })
}

pub(crate) fn input_dir(value: &str, label: &str, allow_windows: bool) -> Result<PathBuf, String> {
    regular_dir_path(
        &crate::common::input_path(value, label, allow_windows)?,
        label,
    )
}

pub(crate) fn repository() -> Result<PathBuf, String> {
    let value = env::var_os("STS2_RELEASE_REPO_ROOT")
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
        .ok_or("cannot locate Git checkout")?;
    regular_dir_path(&value, "Git checkout")
}

fn git_output(repo: &Path, arguments: &[&str], label: &str) -> Result<Vec<u8>, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(arguments)
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("{label} could not start: {error}"))?;
    if !output.status.success() {
        return fail(format!("{label} failed"));
    }
    Ok(output.stdout)
}

pub(crate) fn checkout_identity(
    repo: &Path,
    requested: &str,
    require_clean: bool,
) -> Result<(String, String), String> {
    let head = String::from_utf8(git_output(repo, &["rev-parse", "HEAD"], "checkout HEAD")?)
        .map_err(|_| "checkout HEAD is not ASCII".to_owned())?
        .trim()
        .to_ascii_lowercase();
    let requested = full_hex(&requested.to_ascii_lowercase(), "source commit")?;
    if head != requested {
        return fail("source commit does not equal checkout HEAD");
    }
    if require_clean {
        let status = git_output(
            repo,
            &["status", "--porcelain=v1", "--untracked-files=all"],
            "checkout cleanliness",
        )?;
        if !status.is_empty() {
            return fail("production receipt requires a clean checkout");
        }
    }
    let tree_expression = format!("{head}^{{tree}}");
    let tree = String::from_utf8(git_output(
        repo,
        &["rev-parse", "--verify", &tree_expression],
        "checkout tree",
    )?)
    .map_err(|_| "checkout tree is not ASCII".to_owned())?
    .trim()
    .to_ascii_lowercase();
    Ok((head, full_hex(&tree, "checkout tree")?))
}

pub(crate) fn source_loader(repo: &Path, commit: &str) -> Result<(Vec<u8>, String), String> {
    let expression = format!("{commit}:{}", crate::common::LOADER_MANIFEST);
    let bytes = git_output(repo, &["show", &expression], "selected loader manifest")?;
    let object = parse_object(&bytes, "selected loader manifest")?;
    let version = object
        .get("version")
        .and_then(Value::as_str)
        .ok_or("selected loader manifest version is missing")?;
    safe_token(version, "loader manifest version", false, false)?;
    Ok((bytes, version.into()))
}

pub(crate) fn validate_roots(
    output: &Path,
    evidence: &Path,
    repo: &Path,
    host: Option<&Path>,
) -> Result<(), String> {
    ensure_disjoint(output, "output directory", repo, "Git checkout")?;
    ensure_disjoint(evidence, "evidence directory", repo, "Git checkout")?;
    ensure_disjoint(output, "output directory", evidence, "evidence directory")?;
    if let Some(host) = host {
        ensure_disjoint(output, "output directory", host, "host data directory")?;
        ensure_disjoint(evidence, "evidence directory", host, "host data directory")?;
    }
    Ok(())
}

pub(crate) fn prepare(output: &Path, evidence: &Path) -> Result<(), String> {
    ensure_absent(output, "output directory")?;
    ensure_absent(
        &PathBuf::from(format!("{}.claim", output.display())),
        "output claim",
    )?;
    ensure_absent(evidence, "evidence directory")?;
    let parent = output.parent().ok_or("output parent is missing")?;
    fs::create_dir_all(parent).map_err(|error| format!("cannot create output parent: {error}"))?;
    regular_dir_path(parent, "output parent")?;
    ensure_absent(output, "output directory")?;
    ensure_absent(
        &PathBuf::from(format!("{}.claim", output.display())),
        "output claim",
    )?;
    fs::create_dir(evidence)
        .map_err(|error| format!("cannot create evidence directory: {error}"))?;
    set_private(evidence)?;
    fs::create_dir(evidence.join("logs"))
        .map_err(|error| format!("cannot create evidence logs: {error}"))?;
    set_private(&evidence.join("logs"))?;
    Ok(())
}

pub(crate) fn set_private(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|error| format!("cannot set private permissions: {error}"))
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(())
    }
}
