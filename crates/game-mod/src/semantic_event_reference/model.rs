// SPDX-License-Identifier: MIT

//! Bounds, catalog binding, declared support state, the scope one history belongs to, and the live
//! fence a current-history read holds.

use crate::ContentCursorBinding;

/// Source-only producer identity; this is not a wire or native ABI version.
pub const SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION: &str =
    "game-semantic-event-reference-producer-v1";
/// Maximum bytes accepted for one opaque identity.
pub const SEMANTIC_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum dot-separated segments accepted in one opaque identity.
pub const SEMANTIC_MAX_IDENTITY_SEGMENTS: usize = 8;
/// Maximum bytes accepted for one owner-defined label.
pub const SEMANTIC_MAX_LABEL_BYTES: usize = 1024;
/// Maximum bytes accepted for the unit of one quantity.
pub const SEMANTIC_MAX_UNIT_BYTES: usize = 64;
/// Maximum events, observed or disclosed, retained for one history.
pub const SEMANTIC_MAX_EVENTS: usize = 4096;
/// Maximum declared coverage intervals in one capture window.
pub const SEMANTIC_MAX_INTERVALS: usize = 64;
/// Maximum aggregate bytes retained for one history.
pub const SEMANTIC_MAX_HISTORY_BYTES: usize = 512 * 1024;
/// Maximum entries returned by one bounded page.
pub const SEMANTIC_MAX_PAGE_ITEMS: usize = 64;

/// A bounded gameplay quantity: an amount and the unit it is stated in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticQuantity {
    /// Signed amount; a resource change may be negative, an applied effect is never zero.
    pub amount: i64,
    /// Opaque unit the amount is stated in, for example `health`, `block` or `energy`.
    pub unit: String,
}

/// One content identity an event names, resolved against the content manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticReference {
    /// Manifest entity kind the identity resolves against.
    pub entity_kind: String,
    /// Namespaced identity resolved against the content manifest.
    pub namespaced_id: String,
}

/// Static catalog identity: content manifest and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Declared support state for the semantic-history family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticFamilyState {
    /// The source reports a history record.
    Handled,
    /// The supported build reports no history, so the catalog is explicitly empty.
    Unavailable,
}

/// Declared support state with the counts the source reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticFamilyCoverage {
    /// Whether the source reports a history at all.
    pub state: SemanticFamilyState,
    /// Number of observed events the source declares.
    pub event_count: usize,
    /// Number of disclosed gaps the source declares.
    pub gap_count: usize,
}

/// The run, branch, episode and epoch one history belongs to.
///
/// Sequence numbers are only monotonic inside one such scope, so the scope travels with every event
/// and every reference instead of being assumed to be "the current run".
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticEventScope {
    /// Opaque run identity.
    pub run_id: String,
    /// Opaque branch identity inside that run.
    pub branch_id: String,
    /// Episode number inside that branch.
    pub episode: u64,
    /// Monotonic epoch of the observation stream.
    pub epoch: u64,
}

impl SemanticEventScope {
    /// Returns whether two scopes name the same place, ignoring the observation epoch.
    #[must_use]
    pub fn same_place(&self, other: &Self) -> bool {
        self.run_id == other.run_id
            && self.branch_id == other.branch_id
            && self.episode == other.episode
    }
}

/// The live fence a current-history read holds.
///
/// A history belongs to the run and branch it was observed in, at the epoch the observation was
/// taken. A read that names another place is stale, and a read at another epoch is refused rather
/// than answered with this history.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticHistoryFence {
    /// Opaque run identity the observation was taken from.
    pub run_id: String,
    /// Opaque branch identity the observation was taken from.
    pub branch_id: String,
    /// Episode number the observation was taken at.
    pub episode: u64,
    /// Monotonic epoch of that observation stream.
    pub epoch: u64,
}

/// Exact reference to one retained event.
///
/// The reference carries its catalog witness, so a reference produced for another manifest or
/// producer is refused instead of being resolved against the current snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticEventReference {
    /// Catalog binding this reference was produced for.
    pub catalog: SemanticCatalogBinding,
    /// Opaque event identity.
    pub event_id: String,
}
