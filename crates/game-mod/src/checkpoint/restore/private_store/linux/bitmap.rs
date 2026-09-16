// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::checkpoint::restore::ExactRestoreError;

pub(super) fn set_boundary(file: &mut File, offset: u64) -> Result<(), ExactRestoreError> {
    file.seek(SeekFrom::Start(offset / 8))
        .map_err(|_| ExactRestoreError::StorageUnavailable)?;
    let mut current = [0_u8; 1];
    file.read_exact(&mut current)
        .map_err(|_| ExactRestoreError::StorageUnavailable)?;
    current[0] |= 1 << (offset % 8);
    file.seek(SeekFrom::Start(offset / 8))
        .map_err(|_| ExactRestoreError::StorageUnavailable)?;
    file.write_all(&current)
        .map_err(|_| ExactRestoreError::StorageUnavailable)
}

pub(super) fn clear_from(bitmap: &mut [u8], offset: u64) {
    let byte = (offset / 8) as usize;
    let bit = (offset % 8) as u32;
    if let Some(value) = bitmap.get_mut(byte) {
        *value &= (1_u8 << bit).wrapping_sub(1);
        for value in bitmap.iter_mut().skip(byte + 1) {
            *value = 0;
        }
    } else {
        bitmap.fill(0);
    }
}
