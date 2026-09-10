// SPDX-License-Identifier: MIT

mod filesystem;
mod io;
mod json;

pub use filesystem::{
    OwnedPath, claim, move_no_replace, owned_path, release_claim, remove_owned_dir,
    remove_owned_file, run_logged, temp_suffix, write_new,
};
pub use io::{copy_bytes, copy_checked, digest, read_bounded};
pub use json::parse_object;

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const RECEIPT_SCHEMA: &str = "sts2-release-build-receipt-v1";
pub const MANIFEST_SCHEMA: &str = "sts2-build-artifact-manifest-v2";
pub const LOADER_MANIFEST: &str = "experiments/managed-rust-interop/game-loader/mod_manifest.json";
pub const MANAGED_NAME: &str = "AIAscensionSTS2GameMod.dll";
pub const ENTRYPOINT: &str = "AIAscensionSTS2GameMod.json";
pub const WINDOWS_NATIVE: &str = "AIAscensionSTS2GameModNative.dll";
pub const LINUX_NATIVE: &str = "libAIAscensionSTS2GameModNative.so";
pub const MAX_FILE_BYTES: u64 = 268_435_456;
pub const MAX_TEXT_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}

pub fn record_value(record: &Record) -> Value {
    let mut object = Map::new();
    object.insert("path".into(), Value::String(record.path.clone()));
    object.insert("size_bytes".into(), Value::Number(record.size_bytes.into()));
    object.insert("sha256".into(), Value::String(record.sha256.clone()));
    Value::Object(object)
}

pub fn fail<T>(message: impl Into<String>) -> Result<T, String> {
    Err(message.into())
}

pub fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn absolute(value: &str, label: &str) -> Result<PathBuf, String> {
    if value.is_empty() {
        return fail(format!("{label} must not be empty"));
    }
    let input = Path::new(value);
    let joined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("cannot resolve {label}: {error}"))?
            .join(input)
    };
    let mut path = PathBuf::new();
    for component in joined.components() {
        match component {
            Component::Prefix(prefix) => path.push(prefix.as_os_str()),
            Component::RootDir => path.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                path.pop();
            }
            Component::Normal(part) => path.push(part),
        }
    }
    reject_symlink_components(&path, label)?;
    Ok(path)
}

pub fn reject_symlink_components(path: &Path, label: &str) -> Result<(), String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir | Component::ParentDir => continue,
            Component::Normal(part) => current.push(part),
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return fail(format!(
                        "{label} path contains a symlink: {}",
                        current.display()
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return fail(format!("cannot inspect {label}: {error}")),
        }
    }
    Ok(())
}

pub fn input_path(value: &str, label: &str, allow_windows: bool) -> Result<PathBuf, String> {
    if windows_path(value) {
        if !allow_windows {
            return fail(format!("{label} must be a local WSL/POSIX path"));
        }
        return absolute(&convert_windows_path(value, label)?, label);
    }
    absolute(value, label)
}

pub fn windows_path(value: &str) -> bool {
    (value.len() >= 3
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[2], b'/' | b'\\'))
        || value.starts_with("\\\\")
}

fn convert_windows_path(value: &str, label: &str) -> Result<String, String> {
    let output = Command::new("wslpath")
        .args(["-u", "--", value])
        .output()
        .map_err(|error| format!("could not convert {label} with wslpath: {error}"))?;
    let converted = String::from_utf8(output.stdout)
        .map_err(|_| format!("could not convert {label} with wslpath"))?
        .trim()
        .to_owned();
    if !output.status.success() || converted.is_empty() || converted.contains('\n') {
        return fail(format!("could not convert {label} with wslpath"));
    }
    Ok(converted)
}

pub fn regular_file_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    reject_symlink_components(path, label)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => format!("{label} does not exist: {}", path.display()),
        _ => format!("cannot inspect {label}: {error}"),
    })?;
    if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
        return fail(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    Ok(path.to_path_buf())
}

pub fn regular_dir(value: &str, label: &str) -> Result<PathBuf, String> {
    let path = absolute(value, label)?;
    regular_dir_path(&path, label)
}

pub fn regular_dir_path(path: &Path, label: &str) -> Result<PathBuf, String> {
    reject_symlink_components(path, label)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => format!("{label} does not exist: {}", path.display()),
        _ => format!("cannot inspect {label}: {error}"),
    })?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return fail(format!(
            "{label} must be a regular non-symlink directory: {}",
            path.display()
        ));
    }
    Ok(path.to_path_buf())
}

pub fn exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

pub fn ensure_absent(path: &Path, label: &str) -> Result<(), String> {
    reject_symlink_components(path, label)?;
    if exists(path) {
        return fail(format!(
            "{label} already exists; reconcile it before retrying: {}",
            path.display()
        ));
    }
    Ok(())
}

pub fn paths_overlap(first: &Path, second: &Path) -> bool {
    first == second || first.starts_with(second) || second.starts_with(first)
}

pub fn ensure_disjoint(
    path: &Path,
    label: &str,
    protected: &Path,
    protected_label: &str,
) -> Result<(), String> {
    if paths_overlap(path, protected) {
        return fail(format!(
            "{label} overlaps {protected_label}; choose an external path"
        ));
    }
    Ok(())
}

pub fn safe_token(
    value: &str,
    label: &str,
    allow_slash: bool,
    allow_space: bool,
) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.contains("..") {
        return fail(format!("{label} contains an unsafe token"));
    }
    for character in value.chars() {
        let allowed = character.is_ascii_alphanumeric()
            || matches!(character, '.' | '_' | '-')
            || (allow_slash && character == '/')
            || (allow_space && matches!(character, ' ' | '(' | ')'));
        if !allowed {
            return fail(format!("{label} contains unsupported characters"));
        }
    }
    Ok(())
}

pub fn safe_text(value: &str, label: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES || value.contains("..") {
        return fail(format!("{label} is missing or too long"));
    }
    if value.chars().any(|character| {
        !character.is_ascii() || character.is_ascii_control() || matches!(character, '/' | '\\')
    }) {
        return fail(format!("{label} contains an unsafe path-like value"));
    }
    Ok(())
}

pub fn full_hex(value: &str, label: &str) -> Result<String, String> {
    if value.len() != 40
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return fail(format!("{label} must be a full lowercase Git object ID"));
    }
    Ok(value.to_owned())
}

pub fn sha256_hex(value: &str, label: &str) -> Result<String, String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return fail(format!("{label} must be a lowercase SHA-256 digest"));
    }
    Ok(value.to_owned())
}

pub fn bounded_size(value: &Value, label: &str) -> Result<u64, String> {
    let size = value
        .as_u64()
        .ok_or_else(|| format!("{label} has an invalid size"))?;
    if size == 0 || size > MAX_FILE_BYTES {
        return fail(format!("{label} has an invalid size"));
    }
    Ok(size)
}
