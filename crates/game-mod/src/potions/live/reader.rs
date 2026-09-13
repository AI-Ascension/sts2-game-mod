// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    PotionInstance, PotionInstanceReference, PotionLiveSnapshot, PotionLiveSource, PotionOffer,
};
use crate::potions::{
    catalog_reader::PotionCatalog,
    error::{PotionLiveError, map_live_source_error},
    live_model::{PotionInstanceInput, PotionOfferInput, PotionOfferKind, PotionSlotState},
    live_sizes::instance_bytes,
    live_validation::validate_against_catalog,
    model::{
        POTION_PRODUCER_VERSION, PotionDefinitionReference, PotionLiveBinding,
        PotionVisibilityScope,
    },
};

/// Reader enforcing catalog, run, snapshot, epoch, definition, slot, and instance fences.
#[derive(Clone, Debug)]
pub struct PotionLiveReader {
    catalog: PotionCatalog,
    snapshot: PotionLiveSnapshot,
    scope: PotionVisibilityScope,
}

impl PotionLiveReader {
    /// Reads and validates one source snapshot under an expected identity fence.
    pub fn from_source<S: PotionLiveSource>(
        catalog: &PotionCatalog,
        expected: &PotionLiveBinding,
        source: &S,
    ) -> Result<Self, PotionLiveError> {
        Self::from_source_with_scope(catalog, expected, source, PotionVisibilityScope::Public)
    }

    /// Reads one source snapshot with an explicit visibility authorization.
    pub fn from_source_with_scope<S: PotionLiveSource>(
        catalog: &PotionCatalog,
        expected: &PotionLiveBinding,
        source: &S,
        scope: PotionVisibilityScope,
    ) -> Result<Self, PotionLiveError> {
        let input = source.read_live(expected).map_err(map_live_source_error)?;
        if input.binding != *expected {
            return Err(PotionLiveError::StaleReference);
        }
        Self::new_with_scope(catalog, PotionLiveSnapshot::from_input(input)?, scope)
    }

    /// Joins a live snapshot to the exact static catalog it references.
    pub fn new(
        catalog: &PotionCatalog,
        snapshot: PotionLiveSnapshot,
    ) -> Result<Self, PotionLiveError> {
        Self::new_with_scope(catalog, snapshot, PotionVisibilityScope::Public)
    }

    /// Joins a live snapshot with an explicit visibility authorization.
    pub fn new_with_scope(
        catalog: &PotionCatalog,
        snapshot: PotionLiveSnapshot,
        scope: PotionVisibilityScope,
    ) -> Result<Self, PotionLiveError> {
        if snapshot.binding.catalog != *catalog.binding() {
            return Err(PotionLiveError::CatalogMismatch);
        }
        if snapshot.binding.catalog.producer_version != POTION_PRODUCER_VERSION {
            return Err(PotionLiveError::ProducerVersionMismatch);
        }
        for instance in snapshot.instances.values() {
            validate_against_catalog(catalog, instance, scope)?;
        }
        Ok(Self {
            catalog: catalog.clone(),
            snapshot,
            scope,
        })
    }

    /// Returns the current coherent live binding.
    #[must_use]
    pub fn binding(&self) -> &PotionLiveBinding {
        self.snapshot.binding()
    }

    /// Returns the authorization scope used for live values.
    #[must_use]
    pub fn scope(&self) -> PotionVisibilityScope {
        self.scope
    }

    /// Returns the inventory projection.
    #[must_use]
    pub fn inventory(&self) -> &super::PotionInventory {
        self.snapshot.inventory()
    }

    /// Returns reward/shop offers joined to the current snapshot fence.
    #[must_use]
    pub fn offers(&self) -> Vec<PotionOffer> {
        self.snapshot
            .offers
            .iter()
            .map(|offer| PotionOffer {
                offer_id: offer.offer_id.clone(),
                kind: offer.kind,
                slot: offer.slot.clone(),
                instance_id: offer.instance_id.clone(),
                price: offer.price.clone(),
            })
            .collect()
    }

    /// Returns reward offers in source order.
    #[must_use]
    pub fn reward_offers(&self) -> Vec<PotionOffer> {
        self.offers()
            .into_iter()
            .filter(|offer| offer.kind == PotionOfferKind::Reward)
            .collect()
    }

    /// Returns shop offers in source order.
    #[must_use]
    pub fn shop_offers(&self) -> Vec<PotionOffer> {
        self.offers()
            .into_iter()
            .filter(|offer| offer.kind == PotionOfferKind::Shop)
            .collect()
    }

    /// Replaces the coherent snapshot only with the same instance/run and a newer epoch.
    pub fn replace_snapshot(
        &mut self,
        snapshot: PotionLiveSnapshot,
    ) -> Result<(), PotionLiveError> {
        if snapshot.binding.catalog != *self.catalog.binding() {
            return Err(PotionLiveError::CatalogMismatch);
        }
        if snapshot.binding.game_instance_id != self.snapshot.binding.game_instance_id {
            return Err(PotionLiveError::InstanceMismatch);
        }
        if snapshot.binding.run_id != self.snapshot.binding.run_id {
            return Err(PotionLiveError::RunMismatch);
        }
        if snapshot.binding.epoch <= self.snapshot.binding.epoch {
            return Err(PotionLiveError::NonMonotonicEpoch {
                current: self.snapshot.binding.epoch,
                supplied: snapshot.binding.epoch,
            });
        }
        for instance in snapshot.instances.values() {
            validate_against_catalog(&self.catalog, instance, self.scope)?;
        }
        self.snapshot = snapshot;
        Ok(())
    }

    /// Returns references in deterministic live-instance identity order.
    #[must_use]
    pub fn references(&self) -> Vec<PotionInstanceReference> {
        self.snapshot
            .instances
            .values()
            .map(|instance| self.reference_for(instance))
            .collect()
    }

    /// Reads one instance without mutating the snapshot or game state.
    pub fn get(
        &self,
        reference: &PotionInstanceReference,
    ) -> Result<PotionInstance, PotionLiveError> {
        if reference.live != self.snapshot.binding {
            return Err(PotionLiveError::StaleReference);
        }
        let instance = self
            .snapshot
            .instances
            .get(&reference.instance_id)
            .ok_or(PotionLiveError::NotFound)?;
        if instance.definition_id != reference.definition_id
            || instance.owner_id != reference.owner_id
            || instance.slot != reference.slot
        {
            return Err(PotionLiveError::StaleReference);
        }
        let detail_bytes = instance_bytes(instance);
        if detail_bytes > crate::potions::model::POTION_MAX_LIVE_DETAIL_BYTES {
            return Err(PotionLiveError::DetailTooLarge {
                limit: crate::potions::model::POTION_MAX_LIVE_DETAIL_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(PotionInstance {
            reference: reference.clone(),
            definition: PotionDefinitionReference {
                catalog: self.catalog.binding().clone(),
                potion_id: instance.definition_id.clone(),
            },
            owner_id: instance.owner_id.clone(),
            slot: instance.slot.clone(),
            usability: instance.usability.clone(),
            modifiers: instance.modifiers.clone(),
            effective_parameters: instance.effective_parameters.clone(),
            permitted_targets: instance.permitted_targets.clone(),
        })
    }

    fn reference_for(&self, instance: &PotionInstanceInput) -> PotionInstanceReference {
        PotionInstanceReference {
            live: self.snapshot.binding.clone(),
            instance_id: instance.instance_id.clone(),
            definition_id: instance.definition_id.clone(),
            owner_id: instance.owner_id.clone(),
            slot: instance.slot.clone(),
        }
    }
}

pub(super) fn validate_inventory_links(
    slots: &[PotionSlotState],
    instances: &BTreeMap<String, PotionInstanceInput>,
) -> Result<(), PotionLiveError> {
    for state in slots {
        let PotionSlotState::Occupied { slot, instance_id } = state else {
            continue;
        };
        let Some(instance) = instances.get(instance_id) else {
            return Err(PotionLiveError::UnknownOfferInstance(instance_id.clone()));
        };
        if instance.slot != *slot {
            return Err(PotionLiveError::InvalidState("inventory_slot_instance"));
        }
    }
    Ok(())
}

pub(super) fn validate_offer_links(
    offers: &[PotionOfferInput],
    instances: &BTreeMap<String, PotionInstanceInput>,
) -> Result<(), PotionLiveError> {
    for offer in offers {
        let Some(instance) = instances.get(&offer.instance_id) else {
            return Err(PotionLiveError::UnknownOfferInstance(
                offer.instance_id.clone(),
            ));
        };
        if instance.slot != offer.slot {
            return Err(PotionLiveError::InvalidState("offer_slot_instance"));
        }
    }
    Ok(())
}

pub(super) fn validate_instance_slot(
    instance: &PotionInstanceInput,
    slots: &[PotionSlotState],
    offers: &[PotionOfferInput],
) -> Result<(), PotionLiveError> {
    match &instance.slot.collection {
        crate::potions::model::PotionCollectionKind::Inventory => {
            if !slots.iter().any(|state| {
                matches!(
                    state,
                    PotionSlotState::Occupied {
                        slot,
                        instance_id
                    } if slot == &instance.slot && instance_id == &instance.instance_id
                )
            }) {
                return Err(PotionLiveError::MissingSlot(instance.slot.slot_id.clone()));
            }
        }
        crate::potions::model::PotionCollectionKind::Reward(_)
        | crate::potions::model::PotionCollectionKind::Shop(_) => {
            if !offers.iter().any(|offer| {
                offer.instance_id == instance.instance_id && offer.slot == instance.slot
            }) {
                return Err(PotionLiveError::MissingSlot(instance.slot.slot_id.clone()));
            }
        }
        crate::potions::model::PotionCollectionKind::Other(_) => {}
    }
    Ok(())
}
