// SPDX-License-Identifier: MIT

//! Catalog binding, declared support state, and the live fence a current-result read holds.

use crate::ContentCursorBinding;

/// Source-only producer identity; this is not a wire or native ABI version.
pub const RUN_RESULT_REFERENCE_PRODUCER_VERSION: &str = "game-run-result-reference-producer-v1";
/// Maximum bytes accepted for one opaque identity.
pub const RUN_RESULT_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const RUN_RESULT_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one result record.
pub const RUN_RESULT_MAX_RESULT_BYTES: usize = 128 * 1024;
/// Maximum aggregate bytes retained for one prior-summary record.
pub const RUN_RESULT_MAX_SUMMARY_BYTES: usize = 32 * 1024;
/// Maximum completed-run results in one source snapshot.
pub const RUN_RESULT_MAX_RESULTS: usize = 1_024;
/// Maximum prior run summaries in one source snapshot.
pub const RUN_RESULT_MAX_SUMMARIES: usize = 4_096;
/// Maximum entries returned by one bounded page.
pub const RUN_RESULT_MAX_PAGE_ITEMS: usize = 64;
/// Maximum score components on one result.
pub const RUN_RESULT_MAX_SCORE_COMPONENTS: usize = 64;
/// Maximum statistics on one result.
pub const RUN_RESULT_MAX_STATISTICS: usize = 64;
/// Maximum ending-deck entries on one result.
pub const RUN_RESULT_MAX_DECK_ENTRIES: usize = 256;
/// Maximum ending-inventory entries on one result.
pub const RUN_RESULT_MAX_INVENTORY_ENTRIES: usize = 256;
/// Manifest family resolved for ending-deck card references.
pub const RUN_RESULT_CARD_KIND: &str = "card";
/// Manifest family resolved for ending-inventory relic references.
pub const RUN_RESULT_RELIC_KIND: &str = "relic";
/// Manifest family resolved for ending-inventory potion references.
pub const RUN_RESULT_POTION_KIND: &str = "potion";

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunResultCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized result value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Declared support state for the result family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultFamilyState {
    /// The source reports result records.
    Handled,
    /// The supported build reports no results, so the catalog is explicitly empty.
    Unavailable,
}

/// Declared support state with the record count the source reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultFamilyCoverage {
    /// Whether the source reports results at all.
    pub state: RunResultFamilyState,
    /// Number of result records the source declares.
    pub result_count: usize,
    /// Number of prior summaries the source declares.
    pub summary_count: usize,
}

/// The live fence a current-result read holds.
///
/// A current result is the active run's own terminal state, so reading it requires the instance,
/// run and epoch it was observed at. History reads do not require it and do not move it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunResultLiveFence {
    /// Opaque instance identity the observation was taken from.
    pub instance_id: String,
    /// Opaque run identity the observation was taken from.
    pub run_id: String,
    /// Monotonic epoch of that instance's observation stream.
    pub epoch: u64,
}

/// Reference to a manifest definition the result records resolve against.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunResultManifestReference {
    /// Manifest entity kind, such as `card`.
    pub entity_kind: String,
    /// Namespaced manifest identity.
    pub namespaced_id: String,
}

/// Exact reference to one retained completed-run result.
///
/// The reference carries its catalog witness, so a reference produced for another manifest, locale
/// or producer is refused instead of being resolved against the current snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunResultReference {
    /// Catalog binding this reference was produced for.
    pub catalog: RunResultCatalogBinding,
    /// Opaque result identity.
    pub result_id: String,
}

/// Exact reference to one retained prior-summary record.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunSummaryReference {
    /// Catalog binding this reference was produced for.
    pub catalog: RunResultCatalogBinding,
    /// Opaque summary identity.
    pub summary_id: String,
}
