// SPDX-License-Identifier: MIT

use super::{
    LIVE_CARD_MAX_DETAIL_BYTES, LIVE_CARD_MAX_PAGE_ITEMS, LiveCardError, LiveCardReadReference,
    LiveCardSnapshot,
};

/// Capability disposition for the owner-local live-card source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LiveCardCapability {
    /// Only the deterministic fixture source currently implements this boundary.
    SyntheticFixtureOnly,
    /// The native/source boundary is unavailable.
    Unavailable(LiveCardUnavailableReason),
}

use super::LiveCardUnavailableReason;

impl LiveCardCapability {
    /// Returns whether a source can produce an owned snapshot.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::SyntheticFixtureOnly)
    }
}

/// Owner-local source that copies a coherent, owned live-card snapshot.
pub trait LiveCardSource {
    /// Reports the source capability without inspecting host objects.
    fn capability(&self) -> LiveCardCapability;

    /// Copies one immutable snapshot. No gameplay mutation is permitted.
    fn read_snapshot(&self) -> Result<LiveCardSnapshot, LiveCardError>;
}

/// Explicitly unavailable implementation until exact-host evidence and source wiring exist.
#[derive(Debug, Default)]
pub struct UnavailableLiveCardSource;

impl LiveCardSource for UnavailableLiveCardSource {
    fn capability(&self) -> LiveCardCapability {
        LiveCardCapability::Unavailable(LiveCardUnavailableReason::ExactHostEvidenceRequired)
    }

    fn read_snapshot(&self) -> Result<LiveCardSnapshot, LiveCardError> {
        Err(LiveCardError::Unavailable(
            LiveCardUnavailableReason::ExactHostEvidenceRequired,
        ))
    }
}

/// Synthetic in-memory source used by deterministic contract fixtures.
#[derive(Clone, Debug)]
pub struct FixtureLiveCardSource {
    snapshot: LiveCardSnapshot,
}

impl FixtureLiveCardSource {
    /// Creates a source that returns the supplied owned snapshot.
    #[must_use]
    pub fn new(snapshot: LiveCardSnapshot) -> Self {
        Self { snapshot }
    }

    /// Returns the snapshot fence without exposing a host object.
    #[must_use]
    pub fn reference(&self) -> &LiveCardReadReference {
        &self.snapshot.reference
    }
}

impl LiveCardSource for FixtureLiveCardSource {
    fn capability(&self) -> LiveCardCapability {
        LiveCardCapability::SyntheticFixtureOnly
    }

    fn read_snapshot(&self) -> Result<LiveCardSnapshot, LiveCardError> {
        Ok(self.snapshot.clone())
    }
}

/// Default local bounds used by a synthetic producer.
#[must_use]
pub const fn default_live_card_bounds() -> (usize, usize) {
    (LIVE_CARD_MAX_PAGE_ITEMS, LIVE_CARD_MAX_DETAIL_BYTES)
}
