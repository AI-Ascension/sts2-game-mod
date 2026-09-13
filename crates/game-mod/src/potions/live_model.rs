// SPDX-License-Identifier: MIT

use super::{
    definition::{PotionParameterValue, PotionResolvedParameter},
    model::{PotionField, PotionLiveBinding, PotionOwnerId, PotionSlotReference},
};

/// Live usability state; the reason is retained rather than inferred.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionUsabilityResult {
    /// Current legality observed by the source.
    pub state: PotionUseState,
    /// Optional owner-defined reason for unusable/unknown state.
    pub reason: Option<PotionUsabilityReason>,
}

/// Current usability classification.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionUseState {
    Usable,
    Unusable,
    Unknown,
}

/// Explicit usability reason; unknown remains unknown.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionUsabilityReason {
    CombatOnly,
    OutOfCombatOnly,
    NoTarget,
    InvalidTarget,
    Condition(String),
    SourceUnavailable,
    Unknown,
}

/// Target kind for a permitted-target reference.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionTargetKind {
    Player,
    Ally,
    Enemy,
    Unknown,
}

/// A permitted target reference copied from the current source snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionTargetReference {
    /// Stable target identity.
    pub target_id: String,
    /// Owner-defined target classification.
    pub kind: PotionTargetKind,
    /// Optional bounded display label.
    pub label: Option<String>,
}

/// Scope attached to a permanent or temporary potion modifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionModifierScope {
    Instance,
    Run,
    Combat,
    Turn,
    Room,
}

/// Explicit modifier expiry semantics.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionExpiration {
    Permanent,
    EndOfTurn,
    EndOfCombat,
    EndOfRoom,
    Condition(String),
    Unknown,
}

/// Typed modifier value; unknown does not become zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionModifierValue {
    Integer(i64),
    Boolean(bool),
    Text(String),
    Parameter(PotionParameterValue),
    Marker,
    Unknown,
}

/// One ordered modifier attached to a live potion instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionModifier {
    /// Source effect/enchantment/rule identity.
    pub source_ref: String,
    /// Host-semantic ordering; vector order remains authoritative.
    pub order: u16,
    /// Scope in which the modifier applies.
    pub scope: PotionModifierScope,
    /// Optional signed amount kept separate from the typed value.
    pub amount: Option<i64>,
    /// Typed value or explicit unknown.
    pub value: PotionModifierValue,
    /// Expiry copied from the source when known.
    pub expiration: PotionExpiration,
}

/// A slot is represented explicitly as empty or occupied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionSlotState {
    /// No live instance occupies the slot.
    Empty {
        /// Stable slot identity.
        slot: PotionSlotReference,
    },
    /// The distinct live instance currently occupying the slot.
    Occupied {
        /// Stable slot identity.
        slot: PotionSlotReference,
        /// Live instance identity, not a slot identity.
        instance_id: String,
    },
}

/// Inventory projection preserving maximum slots and empty-slot semantics.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionInventoryInput {
    /// Maximum available inventory slots, if observed.
    pub max_slots: PotionField<u16>,
    /// Explicit slot states, including empty slots.
    pub slots: Vec<PotionSlotState>,
}

impl PotionInventoryInput {
    /// Returns the number of explicitly occupied slots without treating unknown capacity as zero.
    #[must_use]
    pub fn occupied_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, PotionSlotState::Occupied { .. }))
            .count()
    }

    /// Returns the number of explicitly empty slots.
    #[must_use]
    pub fn empty_slots(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, PotionSlotState::Empty { .. }))
            .count()
    }
}

/// Immutable inventory state retained in a live snapshot.
pub type PotionInventory = PotionInventoryInput;

/// Reward or shop offer kind.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionOfferKind {
    Reward,
    Shop,
}

/// Typed price for a shop/reward offer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionPrice {
    /// Owner-defined currency identity.
    pub currency: String,
    /// Explicit amount availability.
    pub amount: PotionField<u64>,
}

/// Source-owned reward/shop offer before snapshot binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionOfferInput {
    /// Stable offer identity.
    pub offer_id: String,
    /// Reward or shop collection.
    pub kind: PotionOfferKind,
    /// Stable slot identity in that collection.
    pub slot: PotionSlotReference,
    /// Live potion instance shown by the offer.
    pub instance_id: String,
    /// Optional/unknown price.
    pub price: PotionField<PotionPrice>,
}

/// Source-owned live instance before a snapshot reference is attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionInstanceInput {
    /// Stable live instance identity, distinct from definition and slot IDs.
    pub instance_id: String,
    /// Static definition identity.
    pub definition_id: String,
    /// Owning player, selector, or offer identity.
    pub owner_id: PotionOwnerId,
    /// Current inventory/reward/shop slot.
    pub slot: PotionSlotReference,
    /// Current usable/unusable/unknown result and reason.
    pub usability: PotionField<PotionUsabilityResult>,
    /// Ordered live modifiers.
    pub modifiers: PotionField<Vec<PotionModifier>>,
    /// Effective parameters resolved for this instance.
    pub effective_parameters: PotionField<Vec<PotionResolvedParameter>>,
    /// Current permitted target references; no effect prediction is included.
    pub permitted_targets: PotionField<Vec<PotionTargetReference>>,
}

/// Coherent live read input with explicit identity fences.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionLiveSnapshotInput {
    /// Manifest/instance/run/snapshot/epoch witness for this collection.
    pub binding: PotionLiveBinding,
    /// Inventory state, including empty slots and maximum capacity.
    pub inventory: PotionInventoryInput,
    /// Reward and shop offers.
    pub offers: Vec<PotionOfferInput>,
    /// Owned instances in source order.
    pub instances: Vec<PotionInstanceInput>,
}
