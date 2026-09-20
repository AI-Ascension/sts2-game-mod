// SPDX-License-Identifier: MIT

//! Owner-reported result and prior-summary records, and the detail state of a listed summary.

use super::{
    ResultFinalization, ResultOrigin, RunOutcome, RunResultCatalogBinding, RunResultDuration,
    RunResultFieldValue, RunResultPersistence, RunResultQuantity, RunResultScore,
    RunResultVisibility, RunStatistic,
};

/// Kind of an ending-inventory entry.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultItemKind {
    /// A relic.
    Relic,
    /// A potion.
    Potion,
    /// An owner-defined item the reference does not name.
    Other {
        /// Owner-defined kind identity.
        entity_kind: String,
    },
}

/// One card in the deck a run ended with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultCardEntry {
    /// Namespaced card identity resolved against the content manifest.
    pub card_id: String,
    /// Copies held at the end of the run.
    pub count: u32,
    /// Whether the copies were upgraded.
    pub upgraded: bool,
}

/// One item in the inventory a run ended with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultItemEntry {
    /// Namespaced item identity resolved against the content manifest.
    pub item_id: String,
    /// Which family the identity resolves against.
    pub kind: RunResultItemKind,
    /// Copies held at the end of the run.
    pub count: u32,
}

/// Linkage from a result to a retained harness trajectory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultLinkage {
    /// Opaque retained trajectory identity.
    pub trajectory_id: String,
    /// Origin the trajectory itself came from, which need not match the result's own origin.
    pub trajectory_origin: ResultOrigin,
    /// Opaque supporting-evidence identity, stated absent when none was retained.
    pub evidence_id: RunResultFieldValue<String>,
}

/// Owner-reported completed-run record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultInput {
    /// Opaque result identity.
    pub result_id: String,
    /// Opaque owning profile identity.
    pub profile_id: String,
    /// Opaque run identity this result belongs to.
    pub run_id: String,
    /// Reported completion outcome.
    pub outcome: RunOutcome,
    /// Whether the result is a persisted fact or an in-progress observation.
    pub finalization: ResultFinalization,
    /// What the host reports about persisting this result.
    pub persistence: RunResultPersistence,
    /// Character the run was played with.
    pub character_id: RunResultFieldValue<String>,
    /// Run configuration this result belongs to.
    pub configuration_id: RunResultFieldValue<String>,
    /// Furthest act reached.
    pub act: RunResultFieldValue<u32>,
    /// Furthest floor reached.
    pub floor: RunResultFieldValue<u32>,
    /// Duration, stated with its clock kind.
    pub duration: RunResultFieldValue<RunResultDuration>,
    /// Deck held at the end of the run.
    pub ending_deck: RunResultFieldValue<Vec<RunResultCardEntry>>,
    /// Inventory held at the end of the run.
    pub ending_inventory: RunResultFieldValue<Vec<RunResultItemEntry>>,
    /// Displayed score total and its components.
    pub score: RunResultScore,
    /// Combat and run statistics.
    pub statistics: RunResultFieldValue<Vec<RunStatistic>>,
    /// Linkage to a retained trajectory, when identity evidence exists.
    pub linkage: RunResultFieldValue<RunResultLinkage>,
    /// Origin of this record.
    pub origin: ResultOrigin,
    /// Visibility class of this record.
    pub visibility: RunResultVisibility,
}

/// Whether a listed summary's detail can be read.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultDetailState {
    /// The detail record exists and may be read.
    Available,
    /// The host carries no detail for this summary.
    Unavailable,
}

/// Owner-reported prior run summary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunSummaryInput {
    /// Opaque summary identity.
    pub summary_id: String,
    /// Opaque owning profile identity.
    pub profile_id: String,
    /// Opaque run identity this summary belongs to.
    pub run_id: String,
    /// The result id whose detail backs this summary, absent when the host carries none.
    pub detail_result_id: RunResultFieldValue<String>,
    /// Reported completion outcome.
    pub outcome: RunOutcome,
    /// Character the run was played with.
    pub character_id: RunResultFieldValue<String>,
    /// Furthest act reached.
    pub act: RunResultFieldValue<u32>,
    /// Furthest floor reached.
    pub floor: RunResultFieldValue<u32>,
    /// Duration, stated with its clock kind.
    pub duration: RunResultFieldValue<RunResultDuration>,
    /// Displayed score total, stated absent when the mode reports none.
    pub score_total: RunResultFieldValue<RunResultQuantity>,
    /// Origin of this record.
    pub origin: ResultOrigin,
    /// Visibility class of this record.
    pub visibility: RunResultVisibility,
}

/// A validated result record bound to one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultRecord {
    /// Catalog binding this record was validated against.
    pub binding: RunResultCatalogBinding,
    /// Validated result value.
    pub result: RunResultInput,
}

impl RunResultRecord {
    /// Binds a validated input to the catalog it was validated against.
    #[must_use]
    pub fn from_input(binding: &RunResultCatalogBinding, result: RunResultInput) -> Self {
        Self {
            binding: binding.clone(),
            result,
        }
    }

    /// Returns the opaque result identity.
    #[must_use]
    pub fn result_id(&self) -> &str {
        &self.result.result_id
    }

    /// Returns whether this record's detail is readable as a persisted fact.
    #[must_use]
    pub const fn detail_state(&self) -> RunResultDetailState {
        match self.result.finalization {
            ResultFinalization::Finalized => RunResultDetailState::Available,
            ResultFinalization::PendingPersistence
            | ResultFinalization::Unsettled
            | ResultFinalization::Unavailable => RunResultDetailState::Unavailable,
        }
    }
}

/// A validated prior-summary record bound to one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunSummaryRecord {
    /// Catalog binding this record was validated against.
    pub binding: RunResultCatalogBinding,
    /// Validated summary value.
    pub summary: RunSummaryInput,
}

impl RunSummaryRecord {
    /// Binds a validated input to the catalog it was validated against.
    #[must_use]
    pub fn from_input(binding: &RunResultCatalogBinding, summary: RunSummaryInput) -> Self {
        Self {
            binding: binding.clone(),
            summary,
        }
    }

    /// Returns the opaque summary identity.
    #[must_use]
    pub fn summary_id(&self) -> &str {
        &self.summary.summary_id
    }

    /// Returns the detail state implied by the summary's own detail reference.
    #[must_use]
    pub fn detail_state(&self) -> RunResultDetailState {
        if self.summary.detail_result_id.is_present() {
            RunResultDetailState::Available
        } else {
            RunResultDetailState::Unavailable
        }
    }
}
