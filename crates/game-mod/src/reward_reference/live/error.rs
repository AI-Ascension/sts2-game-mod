// SPDX-License-Identifier: MIT

use super::super::{RewardCatalogError, RewardSourceError};

/// Sanitized source, validation, and retained-read failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardLiveError {
    /// The source failed before an owned snapshot could be retained.
    Source(RewardSourceError),
    /// A static reward reference failed the existing catalog's binding or visibility checks.
    Catalog(RewardCatalogError),
    /// The source snapshot did not match the reader's static or live identity.
    BindingMismatch(&'static str),
    /// The source returned invalid, duplicate, or internally inconsistent values.
    InvalidInput(&'static str),
    /// One retained offer exceeded its source-owned detail bound.
    DetailTooLarge { limit: usize, actual: usize },
    /// The complete retained snapshot exceeded its source-owned aggregate bound.
    SnapshotTooLarge { limit: usize, actual: usize },
    /// Checked source-byte accounting overflowed.
    AccountingOverflow,
    /// The requested page size was zero or exceeded the local bound.
    InvalidPageSize,
    /// A requested live reference is not in the current retained snapshot.
    NotFound,
    /// A live reference belongs to a previous capture.
    StaleReference,
    /// A continuation belongs to a previous capture or another query.
    StaleContinuation,
    /// A handle or continuation belongs to another reader instance.
    WrongReader,
    /// No successful snapshot is currently retained.
    NoCurrentSnapshot,
    /// The reader-local generation counter was exhausted; the reader is disabled.
    GenerationExhausted,
}

impl From<RewardCatalogError> for RewardLiveError {
    fn from(error: RewardCatalogError) -> Self {
        Self::Catalog(error)
    }
}

impl std::fmt::Display for RewardLiveError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RewardLiveError {}
