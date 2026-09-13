// SPDX-License-Identifier: MIT

use super::ProfileIdentityError;

/// Source-local discovery failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileDiscoveryError {
    /// The exact host integration is not available.
    UnavailableHost,
    /// The selected instance does not match the owner source.
    WrongInstance,
    /// The selected user-data identity does not match the owner source.
    WrongUserData,
    /// The supplied baseline is stale or differs from the owner source.
    StaleBaseline,
}

impl std::fmt::Display for ProfileDiscoveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProfileDiscoveryError {}

/// Fixture construction failure, kept separate from host and selection failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileFixtureError {
    /// More than the bounded number of slots was supplied.
    TooManySlots,
    /// A slot identity was repeated.
    DuplicateSlot,
    /// The current selection was not present in the summary list.
    SelectionNotListed,
    /// A summary carried a different baseline from the discovery fence.
    BaselineMismatch,
    /// A baseline referred to a different user-data identity.
    BaselineUserDataMismatch,
    /// An identity constructor failed.
    InvalidIdentity(ProfileIdentityError),
}

impl std::fmt::Display for ProfileFixtureError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProfileFixtureError {}

/// Explicit selection rejection; no raw host error crosses this boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileSelectionRejection {
    /// Host-thread integration is unavailable.
    UnavailableHost,
    /// The explicit instance identity is wrong.
    WrongInstance,
    /// The explicit user-data identity is wrong.
    WrongUserData,
    /// The baseline fence is stale or mismatched.
    StaleBaseline,
    /// The expected slot is not present.
    UnknownSlot,
    /// The host does not support this slot.
    UnsupportedSlot,
    /// The slot has an active run.
    ActiveRun,
    /// Another owner operation currently uses the slot.
    InUse,
    /// A save is pending.
    PendingSave,
    /// The owner reported a failed save.
    FailedSave,
    /// Another selection operation is in flight.
    ConcurrentSelection,
    /// The key was previously used for a different request.
    IdempotencyConflict,
    /// No committed result is available for reconciliation.
    UnknownOperation,
}

impl std::fmt::Display for ProfileSelectionRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProfileSelectionRejection {}
