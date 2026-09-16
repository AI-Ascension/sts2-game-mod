// SPDX-License-Identifier: MIT

#![allow(
    unsafe_code,
    reason = "Linux openat/fstat/flock provide the reviewed no-follow private-store boundary"
)]

use std::path::Path;

use super::engine::{ExactRestoreError, ExactRestoreStore};

#[cfg(target_os = "linux")]
#[path = "private_store/linux.rs"]
mod linux;

/// Linux private-directory store for durable restore metadata and staged blobs.
///
/// Windows and other platforms fail closed until an equivalent no-follow backend is reviewed.
#[derive(Debug)]
pub struct SecureExactRestoreStore {
    #[cfg(target_os = "linux")]
    inner: linux::PrivateDirectory,
}

impl SecureExactRestoreStore {
    /// Opens or creates one configured owner-private directory without following path symlinks.
    pub fn open(path: &Path) -> Result<Self, ExactRestoreError> {
        #[cfg(target_os = "linux")]
        {
            Ok(Self {
                inner: linux::PrivateDirectory::open(path)?,
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = path;
            Err(ExactRestoreError::StorageUnavailable)
        }
    }
}

#[cfg(target_os = "linux")]
impl ExactRestoreStore for SecureExactRestoreStore {
    fn read_index(&mut self) -> Result<Vec<u8>, ExactRestoreError> {
        self.inner.read_index()
    }

    fn write_index(&mut self, index: &[u8]) -> Result<(), ExactRestoreError> {
        self.inner.write_index(index)
    }

    fn append_chunk(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), ExactRestoreError> {
        self.inner.append_chunk(key, total_bytes, offset, bytes)
    }

    fn read_blob(&mut self, key: &str, maximum_bytes: u64) -> Result<Vec<u8>, ExactRestoreError> {
        self.inner.read_blob(key, maximum_bytes)
    }

    fn read_blob_range(
        &mut self,
        key: &str,
        offset: u64,
        bytes: usize,
    ) -> Result<Vec<u8>, ExactRestoreError> {
        self.inner.read_blob_range(key, offset, bytes)
    }

    fn has_chunk_start(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
    ) -> Result<bool, ExactRestoreError> {
        self.inner.has_chunk_start(key, total_bytes, offset)
    }

    fn next_chunk_start(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        committed_bytes: u64,
    ) -> Result<u64, ExactRestoreError> {
        self.inner
            .next_chunk_start(key, total_bytes, offset, committed_bytes)
    }

    fn reconcile_blob(
        &mut self,
        key: &str,
        total_bytes: u64,
        committed_bytes: u64,
    ) -> Result<(), ExactRestoreError> {
        self.inner.reconcile_blob(key, total_bytes, committed_bytes)
    }

    fn remove_operation(&mut self, operation_id: &str) -> Result<(), ExactRestoreError> {
        self.inner.remove_operation(operation_id)
    }
}

#[cfg(not(target_os = "linux"))]
impl ExactRestoreStore for SecureExactRestoreStore {
    fn read_index(&mut self) -> Result<Vec<u8>, ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn write_index(&mut self, _index: &[u8]) -> Result<(), ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn append_chunk(
        &mut self,
        _key: &str,
        _total_bytes: u64,
        _offset: u64,
        _bytes: &[u8],
    ) -> Result<(), ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn read_blob(&mut self, _key: &str, _maximum_bytes: u64) -> Result<Vec<u8>, ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn read_blob_range(
        &mut self,
        _key: &str,
        _offset: u64,
        _bytes: usize,
    ) -> Result<Vec<u8>, ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn has_chunk_start(
        &mut self,
        _key: &str,
        _total_bytes: u64,
        _offset: u64,
    ) -> Result<bool, ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn next_chunk_start(
        &mut self,
        _key: &str,
        _total_bytes: u64,
        _offset: u64,
        _committed_bytes: u64,
    ) -> Result<u64, ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn reconcile_blob(
        &mut self,
        _key: &str,
        _total_bytes: u64,
        _committed_bytes: u64,
    ) -> Result<(), ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }

    fn remove_operation(&mut self, _operation_id: &str) -> Result<(), ExactRestoreError> {
        Err(ExactRestoreError::StorageUnavailable)
    }
}
