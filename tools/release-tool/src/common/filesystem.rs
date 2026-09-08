// SPDX-License-Identifier: MIT

use std::fs::{self, Metadata, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use super::{ensure_absent, exists, fail};

pub fn write_new(path: &Path, data: &[u8], label: &str, mode: u32) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let mut file = options
        .open(path)
        .map_err(|error| format!("cannot write {label}: {error}"))?;
    let owner = identity(
        &file
            .metadata()
            .map_err(|error| format!("cannot inspect {label}: {error}"))?,
    );
    if let Err(error) = file.write_all(data).and_then(|()| file.sync_all()) {
        drop(file);
        remove_owned_file_identity(path, &owner);
        return Err(format!("cannot persist {label}: {error}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(mode)) {
            remove_owned_file_identity(path, &owner);
            return Err(format!("cannot set {label} mode: {error}"));
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug)]
struct Identity {
    #[cfg(unix)]
    device: u64,
    first: u64,
    #[cfg(not(unix))]
    second: u64,
}

fn identity(metadata: &Metadata) -> Identity {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Identity {
            device: metadata.dev(),
            first: metadata.ino(),
        }
    }
    #[cfg(not(unix))]
    {
        Identity {
            first: metadata.len(),
            second: metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |value| value.as_nanos() as u64),
        }
    }
}

pub struct Claim {
    pub path: PathBuf,
    id: Identity,
}

pub fn claim(path: &Path) -> Result<Claim, String> {
    ensure_absent(path, "output claim")?;
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("could not claim output: {error}"))?;
    let id = match file.metadata() {
        Ok(metadata) => identity(&metadata),
        Err(error) => {
            drop(file);
            return Err(format!(
                "cannot inspect output claim; claim retained for reconciliation: {error}"
            ));
        }
    };
    if let Err(error) = writeln!(file, "pid={}", std::process::id()).and_then(|()| file.sync_all())
    {
        drop(file);
        remove_owned_file_identity(path, &id);
        return Err(format!("could not persist output claim: {error}"));
    }
    Ok(Claim {
        path: path.to_path_buf(),
        id,
    })
}

pub fn release_claim(claim: &Claim) {
    let Ok(metadata) = fs::symlink_metadata(&claim.path) else {
        return;
    };
    if same_identity(&identity(&metadata), &claim.id) {
        let _ = fs::remove_file(&claim.path);
    }
}

fn same_identity(left: &Identity, right: &Identity) -> bool {
    #[cfg(unix)]
    {
        left.device == right.device && left.first == right.first
    }
    #[cfg(not(unix))]
    {
        left.first == right.first && left.second == right.second
    }
}

#[derive(Debug)]
pub struct OwnedPath {
    path: PathBuf,
    id: Identity,
}

pub fn owned_path(path: &Path, label: &str) -> Result<OwnedPath, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("cannot inspect {label}: {error}"))?;
    Ok(OwnedPath {
        path: path.to_path_buf(),
        id: identity(&metadata),
    })
}

pub fn remove_owned_dir(owner: &OwnedPath) {
    let Ok(metadata) = fs::symlink_metadata(&owner.path) else {
        return;
    };
    if same_identity(&identity(&metadata), &owner.id) {
        let _ = fs::remove_dir_all(&owner.path);
    }
}

pub fn remove_owned_file(owner: &OwnedPath) {
    let Ok(metadata) = fs::symlink_metadata(&owner.path) else {
        return;
    };
    if same_identity(&identity(&metadata), &owner.id) {
        let _ = fs::remove_file(&owner.path);
    }
}

fn remove_owned_file_identity(path: &Path, owner: &Identity) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if same_identity(&identity(&metadata), owner) {
        let _ = fs::remove_file(path);
    }
}

pub fn move_no_replace(source: &Path, destination: &Path, label: &str) -> Result<(), String> {
    let before = fs::symlink_metadata(source)
        .map_err(|error| format!("cannot inspect {label} source: {error}"))?;
    let source_id = identity(&before);
    let parent = destination
        .parent()
        .ok_or_else(|| format!("{label} parent is missing"))?;
    let parent_metadata = fs::symlink_metadata(parent)
        .map_err(|error| format!("cannot inspect {label} parent: {error}"))?;
    if !same_filesystem(&source_id, &identity(&parent_metadata)) {
        return fail(format!(
            "{label} source and destination parent must be on the same filesystem"
        ));
    }
    let result = Command::new("mv")
        .args([
            "-nT",
            &source.to_string_lossy(),
            &destination.to_string_lossy(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| format!("could not claim {label} atomically: {error}"))?;
    if !result.status.success() || exists(source) {
        return fail(format!("could not claim {label} atomically"));
    }
    let after = fs::symlink_metadata(destination)
        .map_err(|error| format!("cannot inspect published {label}: {error}"))?;
    if !same_identity(&source_id, &identity(&after)) {
        return fail(format!("published {label} did not retain staged identity"));
    }
    Ok(())
}

fn same_filesystem(left: &Identity, right: &Identity) -> bool {
    #[cfg(unix)]
    {
        left.device == right.device
    }
    #[cfg(not(unix))]
    {
        let _ = right;
        true
    }
}

pub fn temp_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    format!("{}-{nanos}", std::process::id())
}

pub fn run_logged(command: &mut Command, log: &Path, label: &str) -> Result<(), String> {
    let output = command
        .output()
        .map_err(|error| format!("{label} could not start: {error}"))?;
    let mut bytes = output.stdout;
    bytes.extend_from_slice(&output.stderr);
    write_new(log, &bytes, "task-private build log", 0o600)?;
    if !output.status.success() {
        return fail(format!(
            "{label} failed; inspect the task-private build log"
        ));
    }
    Ok(())
}
