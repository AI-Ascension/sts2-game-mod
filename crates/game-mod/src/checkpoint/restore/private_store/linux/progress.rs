// SPDX-License-Identifier: MIT

use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;

use crate::checkpoint::restore::ExactRestoreError;

use super::files::{
    bits_name, bits_name_checked, check_private_regular, data_name_checked,
    open_regular_file_optional, stat_file, valid_key,
};
use super::storage::PrivateDirectory;

impl PrivateDirectory {
    pub(in crate::checkpoint::restore::private_store) fn read_blob_range(
        &self,
        key: &str,
        offset: u64,
        bytes: usize,
    ) -> Result<Vec<u8>, ExactRestoreError> {
        let Some(mut file) =
            open_regular_file_optional(self.root.as_raw_fd(), &data_name_checked(key)?)?
        else {
            return if bytes == 0 {
                Ok(Vec::new())
            } else {
                Err(ExactRestoreError::StorageUnavailable)
            };
        };
        let end = offset
            .checked_add(bytes as u64)
            .ok_or(ExactRestoreError::StorageUnavailable)?;
        let metadata = stat_file(&file)?;
        check_private_regular(&metadata)?;
        if metadata.st_size < 0 || end > metadata.st_size as u64 {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        file.seek(SeekFrom::Start(offset))
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let mut output = vec![0_u8; bytes];
        file.read_exact(&mut output)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        Ok(output)
    }

    pub(in crate::checkpoint::restore::private_store) fn has_chunk_start(
        &self,
        key: &str,
        total_bytes: u64,
        offset: u64,
    ) -> Result<bool, ExactRestoreError> {
        if !valid_key(key) || offset >= total_bytes {
            return Ok(false);
        }
        let Some(mut bits) = open_regular_file_optional(self.root.as_raw_fd(), &bits_name(key))?
        else {
            return Ok(false);
        };
        let metadata = stat_file(&bits)?;
        check_private_regular(&metadata)?;
        if metadata.st_size < 0 || metadata.st_size as u64 != total_bytes.div_ceil(8) {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        bits.seek(SeekFrom::Start(offset / 8))
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let mut byte = [0_u8; 1];
        bits.read_exact(&mut byte)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        Ok(byte[0] & (1 << (offset % 8)) != 0)
    }

    pub(in crate::checkpoint::restore::private_store) fn next_chunk_start(
        &self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        committed_bytes: u64,
    ) -> Result<u64, ExactRestoreError> {
        if committed_bytes > total_bytes {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let Some(mut bits) =
            open_regular_file_optional(self.root.as_raw_fd(), &bits_name_checked(key)?)?
        else {
            return Ok(committed_bytes);
        };
        let metadata = stat_file(&bits)?;
        check_private_regular(&metadata)?;
        if metadata.st_size < 0 || metadata.st_size as u64 != total_bytes.div_ceil(8) {
            return Err(ExactRestoreError::StorageUnavailable);
        }
        let first = offset.saturating_add(1);
        if first >= committed_bytes {
            return Ok(committed_bytes);
        }
        let start_byte = first / 8;
        bits.seek(SeekFrom::Start(start_byte))
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let scan_length = committed_bytes.div_ceil(8).saturating_sub(start_byte) as usize;
        let mut buffer = vec![0_u8; scan_length];
        bits.read_exact(&mut buffer)
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        for (index, value) in buffer.into_iter().enumerate() {
            let base = start_byte + index as u64;
            let min_bit = if base == start_byte { first % 8 } else { 0 };
            let max_bit = if base + 1 == committed_bytes.div_ceil(8) {
                (committed_bytes % 8).max(1)
            } else {
                8
            };
            for bit in min_bit..max_bit {
                if value & (1 << bit) != 0 {
                    return Ok(base * 8 + bit);
                }
            }
        }
        Ok(committed_bytes)
    }
}
