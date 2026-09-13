// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{CombatCardInstanceReference, CombatField};

/// Position evidence that cannot turn secret order into an invented index.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatCardPosition {
    /// Public position in an ordered zone.
    Known(u32),
    /// The source did not expose position.
    NotObserved,
    /// The zone has no meaningful position.
    NotApplicable,
}

/// One card membership in a visible zone composition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatCardMembership {
    /// Distinct live card identity.
    pub card: CombatCardInstanceReference,
    /// Public order evidence, if any.
    pub position: CombatCardPosition,
}

/// Whether a zone's card order is safe to publish.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatOrder {
    /// Source exposed a player-permitted order.
    Public,
    /// Composition is public but order is intentionally not exposed.
    Unordered,
    /// Source knows order but it is not public.
    Hidden,
    /// The zone has no meaningful order.
    NotApplicable,
    /// Source could not classify order.
    Unknown,
}

/// Whether a visible composition is complete.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatCompositionCompleteness {
    /// Every member of the zone is included.
    Complete,
    /// Only player-permitted members are included.
    Partial,
    /// The source could not establish completeness.
    Unknown,
}

/// Supported combat zones and their public semantics.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatZoneKind {
    /// Cards held by the player.
    Hand,
    /// Current draw pile.
    Draw,
    /// Cards waiting in discard.
    Discard,
    /// Cards removed from the current combat.
    Exhaust,
    /// Cards temporarily outside ordinary piles while an effect resolves.
    Limbo,
    /// Cards currently being resolved by the host.
    Resolving,
    /// Generated or otherwise temporary combat cards.
    Temporary,
}

impl CombatZoneKind {
    /// Returns the stable owner-local spelling and semantic family.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Hand => "hand",
            Self::Draw => "draw",
            Self::Discard => "discard",
            Self::Exhaust => "exhaust",
            Self::Limbo => "limbo",
            Self::Resolving => "resolving",
            Self::Temporary => "temporary",
        }
    }

    /// Returns every supported zone in deterministic order.
    #[must_use]
    pub const fn all() -> &'static [Self; 7] {
        &[
            Self::Hand,
            Self::Draw,
            Self::Discard,
            Self::Exhaust,
            Self::Limbo,
            Self::Resolving,
            Self::Temporary,
        ]
    }
}

/// Explicit inventory status for a supported zone.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CombatZoneStatus {
    /// The source supports the zone, including an explicitly empty result.
    Available,
    /// The source supports the concept but did not observe it.
    NotObserved,
    /// No extractor exists for this zone on the selected source.
    Unsupported,
}

impl CombatZoneStatus {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::NotObserved => "not_observed",
            Self::Unsupported => "unsupported",
        }
    }
}

/// Explicit source inventory for empty, unsupported, and unobserved zones.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatZoneInventory {
    /// Status keyed by the exact supported zone.
    pub statuses: BTreeMap<CombatZoneKind, CombatZoneStatus>,
}

impl CombatZoneInventory {
    /// Creates an inventory with every supported zone unobserved.
    #[must_use]
    pub fn new() -> Self {
        Self {
            statuses: CombatZoneKind::all()
                .iter()
                .copied()
                .map(|zone| (zone, CombatZoneStatus::NotObserved))
                .collect(),
        }
    }

    /// Sets the explicit status for one supported zone.
    pub fn set(&mut self, zone: CombatZoneKind, status: CombatZoneStatus) {
        self.statuses.insert(zone, status);
    }

    /// Returns the explicit status, defaulting to not observed.
    #[must_use]
    pub fn status(&self, zone: CombatZoneKind) -> CombatZoneStatus {
        self.statuses
            .get(&zone)
            .copied()
            .unwrap_or(CombatZoneStatus::NotObserved)
    }
}

impl Default for CombatZoneInventory {
    fn default() -> Self {
        Self::new()
    }
}

/// A bounded zone projection before it is adopted by a snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatZoneInput {
    /// Exact supported zone.
    pub kind: CombatZoneKind,
    /// Exact count, even when composition is not permitted.
    pub total: CombatField<u32>,
    /// Player-permitted instance membership, never an invented order.
    pub composition: CombatField<Vec<CombatCardMembership>>,
    /// Whether the returned membership is complete.
    pub completeness: CombatCompositionCompleteness,
    /// Order visibility classification.
    pub ordering: CombatOrder,
}

/// Validated owned zone projection.
pub type CombatZone = CombatZoneInput;

/// Permanent deck and in-combat total reconciliation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatDeckReconciliation {
    /// Count of permanent deck cards before temporary combat generation.
    pub permanent_deck_count: CombatField<u32>,
    /// Count of all cards currently represented by combat zones.
    pub combat_card_count: CombatField<u32>,
    /// Count of generated/temporary cards currently in combat zones.
    pub temporary_card_count: CombatField<u32>,
}
