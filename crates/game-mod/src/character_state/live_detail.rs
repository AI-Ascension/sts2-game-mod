// SPDX-License-Identifier: MIT

use super::{
    error::CharacterStateLiveError,
    live::CharacterStateLiveReader,
    live_measure::{entity_detail_bytes, resource_detail_bytes},
    live_model::{CharacterResourceState, CharacterSecondaryEntity},
    live_snapshot::CharacterStateLiveSnapshot,
    live_validation::validate_snapshot,
    model::{
        CHARACTER_STATE_MAX_DETAIL_BYTES, CharacterResourceReference,
        CharacterSecondaryEntityReference,
    },
};

impl CharacterStateLiveReader {
    /// Reads one resource without mutating the snapshot or source.
    pub fn get_resource(
        &self,
        reference: &CharacterResourceReference,
    ) -> Result<CharacterResourceState, CharacterStateLiveError> {
        if reference.live != self.snapshot.binding {
            return Err(CharacterStateLiveError::StaleReference);
        }
        let resource = self
            .snapshot
            .resources
            .get(&reference.instance_id)
            .ok_or(CharacterStateLiveError::NotFound)?;
        if resource.definition_id != reference.definition_id
            || resource.owner.id != reference.owner_id
        {
            return Err(CharacterStateLiveError::StaleReference);
        }
        let definition = self
            .catalog
            .resource_definition(&resource.definition_id)
            .ok_or_else(|| {
                CharacterStateLiveError::UnknownResourceDefinition(resource.definition_id.clone())
            })?;
        let detail_bytes = resource_detail_bytes(resource, definition, &self.snapshot.binding);
        if detail_bytes > CHARACTER_STATE_MAX_DETAIL_BYTES {
            return Err(CharacterStateLiveError::DetailTooLarge {
                limit: CHARACTER_STATE_MAX_DETAIL_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(CharacterResourceState {
            reference: reference.clone(),
            definition: definition.clone(),
            owner: resource.owner.clone(),
            current: resource.current.clone(),
            maximum: resource.maximum.clone(),
            slots: resource.slots.clone(),
            active: resource.active.clone(),
        })
    }

    /// Reads one secondary entity without mutating the snapshot or source.
    pub fn get_secondary_entity(
        &self,
        reference: &CharacterSecondaryEntityReference,
    ) -> Result<CharacterSecondaryEntity, CharacterStateLiveError> {
        if reference.live != self.snapshot.binding {
            return Err(CharacterStateLiveError::StaleReference);
        }
        let entity = self
            .snapshot
            .secondary_entities
            .get(&reference.instance_id)
            .ok_or(CharacterStateLiveError::NotFound)?;
        if entity.definition_id != reference.definition_id || entity.owner.id != reference.owner_id
        {
            return Err(CharacterStateLiveError::StaleReference);
        }
        let definition = self
            .catalog
            .secondary_definition(&entity.definition_id)
            .ok_or_else(|| {
                CharacterStateLiveError::UnknownSecondaryEntityDefinition(
                    entity.definition_id.clone(),
                )
            })?;
        let detail_bytes = entity_detail_bytes(entity, definition, &self.snapshot.binding);
        if detail_bytes > CHARACTER_STATE_MAX_DETAIL_BYTES {
            return Err(CharacterStateLiveError::DetailTooLarge {
                limit: CHARACTER_STATE_MAX_DETAIL_BYTES,
                actual: detail_bytes,
            });
        }
        Ok(CharacterSecondaryEntity {
            reference: reference.clone(),
            definition: definition.clone(),
            owner: entity.owner.clone(),
            controller: entity.controller.clone(),
            hp: entity.hp.clone(),
            maximum_hp: entity.maximum_hp.clone(),
            block: entity.block.clone(),
            statuses: entity.statuses.clone(),
            intent: entity.intent.clone(),
            active: entity.active.clone(),
        })
    }

    /// Replaces the snapshot only when catalog, game, run, mode identities match and epoch grows.
    pub fn replace_snapshot(
        &mut self,
        snapshot: CharacterStateLiveSnapshot,
    ) -> Result<(), CharacterStateLiveError> {
        if snapshot.binding.catalog != *self.catalog.binding() {
            return Err(CharacterStateLiveError::CatalogMismatch);
        }
        if snapshot.binding.game_instance_id != self.binding().game_instance_id {
            return Err(CharacterStateLiveError::GameInstanceMismatch);
        }
        if snapshot.binding.run_id != self.binding().run_id {
            return Err(CharacterStateLiveError::RunMismatch);
        }
        if snapshot.binding.mode_id != self.binding().mode_id {
            return Err(CharacterStateLiveError::ModeMismatch);
        }
        if snapshot.binding.epoch <= self.binding().epoch {
            return Err(CharacterStateLiveError::NonMonotonicEpoch {
                current: self.binding().epoch,
                supplied: snapshot.binding.epoch,
            });
        }
        validate_snapshot(&self.catalog, &snapshot, self.scope)?;
        self.snapshot = snapshot;
        Ok(())
    }
}
