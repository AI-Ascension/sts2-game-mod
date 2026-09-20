// SPDX-License-Identifier: MIT

//! Per-field availability for a reported result, so unknown never becomes zero or empty.

/// Field of a completed-run result.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultField {
    /// The reported completion outcome.
    Outcome,
    /// The character the run was played with.
    Character,
    /// The run configuration the result belongs to.
    Configuration,
    /// The furthest act the run reached.
    Act,
    /// The furthest floor the run reached.
    Floor,
    /// The run's wall-clock or in-game duration.
    Duration,
    /// The deck the run ended with.
    EndingDeck,
    /// The inventory the run ended with.
    EndingInventory,
    /// The displayed score total.
    ScoreTotal,
    /// The score components that reconcile to the displayed total.
    ScoreComponents,
    /// Combat statistics for the run.
    RunStatistics,
}

/// Availability of one result field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultFieldStatus {
    /// A value was reported.
    Present,
    /// The host reports the field as absent for this run.
    Absent,
    /// The supported build does not carry the field.
    Unsupported,
    /// The field exists but is withheld at this scope.
    Withheld,
}

impl RunResultFieldStatus {
    /// Returns whether a value may be published for this status.
    #[must_use]
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }

    /// Returns whether the status states a reason a field carries no value.
    #[must_use]
    pub const fn is_stated(self) -> bool {
        !self.is_present()
    }
}
