// SPDX-License-Identifier: MIT

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::{ADDON_ID, Plan, prepare, unique_json};

pub(super) fn digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn plain(path: &Path) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
    {
        return Err("absolute paths without traversal are required".into());
    }
    for ancestor in path.ancestors() {
        let metadata = fs::symlink_metadata(ancestor).map_err(|_| "path inspection failed")?;
        if metadata.file_type().is_symlink() {
            return Err("linked paths are not supported".into());
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err("reparse paths are not supported".into());
            }
        }
    }
    Ok(())
}

fn read(path: &Path, bound: u64) -> Result<Vec<u8>, String> {
    plain(path)?;
    let file = File::open(path).map_err(|_| "cannot open settings or addon file")?;
    if !file
        .metadata()
        .map_err(|_| "cannot inspect input file")?
        .is_file()
    {
        return Err("input must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(bound + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "cannot read input file")?;
    if bytes.len() as u64 > bound {
        return Err("input file exceeds byte bound".into());
    }
    Ok(bytes)
}

fn check_addon(mods: &Path) -> Result<(), String> {
    plain(mods)?;
    let allowed = [
        "AIAscensionSTS2GameMod.json",
        "AIAscensionSTS2GameMod.dll",
        "AIAscensionSTS2GameMod.deps.json",
        "AIAscensionSTS2GameMod.pdb",
        "AIAscensionSTS2GameModNative.dll",
        "libAIAscensionSTS2GameModNative.so",
    ];
    for entry in fs::read_dir(mods).map_err(|_| "cannot inspect addon directory")? {
        let entry = entry.map_err(|_| "cannot inspect addon entry")?;
        if !allowed.iter().any(|name| entry.file_name() == *name)
            || !entry
                .file_type()
                .map_err(|_| "cannot inspect addon type")?
                .is_file()
        {
            return Err(
                "mod-loading preparation requires only the intended addon in this directory".into(),
            );
        }
    }
    let manifest = unique_json::parse(&read(&mods.join("AIAscensionSTS2GameMod.json"), 65536)?)?;
    if manifest["id"] != ADDON_ID || manifest["has_dll"] != true || manifest["has_pck"] != false {
        return Err("installed manifest does not identify the supported addon".into());
    }
    if read(&mods.join("AIAscensionSTS2GameMod.dll"), 16 * 1024 * 1024)?.is_empty() {
        return Err("intended addon DLL is empty".into());
    }
    Ok(())
}

/// Read and validate a candidate without creating files or changing the profile.
pub fn inspect(settings: &Path, mods: &Path) -> Result<Plan, String> {
    check_addon(mods)?;
    prepare(&read(settings, 1024 * 1024)?)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "cannot create exclusive output file")?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "cannot persist output file".into())
}

/// Apply a digest-fenced candidate while the caller keeps the selected game stopped.
/// Returns the backup directory's relative label, or `None` for an unchanged file.
pub fn apply(
    settings: &Path,
    mods: &Path,
    backups: &Path,
    expected: &str,
) -> Result<Option<String>, String> {
    plain(backups)?;
    let parent = settings.parent().ok_or("settings parent is missing")?;
    plain(parent)?;
    let lock = parent.join(".sts2-mod-loading.lock");
    fs::create_dir(&lock)
        .map_err(|_| "another preparation is active or its lock needs inspection")?;
    let outcome = apply_locked(settings, mods, backups, expected);
    fs::remove_dir(&lock)
        .map_err(|_| "mod-loading lock cleanup failed; inspect the settings and backup")?;
    outcome
}

fn apply_locked(
    settings: &Path,
    mods: &Path,
    backups: &Path,
    expected: &str,
) -> Result<Option<String>, String> {
    let plan = inspect(settings, mods)?;
    if expected.len() != 64 || plan.before_sha256 != expected {
        return Err("settings changed since review; no update applied".into());
    }
    if !plan.changed {
        return Ok(None);
    }
    let label = format!(
        "mod-loading-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "clock is before epoch")?
            .as_nanos()
    );
    let backup = backups.join(&label);
    let mut builder = fs::DirBuilder::new();
    builder.recursive(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .create(&backup)
        .map_err(|_| "cannot create exclusive backup directory")?;
    write_new(&backup.join("settings.before.json"), &plan.before)?;
    write_new(
        &backup.join("digests.json"),
        serde_json::json!({"before_sha256":plan.before_sha256,
        "after_sha256":plan.after_sha256,"addon_id":ADDON_ID})
        .to_string()
        .as_bytes(),
    )?;
    let candidate = settings.with_file_name(format!(".{label}.candidate"));
    write_new(&candidate, &plan.after)?;
    let result = (|| {
        let metadata = fs::metadata(settings).map_err(|_| "cannot inspect settings permissions")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            std::os::unix::fs::chown(&candidate, Some(metadata.uid()), Some(metadata.gid()))
                .map_err(|_| "cannot preserve settings ownership")?;
        }
        let permissions = metadata.permissions();
        fs::set_permissions(&candidate, permissions)
            .map_err(|_| "cannot preserve settings permissions")?;
        if read(settings, 1024 * 1024)? != plan.before {
            return Err("settings changed before replacement".into());
        }
        check_addon(mods)?;
        fs::rename(&candidate, settings).map_err(|_| "atomic settings replacement failed")?;
        if read(settings, 1024 * 1024)? != plan.after {
            return Err("settings readback failed; inspect backup".into());
        }
        Ok(Some(label))
    })();
    if candidate.exists() {
        fs::remove_file(&candidate).map_err(|_| "candidate cleanup failed; inspect backup")?;
    }
    result
}
