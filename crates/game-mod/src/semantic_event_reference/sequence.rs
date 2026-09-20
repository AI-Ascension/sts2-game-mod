// SPDX-License-Identifier: MIT

//! Monotonic sequence positions inside one run, branch, episode and epoch.

use super::model::SemanticEventScope;

/// One event's position: the scope it belongs to and its number inside that scope.
///
/// Ordering only means something inside one scope. Two positions from different runs, branches,
/// episodes or epochs describe unrelated histories, so [`Self::precedes`] reports `false` rather
/// than comparing two numbers that are not part of the same order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventSequence {
    /// The scope this number is monotonic inside.
    pub scope: SemanticEventScope,
    /// Ordinal of the event inside that scope.
    pub sequence: u64,
}

impl SemanticEventSequence {
    /// Builds one position.
    #[must_use]
    pub fn new(scope: SemanticEventScope, sequence: u64) -> Self {
        Self { scope, sequence }
    }

    /// Returns whether two positions belong to the same run, branch, episode and epoch.
    #[must_use]
    pub fn shares_scope(&self, other: &Self) -> bool {
        self.scope == other.scope
    }

    /// Returns whether this position strictly precedes another inside the same scope.
    #[must_use]
    pub fn precedes(&self, other: &Self) -> bool {
        self.shares_scope(other) && self.sequence < other.sequence
    }
}
