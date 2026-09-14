// SPDX-License-Identifier: MIT

use super::{
    EnemyIntentError, EnemyIntentLiveBinding, EnemyIntentSnapshotInput, EnemyIntentSourceError,
    EnemyIntentUnavailableReason, EnemyIntentVisibilityScope, error::map_source_error,
};

/// Capability of an owner-local structured enemy-intent source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemyIntentCapability {
    /// Only deterministic in-memory fixtures implement this boundary.
    SyntheticFixtureOnly,
    /// No supported source is attached.
    Unavailable(EnemyIntentUnavailableReason),
}

impl EnemyIntentCapability {
    /// Returns whether a source can produce an owned snapshot.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::SyntheticFixtureOnly)
    }
}

/// Owner-local source that copies one coherent enemy/intent snapshot.
pub trait EnemyIntentSource {
    /// Reports capability without inspecting host objects.
    fn capability(&self) -> EnemyIntentCapability;

    /// Copies a coherent snapshot for the exact expected identity and scope.
    fn read_snapshot(
        &self,
        expected: &EnemyIntentLiveBinding,
        scope: EnemyIntentVisibilityScope,
    ) -> Result<EnemyIntentSnapshotInput, EnemyIntentSourceError>;
}

/// Explicitly unavailable source until exact-host field evidence exists.
#[derive(Debug, Default)]
pub struct UnavailableEnemyIntentSource;

impl EnemyIntentSource for UnavailableEnemyIntentSource {
    fn capability(&self) -> EnemyIntentCapability {
        EnemyIntentCapability::Unavailable(EnemyIntentUnavailableReason::ExactHostEvidenceRequired)
    }

    fn read_snapshot(
        &self,
        _expected: &EnemyIntentLiveBinding,
        _scope: EnemyIntentVisibilityScope,
    ) -> Result<EnemyIntentSnapshotInput, EnemyIntentSourceError> {
        Err(EnemyIntentSourceError::NoActiveSource)
    }
}

/// Deterministic source fixture used by owner-local tests.
#[derive(Clone, Debug)]
pub struct FixtureEnemyIntentSource {
    snapshot: EnemyIntentSnapshotInput,
}

impl FixtureEnemyIntentSource {
    /// Creates a fixture source from owned values.
    #[must_use]
    pub fn new(snapshot: EnemyIntentSnapshotInput) -> Self {
        Self { snapshot }
    }

    /// Returns the fixture's exact binding without exposing a host object.
    #[must_use]
    pub fn binding(&self) -> &EnemyIntentLiveBinding {
        &self.snapshot.binding
    }
}

impl EnemyIntentSource for FixtureEnemyIntentSource {
    fn capability(&self) -> EnemyIntentCapability {
        EnemyIntentCapability::SyntheticFixtureOnly
    }

    fn read_snapshot(
        &self,
        expected: &EnemyIntentLiveBinding,
        _scope: EnemyIntentVisibilityScope,
    ) -> Result<EnemyIntentSnapshotInput, EnemyIntentSourceError> {
        if &self.snapshot.binding != expected {
            return Err(EnemyIntentSourceError::Stale);
        }
        Ok(self.snapshot.clone())
    }
}

/// Maps a source error without exposing host exception text.
pub(super) fn map_error(error: EnemyIntentSourceError) -> EnemyIntentError {
    map_source_error(error)
}
