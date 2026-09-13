// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    definition::PotionResolvedParameter,
    error::{PotionLiveError, PotionSourceError},
    live_model::{
        PotionInstanceInput, PotionInventory, PotionLiveSnapshotInput, PotionModifier,
        PotionOfferInput, PotionOfferKind, PotionPrice, PotionTargetReference,
        PotionUsabilityResult,
    },
    live_validation::{
        validate_binding, validate_instance_identity, validate_inventory, validate_offers,
    },
    model::{
        POTION_MAX_INSTANCES, PotionDefinitionReference, PotionField, PotionLiveBinding,
        PotionOwnerId, PotionSlotReference,
    },
};

mod reader;
pub use reader::PotionLiveReader;

/// Owner-local live state source boundary.
pub trait PotionLiveSource {
    /// Copies one coherent, bounded live snapshot for the expected identity.
    fn read_live(
        &self,
        expected: &PotionLiveBinding,
    ) -> Result<PotionLiveSnapshotInput, PotionSourceError>;
}

/// Immutable coherent live snapshot retaining instances by distinct live identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionLiveSnapshot {
    binding: PotionLiveBinding,
    inventory: PotionInventory,
    offers: Vec<PotionOfferInput>,
    instances: BTreeMap<String, PotionInstanceInput>,
}

impl PotionLiveSnapshot {
    /// Validates source-owned snapshot shape without attaching a static catalog.
    pub fn from_input(input: PotionLiveSnapshotInput) -> Result<Self, PotionLiveError> {
        validate_binding(&input.binding)?;
        if input.instances.len() > POTION_MAX_INSTANCES {
            return Err(PotionLiveError::InvalidInput("instances"));
        }
        validate_inventory(&input.inventory.max_slots, &input.inventory.slots)?;
        validate_offers(&input.offers)?;
        let mut instances = BTreeMap::new();
        for instance in input.instances {
            validate_instance_identity(&instance)?;
            let instance_id = instance.instance_id.clone();
            if instances.insert(instance_id.clone(), instance).is_some() {
                return Err(PotionLiveError::DuplicateInstance(instance_id));
            }
        }
        reader::validate_inventory_links(&input.inventory.slots, &instances)?;
        reader::validate_offer_links(&input.offers, &instances)?;
        for instance in instances.values() {
            reader::validate_instance_slot(instance, &input.inventory.slots, &input.offers)?;
        }
        Ok(Self {
            binding: input.binding,
            inventory: input.inventory,
            offers: input.offers,
            instances,
        })
    }

    /// Returns the exact snapshot identity fence.
    #[must_use]
    pub fn binding(&self) -> &PotionLiveBinding {
        &self.binding
    }

    /// Returns the inventory projection with maximum and empty-slot semantics intact.
    #[must_use]
    pub fn inventory(&self) -> &PotionInventory {
        &self.inventory
    }

    /// Returns the source-owned reward/shop offers in deterministic input order.
    #[must_use]
    pub fn offers(&self) -> &[PotionOfferInput] {
        &self.offers
    }

    /// Returns the number of retained live instances.
    #[must_use]
    pub fn len(&self) -> usize {
        self.instances.len()
    }

    /// Whether the snapshot has no live instances.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.instances.is_empty()
    }
}

/// Exact live instance reference; definition, slot, and instance identities remain distinct.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PotionInstanceReference {
    /// Coherent live identity.
    pub live: PotionLiveBinding,
    /// Live instance identity.
    pub instance_id: String,
    /// Static definition identity.
    pub definition_id: String,
    /// Owner identity at the same snapshot.
    pub owner_id: PotionOwnerId,
    /// Slot occupied by this instance.
    pub slot: PotionSlotReference,
}

/// Live potion detail joined to a static definition reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionInstance {
    /// Exact live instance identity.
    pub reference: PotionInstanceReference,
    /// Static definition reference, without copying static rule values.
    pub definition: PotionDefinitionReference,
    /// Owner identity.
    pub owner_id: PotionOwnerId,
    /// Current slot, separate from instance identity.
    pub slot: PotionSlotReference,
    /// Current usability result and reason.
    pub usability: PotionField<PotionUsabilityResult>,
    /// Ordered live modifiers.
    pub modifiers: PotionField<Vec<PotionModifier>>,
    /// Effective visible parameters.
    pub effective_parameters: PotionField<Vec<PotionResolvedParameter>>,
    /// Permitted current targets; target-specific prediction is out of scope.
    pub permitted_targets: PotionField<Vec<PotionTargetReference>>,
}

/// Reward/shop offer joined to the live snapshot fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionOffer {
    /// Stable offer identity.
    pub offer_id: String,
    /// Reward or shop kind.
    pub kind: PotionOfferKind,
    /// Stable collection slot.
    pub slot: PotionSlotReference,
    /// Live instance identity shown by this offer.
    pub instance_id: String,
    /// Optional/unknown price.
    pub price: PotionField<PotionPrice>,
}
