// SPDX-License-Identifier: MIT

use super::model::CombatBookkeepingBinding;
use super::{
    CombatBookkeepingError, CombatSnapshotInput, CombatSourceError, CombatUnavailableReason,
};

/// Source capability for owner-local combat bookkeeping.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CombatBookkeepingCapability {
    /// Only deterministic in-memory fixtures implement this boundary.
    SyntheticFixtureOnly,
    /// No supported source is attached.
    Unavailable(CombatUnavailableReason),
}

impl CombatBookkeepingCapability {
    /// Returns whether a source can produce an owned snapshot.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::SyntheticFixtureOnly)
    }
}

/// Owner-local source that copies one coherent combat snapshot.
pub trait CombatBookkeepingSource {
    /// Reports capability without inspecting host objects.
    fn capability(&self) -> CombatBookkeepingCapability;

    /// Copies a coherent snapshot for the exact expected binding and scope.
    fn read_snapshot(
        &self,
        expected: &CombatBookkeepingBinding,
        scope: super::CombatVisibilityScope,
    ) -> Result<CombatSnapshotInput, CombatSourceError>;
}

/// Explicitly unavailable source until exact-host evidence and wiring exist.
#[derive(Debug, Default)]
pub struct UnavailableCombatBookkeepingSource;

impl CombatBookkeepingSource for UnavailableCombatBookkeepingSource {
    fn capability(&self) -> CombatBookkeepingCapability {
        CombatBookkeepingCapability::Unavailable(CombatUnavailableReason::ExactHostEvidenceRequired)
    }

    fn read_snapshot(
        &self,
        _expected: &CombatBookkeepingBinding,
        _scope: super::CombatVisibilityScope,
    ) -> Result<CombatSnapshotInput, CombatSourceError> {
        Err(CombatSourceError::NoActiveSource)
    }
}

/// Deterministic source fixture used by owner-local tests.
#[derive(Clone, Debug)]
pub struct FixtureCombatBookkeepingSource {
    snapshot: CombatSnapshotInput,
}

impl FixtureCombatBookkeepingSource {
    /// Creates a fixture source from owned input.
    #[must_use]
    pub fn new(snapshot: CombatSnapshotInput) -> Self {
        Self { snapshot }
    }

    /// Returns the fixture binding without exposing a host object.
    #[must_use]
    pub fn binding(&self) -> &CombatBookkeepingBinding {
        &self.snapshot.binding
    }
}

impl CombatBookkeepingSource for FixtureCombatBookkeepingSource {
    fn capability(&self) -> CombatBookkeepingCapability {
        CombatBookkeepingCapability::SyntheticFixtureOnly
    }

    fn read_snapshot(
        &self,
        expected: &CombatBookkeepingBinding,
        _scope: super::CombatVisibilityScope,
    ) -> Result<CombatSnapshotInput, CombatSourceError> {
        if &self.snapshot.binding != expected {
            return Err(CombatSourceError::Stale);
        }
        Ok(self.snapshot.clone())
    }
}

/// Maps a source error without exposing host exception text.
pub(super) fn map_source_error(error: CombatSourceError) -> CombatBookkeepingError {
    match error {
        CombatSourceError::NoActiveSource => {
            CombatBookkeepingError::Unavailable(CombatUnavailableReason::NoActiveSource)
        }
        CombatSourceError::AccessDenied => {
            CombatBookkeepingError::Unavailable(CombatUnavailableReason::ScopeDenied)
        }
        CombatSourceError::Busy => CombatBookkeepingError::Busy,
        CombatSourceError::Stale => CombatBookkeepingError::SourceStale,
        CombatSourceError::Malformed => CombatBookkeepingError::InvalidInput("source"),
    }
}
