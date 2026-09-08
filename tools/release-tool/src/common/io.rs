// SPDX-License-Identifier: MIT

use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use sha2::{Digest, Sha256};

use super::{MAX_FILE_BYTES, fail, filesystem::write_new, regular_file_path};

pub fn digest(path: &Path, label: &str) -> Result<(u64, String), String> {
    regular_file_path(path, label)?;
    let mut file = File::open(path).map_err(|error| format!("cannot read {label}: {error}"))?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut chunk = [0u8; 1024 * 1024];
    loop {
        let count = file
            .read(&mut chunk)
            .map_err(|error| format!("cannot read {label}: {error}"))?;
        if count == 0 {
            break;
        }
        size = size.saturating_add(count as u64);
        hasher.update(&chunk[..count]);
    }
    if size == 0 || size > MAX_FILE_BYTES {
        return fail(format!("{label} has an invalid size: {}", path.display()));
    }
    Ok((size, format!("{:x}", hasher.finalize())))
}

pub fn read_bounded(path: &Path, label: &str, maximum: u64) -> Result<Vec<u8>, String> {
    regular_file_path(path, label)?;
    let file = File::open(path).map_err(|error| format!("cannot read {label}: {error}"))?;
    let mut bytes = Vec::new();
    file.take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {label}: {error}"))?;
    if bytes.is_empty() || bytes.len() as u64 > maximum {
        return fail(format!("{label} exceeds its byte bound"));
    }
    Ok(bytes)
}

pub fn copy_bytes(data: &[u8], destination: &Path, label: &str) -> Result<(), String> {
    write_new(destination, data, label, 0o600)
}

pub fn copy_checked(
    source: &Path,
    destination: &Path,
    expected_size: u64,
    expected_digest: &str,
) -> Result<(), String> {
    regular_file_path(source, "payload entry")?;
    let before =
        fs::metadata(source).map_err(|error| format!("cannot inspect payload: {error}"))?;
    let mut input = File::open(source).map_err(|error| format!("cannot copy payload: {error}"))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|error| format!("cannot copy payload: {error}"))?;
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut chunk = [0u8; 1024 * 1024];
    loop {
        let count = input
            .read(&mut chunk)
            .map_err(|error| format!("cannot copy payload: {error}"))?;
        if count == 0 {
            break;
        }
        if size.saturating_add(count as u64) > MAX_FILE_BYTES {
            return fail(format!(
                "payload exceeds its byte bound while staging: {}",
                source.display()
            ));
        }
        output
            .write_all(&chunk[..count])
            .map_err(|error| format!("cannot copy payload: {error}"))?;
        hasher.update(&chunk[..count]);
        size = size.saturating_add(count as u64);
    }
    output
        .sync_all()
        .map_err(|error| format!("cannot persist payload: {error}"))?;
    if size != expected_size || format!("{:x}", hasher.finalize()) != expected_digest {
        return fail(format!(
            "payload bytes do not match the immutable build manifest: {}",
            source.display()
        ));
    }
    let after = fs::metadata(source).map_err(|error| format!("cannot inspect payload: {error}"))?;
    if !same_state(&before, &after) {
        return fail(format!(
            "payload changed while it was being staged: {}",
            source.display()
        ));
    }
    Ok(())
}

fn same_state(before: &Metadata, after: &Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.len() == after.len()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        before.volume_serial_number() == after.volume_serial_number()
            && before.file_index() == after.file_index()
            && before.file_size() == after.file_size()
            && before.last_write_time() == after.last_write_time()
    }
    #[cfg(not(any(unix, windows)))]
    {
        before.len() == after.len()
    }
}
