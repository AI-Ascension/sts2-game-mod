// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{CombatBookkeepingError, CombatCounterKind};

mod counters;
mod identity;
mod pending;
mod zones;

pub(crate) use counters::validate_turn_identity;
pub use counters::*;
pub(crate) use identity::validate_identity;
pub(crate) use identity::validate_text;
pub use identity::*;
pub use pending::*;
pub use zones::*;

/// Source-owned snapshot input before validation and adoption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatSnapshotInput {
    /// Identity fence for this snapshot.
    pub binding: CombatBookkeepingBinding,
    /// Current round/player-turn identity.
    pub turn: CombatField<CombatTurnIdentity>,
    /// Supported card zones in source order.
    pub zones: Vec<CombatZoneInput>,
    /// Explicit zone inventory.
    pub zone_inventory: CombatZoneInventory,
    /// Permanent/temporary reconciliation counts.
    pub deck: CombatDeckReconciliation,
    /// Named public counters.
    pub counters: CombatCounters,
    /// Current host resolving state.
    pub resolution: CombatField<CombatResolutionState>,
    /// Pending public selection/effect.
    pub pending: CombatField<CombatPending>,
}

/// Immutable validated coherent combat snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatBookkeepingSnapshot {
    /// Identity fence shared by every nested value.
    pub binding: CombatBookkeepingBinding,
    /// Current round/player-turn identity.
    pub turn: CombatField<CombatTurnIdentity>,
    /// Supported zones keyed by kind.
    pub zones: BTreeMap<CombatZoneKind, CombatZone>,
    /// Explicit zone inventory.
    pub zone_inventory: CombatZoneInventory,
    /// Permanent/temporary reconciliation counts.
    pub deck: CombatDeckReconciliation,
    /// Named public counters.
    pub counters: CombatCounters,
    /// Host resolving state.
    pub resolution: CombatField<CombatResolutionState>,
    /// Pending public selection/effect.
    pub pending: CombatField<CombatPending>,
}

impl CombatBookkeepingSnapshot {
    /// Validates and adopts a source-owned input.
    pub fn from_input(input: CombatSnapshotInput) -> Result<Self, CombatBookkeepingError> {
        crate::combat_bookkeeping::validation::validate_snapshot(&input)?;
        let zones = input
            .zones
            .into_iter()
            .map(|zone| (zone.kind, zone))
            .collect();
        Ok(Self {
            binding: input.binding,
            turn: input.turn,
            zones,
            zone_inventory: input.zone_inventory,
            deck: input.deck,
            counters: input.counters,
            resolution: input.resolution,
            pending: input.pending,
        })
    }

    /// Returns the current zone by exact kind.
    #[must_use]
    pub fn zone(&self, kind: CombatZoneKind) -> Option<&CombatZone> {
        self.zones.get(&kind)
    }

    /// Returns one typed counter without allowing an arbitrary counter bag.
    #[must_use]
    pub fn counter(&self, kind: CombatCounterKind) -> &CombatCounter {
        self.counters.get(kind)
    }
}
