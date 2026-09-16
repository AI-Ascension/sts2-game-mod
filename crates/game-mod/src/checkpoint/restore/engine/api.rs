// SPDX-License-Identifier: MIT

use serde_json::Value;

use super::super::wire::WireError;

/// Capability reported by a game-thread exact restore adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactRestoreCapability {
    /// No verified exact-host restore port is installed.
    Unavailable,
    /// A test or future host adapter is installed.
    Available,
}

/// Exact closure bytes handed to a restore adapter after independent closure verification.
///
/// The debug view deliberately reveals lengths and digest labels only, never artifact contents.
#[derive(Clone)]
pub struct RestoreClosureView<'a> {
    /// Exact checkpoint manifest bytes.
    pub manifest: &'a [u8],
    /// Distinct canonical/restore blob bytes in first-reference order.
    pub blobs: Vec<(&'a str, &'a [u8])>,
}

impl std::fmt::Debug for RestoreClosureView<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RestoreClosureView")
            .field("manifest_bytes", &self.manifest.len())
            .field(
                "blobs",
                &self
                    .blobs
                    .iter()
                    .map(|(digest, bytes)| (*digest, bytes.len()))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Result from a restore adapter after it has attempted its single host effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RestoreApplyOutcome {
    /// The adapter independently recaptured the destination exact state.
    Verified {
        /// Digest of the independently recaptured canonical destination state.
        recaptured_exact_state_digest: String,
    },
    /// The host effect may have started or its result could not be established.
    Unknown,
}

/// Host-thread restore adapter. Production currently has no implementation.
pub trait RestoreHostApplier {
    /// Reports whether this exact host/build has a verified restore capability.
    fn capability(&self) -> ExactRestoreCapability;

    /// Performs the single host effect and independently recaptures destination state.
    fn restore(&mut self, request: &Value, closure: RestoreClosureView<'_>) -> RestoreApplyOutcome;
}

/// Production restore adapter that always reports unavailable.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoExactRestoreApplier;

impl RestoreHostApplier for NoExactRestoreApplier {
    fn capability(&self) -> ExactRestoreCapability {
        ExactRestoreCapability::Unavailable
    }

    fn restore(
        &mut self,
        _request: &Value,
        _closure: RestoreClosureView<'_>,
    ) -> RestoreApplyOutcome {
        RestoreApplyOutcome::Unknown
    }
}

/// Durable staging backend required before the owner accepts restore bytes.
pub trait ExactRestoreStore {
    /// Reads the bounded metadata index. Absence returns an empty byte vector.
    fn read_index(&mut self) -> Result<Vec<u8>, ExactRestoreError>;
    /// Atomically replaces the metadata index and durably syncs the private directory.
    fn write_index(&mut self, index: &[u8]) -> Result<(), ExactRestoreError>;
    /// Appends one bounded chunk and durably records its offset boundary.
    fn append_chunk(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        bytes: &[u8],
    ) -> Result<(), ExactRestoreError>;
    /// Reads a committed blob up to the caller's explicit limit.
    fn read_blob(&mut self, key: &str, maximum_bytes: u64) -> Result<Vec<u8>, ExactRestoreError>;
    /// Reads a bounded committed blob range for idempotent duplicate comparison.
    fn read_blob_range(
        &mut self,
        key: &str,
        offset: u64,
        bytes: usize,
    ) -> Result<Vec<u8>, ExactRestoreError>;
    /// Checks whether a byte offset is the start of an already accepted chunk.
    fn has_chunk_start(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
    ) -> Result<bool, ExactRestoreError>;
    /// Returns the next recorded chunk start after `offset`, bounded by committed progress.
    fn next_chunk_start(
        &mut self,
        key: &str,
        total_bytes: u64,
        offset: u64,
        committed_bytes: u64,
    ) -> Result<u64, ExactRestoreError>;
    /// Truncates crash-tail bytes and removes uncommitted boundary bits.
    fn reconcile_blob(
        &mut self,
        key: &str,
        total_bytes: u64,
        committed_bytes: u64,
    ) -> Result<(), ExactRestoreError>;
    /// Removes only filenames derived from one validated operation UUID.
    fn remove_operation(&mut self, operation_id: &str) -> Result<(), ExactRestoreError>;
}

/// Exact restore state machine error returned to the owning local route.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactRestoreError {
    /// The operation or frame could not be interpreted.
    InvalidFrame,
    /// The frame is valid JSON but is not the pinned canonical wire encoding.
    NonCanonicalFrame,
    /// Request frame exceeds the 16 KiB contract bound.
    FrameTooLarge,
    /// The caller or configured principal does not match.
    Unauthorized,
    /// The current owner tuple or lease is stale.
    StaleOwner,
    /// No authoritative owner provider is installed.
    OwnerUnavailable,
    /// The configured host restore adapter is unavailable.
    NoRestoreAdapter,
    /// An operation UUID was reused with different immutable begin data.
    OperationConflict,
    /// One operation is in a phase that does not permit this request.
    InvalidPhase,
    /// An artifact digest is not part of the accepted closure.
    UnknownArtifact,
    /// Chunk offset, size, or duplicate data conflicts with staged progress.
    ChunkConflict,
    /// A staged artifact or closure digest does not match its declaration.
    DigestMismatch,
    /// Operation capacity is full and no entry may be evicted.
    CapacityFull,
    /// Durable storage failed closed.
    StorageUnavailable,
}

impl std::fmt::Display for ExactRestoreError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidFrame => "invalid exact-restore frame",
            Self::NonCanonicalFrame => "exact-restore frame is not canonical",
            Self::FrameTooLarge => "exact-restore frame exceeds its bound",
            Self::Unauthorized => "exact-restore principal is not authorized",
            Self::StaleOwner => "exact-restore owner is stale",
            Self::OwnerUnavailable => "authoritative exact-restore owner is unavailable",
            Self::NoRestoreAdapter => "exact-restore host adapter is unavailable",
            Self::OperationConflict => "exact-restore operation identity conflicts",
            Self::InvalidPhase => "exact-restore phase is not valid for operation state",
            Self::UnknownArtifact => "exact-restore artifact is not in the accepted closure",
            Self::ChunkConflict => "exact-restore chunk conflicts with staged offsets",
            Self::DigestMismatch => "exact-restore digest does not match staged bytes",
            Self::CapacityFull => "exact-restore operation capacity is full",
            Self::StorageUnavailable => "exact-restore durable storage is unavailable",
        })
    }
}

impl std::error::Error for ExactRestoreError {}

impl From<WireError> for ExactRestoreError {
    fn from(error: WireError) -> Self {
        match error {
            WireError::FrameSize => Self::FrameTooLarge,
            WireError::InvalidJson | WireError::InvalidFrame | WireError::UnsupportedContract => {
                Self::InvalidFrame
            }
            WireError::NonCanonical => Self::NonCanonicalFrame,
            WireError::SchemaUnavailable | WireError::RandomUnavailable => Self::StorageUnavailable,
        }
    }
}
