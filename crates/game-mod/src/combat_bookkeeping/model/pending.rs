// SPDX-License-Identifier: MIT

use super::CombatCardInstanceReference;

/// Publicly visible pending selection or effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatPending {
    /// No pending selection/effect is visible.
    None,
    /// Public card selection waiting for a player choice.
    Selection {
        /// Stable selection identity.
        selection_id: String,
        /// Owner-defined bounded selection kind.
        kind: String,
        /// Choices copied by live instance identity.
        choices: Vec<CombatCardInstanceReference>,
    },
    /// Public effect currently waiting on host resolution.
    Effect {
        /// Stable effect identity.
        effect_id: String,
        /// Owner-defined bounded effect kind.
        kind: String,
        /// Public target identities, if the source exposes them.
        target_ids: Vec<String>,
    },
}

/// Coherent host resolving status.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatResolutionState {
    /// No action is currently resolving.
    Idle,
    /// One public effect is resolving.
    Resolving { effect_id: String },
    /// Source reports a busy state without a safe effect identity.
    Busy,
    /// Source cannot classify the current state.
    Unknown,
}
