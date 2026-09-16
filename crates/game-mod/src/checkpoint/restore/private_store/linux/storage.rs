// SPDX-License-Identifier: MIT

use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::checkpoint::restore::ExactRestoreError;

use super::bitmap::{clear_from, set_boundary};
use super::files::{
    bits_name, check_private_regular, data_name, data_name_checked, open_new_regular_file,
    open_private_directory, open_regular_file, open_regular_file_optional, rename_at,
    safe_filename, stat_file, sync_directory, unlink_at, valid_key, valid_operation_id,
};

const INDEX_FILE: &str = "index.json";
const LOCK_FILE: &str = "owner.lock";
const MAX_INDEX_BYTES: u64 = 16 * 1024 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(in crate::checkpoint::restore::private_store) struct PrivateDirectory {
    pub(super) root: File,
    _lock: File,
}

impl PrivateDirectory {
    pub(in crate::checkpoint::restore::private_store) fn open(
        path: &Path,
    ) -> Result<Self, ExactRestoreError> {
        let root = open_private_directory(path)?;
        let lock = open_regular_file(root.as_raw_fd(), LOCK_FILE, true)?;
        let lock_stat = stat_file(&lock)?;
        check_private_regular(&lock_stat)?;
        // SAFETY: `lock` is a live regular descriptor retained for the store's lifetime.
        // flock is advisory and prevents a second process from writing the same index.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        Ok(Self { root, _lock: lock })
    }

    pub(in crate::checkpoint::restore::private_store) fn read_index(
        &self,
    ) -> Result<Vec<u8>, ExactRestoreError> {
        let file = match open_regular_file_optional(self.root.as_raw_fd(), INDEX_FILE)? {
            Some(value) => value,
            None => return Ok(Vec::new()),
        };
        let metadata = stat_file(&file)?;
        check_private_regular(&metadata)?;
        if metadata.st_size < 0 || metadata.st_size as u64 > MAX_INDEX_BYTES {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut bytes = Vec::with_capacity(metadata.st_size as usize);
        file.take(MAX_INDEX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        Ok(bytes)
    }

    pub(in crate::checkpoint::restore::private_store) fn write_index(
        &self,
        bytes: &[u8],
    ) -> Result<(), ExactRestoreError> {
        if bytes.len() as u64 > MAX_INDEX_BYTES {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let name = format!(
            "index-{}-{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let mut file = open_new_regular_file(self.root.as_raw_fd(), &name)?;
        file.write_all(bytes)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        file.sync_all()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        rename_at(self.root.as_raw_fd(), &name, INDEX_FILE)?;
        sync_directory(self.root.as_raw_fd())
    }

    pub(in crate::checkpoint::restore::private_store) fn append_chunk(
        &self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), ExactRestoreError> {
        if !valid_key(key)
            || total_bytes == 0
            || total_bytes > 16 * 1024 * 1024
            || bytes.is_empty()
            || bytes.len() > 8192
            || offset
                .checked_add(bytes.len() as u64)
                .is_none_or(|end| end > total_bytes)
        {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut data = open_regular_file(self.root.as_raw_fd(), &data_name(key), true)?;
        let metadata = stat_file(&data)?;
        check_private_regular(&metadata)?;
        if metadata.st_size < 0 || metadata.st_size as u64 != offset {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        data.seek(SeekFrom::End(0))
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        data.write_all(bytes)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        data.sync_all()
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;

        let bit_name = bits_name(key);
        let mut bits = open_regular_file(self.root.as_raw_fd(), &bit_name, true)?;
        let metadata = stat_file(&bits)?;
        check_private_regular(&metadata)?;
        let bit_length = total_bytes.div_ceil(8);
        if metadata.st_size == 0 {
            bits.set_len(bit_length)
                .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        } else if metadata.st_size < 0 || metadata.st_size as u64 != bit_length {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        set_boundary(&mut bits, offset)?;
        bits.sync_all()
            .map_err(|_| ExactRestoreError::StorageUnavailable)
    }

    pub(in crate::checkpoint::restore::private_store) fn read_blob(
        &self,
        key: &str,
        maximum_bytes: u64,
    ) -> Result<Vec<u8>, ExactRestoreError> {
        let file =
            match open_regular_file_optional(self.root.as_raw_fd(), &data_name_checked(key)?)? {
                Some(value) => value,
                None => return Ok(Vec::new()),
            };
        let metadata = stat_file(&file)?;
        check_private_regular(&metadata)?;
        if metadata.st_size < 0 || metadata.st_size as u64 > maximum_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut bytes = Vec::with_capacity(metadata.st_size as usize);
        file.take(maximum_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        if bytes.len() as u64 > maximum_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        Ok(bytes)
    }

    pub(in crate::checkpoint::restore::private_store) fn reconcile_blob(
        &self,
        key: &str,
        total_bytes: u64,
        committed_bytes: u64,
    ) -> Result<(), ExactRestoreError> {
        if !valid_key(key) || committed_bytes > total_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let data_name = data_name(key);
        match open_regular_file_optional(self.root.as_raw_fd(), &data_name)? {
            Some(file) => {
                let metadata = stat_file(&file)?;
                check_private_regular(&metadata)?;
                if metadata.st_size < 0 || (metadata.st_size as u64) < committed_bytes {
                    return Err(ExactRestoreError::StorageUnavailable);
                }
                if metadata.st_size as u64 > committed_bytes {
                    file.set_len(committed_bytes)
                        .map_err(|_| ExactRestoreError::StorageUnavailable)?;
                    file.sync_all()
                        .map_err(|_| ExactRestoreError::StorageUnavailable)?;
                }
            }
            None if committed_bytes != 0 => {
                return Err(ExactRestoreError::StorageUnavailable);
            }
            None => {}
        }
        let Some(mut bits) = open_regular_file_optional(self.root.as_raw_fd(), &bits_name(key))?
        else {
            return if committed_bytes == 0 {
                Ok(())
            } else {
                Err(ExactRestoreError::StorageUnavailable)
            };
        };
        let metadata = stat_file(&bits)?;
        check_private_regular(&metadata)?;
        let expected = total_bytes.div_ceil(8);
        if metadata.st_size < 0 || metadata.st_size as u64 != expected {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let mut bitmap = vec![0_u8; expected as usize];
        bits.read_exact(&mut bitmap)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        clear_from(&mut bitmap, committed_bytes);
        bits.seek(SeekFrom::Start(0))
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        bits.write_all(&bitmap)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        bits.sync_all()
            .map_err(|_| ExactRestoreError::StorageUnavailable)
    }

    pub(in crate::checkpoint::restore::private_store) fn remove_operation(
        &self,
        operation_id: &str,
    ) -> Result<(), ExactRestoreError> {
        if !valid_operation_id(operation_id) {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let prefix = format!("{operation_id}-");
        let descriptor_path = format!("/proc/self/fd/{}", self.root.as_raw_fd());
        let entries =
            fs::read_dir(descriptor_path).map_err(|_| ExactRestoreError::StorageUnavailable)?;
        for entry in entries {
            let entry = entry.map_err(|_| ExactRestoreError::StorageUnavailable)?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(&prefix) && safe_filename(&name) {
                unlink_at(self.root.as_raw_fd(), &name)?;
            }
        }
        sync_directory(self.root.as_raw_fd())
    }
}
