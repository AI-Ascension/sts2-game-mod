// SPDX-License-Identifier: MIT

use super::binding::{
    RetainedMapLiveBinding, RetainedMapObservationState, RetainedMapVisibilityScope,
};
use super::error::{
    RetainedMapError, RetainedMapSourceError, RetainedMapUnavailableReason, map_source_error,
};
use super::model::RetainedMapSnapshotInput;

/// Sanitized result of one owner-local retained map observation.
pub type RetainedMapSourceResult = Result<RetainedMapSnapshotInput, RetainedMapSourceError>;

/// Capability of an owner-local retained map source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedMapCapability {
    /// Only deterministic in-memory fixtures implement this boundary.
    SyntheticFixtureOnly,
    /// No supported source is attached.
    Unavailable(RetainedMapUnavailableReason),
}

impl RetainedMapCapability {
    /// Returns whether a source can produce an owned observation.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::SyntheticFixtureOnly)
    }
}

/// Owner-local source that copies one coherent already-public map observation.
pub trait RetainedMapSource {
    /// Reports capability without inspecting host objects.
    fn capability(&self) -> RetainedMapCapability;

    /// Reports whether the map surface is currently open and permitted.
    fn observation_state(&self) -> RetainedMapObservationState;

    /// Copies a coherent observation for the exact expected identity and scope.
    ///
    /// Implementations must not open, scroll, or navigate the map UI.
    fn read_snapshot(
        &self,
        expected: &RetainedMapLiveBinding,
        scope: RetainedMapVisibilityScope,
    ) -> RetainedMapSourceResult;
}

/// Explicitly unavailable source until exact-host visibility evidence exists.
#[derive(Debug, Default)]
pub struct UnavailableRetainedMapSource;

impl RetainedMapSource for UnavailableRetainedMapSource {
    fn capability(&self) -> RetainedMapCapability {
        RetainedMapCapability::Unavailable(RetainedMapUnavailableReason::ExactHostEvidenceRequired)
    }

    fn observation_state(&self) -> RetainedMapObservationState {
        RetainedMapObservationState::Unsupported
    }

    fn read_snapshot(
        &self,
        _expected: &RetainedMapLiveBinding,
        _scope: RetainedMapVisibilityScope,
    ) -> RetainedMapSourceResult {
        Err(RetainedMapSourceError::NoActiveSource)
    }
}

/// Deterministic source fixture used by owner-local tests.
#[derive(Clone, Debug)]
pub struct FixtureRetainedMapSource {
    snapshot: RetainedMapSnapshotInput,
    observation: RetainedMapObservationState,
}

impl FixtureRetainedMapSource {
    /// Creates an observable fixture source from owned input.
    #[must_use]
    pub fn new(snapshot: RetainedMapSnapshotInput) -> Self {
        Self {
            snapshot,
            observation: RetainedMapObservationState::Observable,
        }
    }

    /// Creates a fixture source whose map surface has the given state.
    #[must_use]
    pub fn with_observation(
        snapshot: RetainedMapSnapshotInput,
        observation: RetainedMapObservationState,
    ) -> Self {
        Self {
            snapshot,
            observation,
        }
    }

    /// Returns the fixture's exact binding without exposing a host object.
    #[must_use]
    pub fn binding(&self) -> &RetainedMapLiveBinding {
        &self.snapshot.binding
    }

    /// Returns whether the fixture surface is observable.
    #[must_use]
    pub const fn is_observable(&self) -> bool {
        matches!(self.observation, RetainedMapObservationState::Observable)
    }
}

impl RetainedMapSource for FixtureRetainedMapSource {
    fn capability(&self) -> RetainedMapCapability {
        RetainedMapCapability::SyntheticFixtureOnly
    }

    fn observation_state(&self) -> RetainedMapObservationState {
        self.observation
    }

    fn read_snapshot(
        &self,
        expected: &RetainedMapLiveBinding,
        _scope: RetainedMapVisibilityScope,
    ) -> RetainedMapSourceResult {
        if &self.snapshot.binding != expected {
            return Err(RetainedMapSourceError::Stale);
        }
        Ok(self.snapshot.clone())
    }
}

/// Maps a source error without exposing host exception text.
pub(super) fn map_error(error: RetainedMapSourceError) -> RetainedMapError {
    map_source_error(error)
}
