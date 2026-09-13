// SPDX-License-Identifier: MIT

//! Source-only public combat bookkeeping and draw-pile projection.
//!
//! This module copies a coherent, bounded combat snapshot into owned values.  It keeps live card
//! instance identity separate from definitions, exposes only explicitly permitted composition, and
//! never derives counters from an arbitrary before/after diff.  It is an owner-local contract
//! witness, not a native extractor, transport route, or gameplay simulator.

mod error;
mod measurement;
mod model;
mod reader;
mod reconciliation;
mod source;
mod validation;

pub use error::{
    CombatBookkeepingError, CombatCounterKind, CombatSourceError, CombatUnavailableReason,
};
pub use model::{
    CombatBookkeepingBinding, CombatBookkeepingSnapshot, CombatCardInstanceReference,
    CombatCardMembership, CombatCardPosition, CombatCompositionCompleteness, CombatCounter,
    CombatCounterProvenance, CombatCounterReset, CombatCounters, CombatDeckReconciliation,
    CombatField, CombatFieldStatus, CombatOrder, CombatPending, CombatResolutionState,
    CombatSnapshotInput, CombatTurnIdentity, CombatTurnOwner, CombatVisibilityScope, CombatZone,
    CombatZoneInput, CombatZoneInventory, CombatZoneKind, CombatZoneStatus,
};
pub use reader::CombatBookkeepingReader;
pub use source::{
    CombatBookkeepingCapability, CombatBookkeepingSource, FixtureCombatBookkeepingSource,
    UnavailableCombatBookkeepingSource,
};

/// Owner-local producer version.  This is not a wire or native ABI version.
pub const COMBAT_BOOKKEEPING_PRODUCER_VERSION: &str = "game-combat-bookkeeping-v1";
/// Maximum bytes for one identity component.
pub const COMBAT_BOOKKEEPING_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum source-owned zones in one snapshot.
pub const COMBAT_BOOKKEEPING_MAX_ZONES: usize = 16;
/// Maximum card memberships retained by one zone.
pub const COMBAT_BOOKKEEPING_MAX_ZONE_CARDS: usize = 512;
/// Maximum total cards reported by one zone.
pub const COMBAT_BOOKKEEPING_MAX_ZONE_TOTAL: u32 = 4_096;
/// Maximum pending public selection choices.
pub const COMBAT_BOOKKEEPING_MAX_PENDING_CHOICES: usize = 128;
/// Maximum bytes in a source-owned pending/effect identity.
pub const COMBAT_BOOKKEEPING_MAX_TEXT_BYTES: usize = 4 * 1024;
/// Maximum bytes retained by one source-owned combat snapshot.
pub const COMBAT_BOOKKEEPING_MAX_SNAPSHOT_BYTES: usize = 64 * 1024;
