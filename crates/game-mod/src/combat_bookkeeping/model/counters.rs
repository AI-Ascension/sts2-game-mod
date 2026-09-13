// SPDX-License-Identifier: MIT

use super::{CombatBookkeepingError, CombatCounterKind, CombatField};
use crate::combat_bookkeeping::model::identity::validate_identity;

/// Combat/round/player-turn identity.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CombatTurnIdentity {
    /// Stable round identity.
    pub round_id: String,
    /// Stable player-turn identity.
    pub player_turn_id: String,
    /// Current turn owner.
    pub owner: CombatTurnOwner,
}

/// Current owner of a turn.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CombatTurnOwner {
    /// The player's action window.
    Player,
    /// An enemy or encounter action window.
    Enemy,
    /// Host is resolving a previous action.
    Resolving,
    /// Source could not classify the owner.
    Unknown,
}

/// Explicit counter reset boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatCounterReset {
    /// Counter resets when combat ends.
    Combat,
    /// Counter resets at the round boundary.
    Round,
    /// Counter resets at the player-turn boundary.
    Turn,
    /// Counter persists for the run.
    Run,
    /// Counter does not reset in the source lifetime.
    Never,
    /// Source did not establish a reset boundary.
    Unknown,
}

/// Provenance of a typed public counter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatCounterProvenance {
    /// Counter was supplied by semantic history, with no inferred events.
    SemanticHistory {
        history_id: String,
        event_count: u64,
    },
    /// Host supplied the counter directly.
    HostReported,
    /// Counter value was not observed.
    NotObserved,
    /// Source could not classify provenance.
    Unknown,
}

/// One typed counter with an explicit reset and provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatCounter {
    /// Named counter kind.
    pub kind: CombatCounterKind,
    /// Value or explicit availability state.
    pub value: CombatField<u64>,
    /// Reset boundary.
    pub reset: CombatCounterReset,
    /// Source provenance.
    pub provenance: CombatCounterProvenance,
}

/// Fixed named public counter inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatCounters {
    /// Cards played in this combat.
    pub cards_played: CombatCounter,
    /// Damage received in this combat.
    pub damage_taken: CombatCounter,
    /// Damage dealt in this combat.
    pub damage_dealt: CombatCounter,
    /// Enemies defeated in this combat.
    pub enemies_defeated: CombatCounter,
}

impl CombatCounters {
    /// Returns every typed counter in deterministic order.
    #[must_use]
    pub fn all(&self) -> [&CombatCounter; 4] {
        [
            &self.cards_played,
            &self.damage_taken,
            &self.damage_dealt,
            &self.enemies_defeated,
        ]
    }

    /// Returns one named counter.
    #[must_use]
    pub fn get(&self, kind: CombatCounterKind) -> &CombatCounter {
        match kind {
            CombatCounterKind::CardsPlayed => &self.cards_played,
            CombatCounterKind::DamageTaken => &self.damage_taken,
            CombatCounterKind::DamageDealt => &self.damage_dealt,
            CombatCounterKind::EnemiesDefeated => &self.enemies_defeated,
        }
    }
}

pub(crate) fn validate_turn_identity(
    field: &CombatField<CombatTurnIdentity>,
) -> Result<(), CombatBookkeepingError> {
    let Some(turn) = field.value() else {
        return Ok(());
    };
    validate_identity(&turn.round_id, "round_id")?;
    validate_identity(&turn.player_turn_id, "player_turn_id")?;
    Ok(())
}
