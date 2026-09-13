// SPDX-License-Identifier: MIT

use super::reader::CharacterStateDefinitionKind;
use super::{
    catalog::CharacterStateCatalog,
    error::{CharacterStateLiveError, map_live_source_error},
    live_model::{CharacterResourceInput, SecondaryEntityInput},
    live_snapshot::{CharacterStateLiveSnapshot, CharacterStateLiveSource},
    live_validation::{ensure_state_for_character, ensure_state_for_owner, validate_snapshot},
    model::{
        CharacterResourceReference, CharacterSecondaryEntityReference, CharacterStateLiveBinding,
        CharacterStateVisibilityScope,
    },
};

/// Reader enforcing catalog, game, run, mode, snapshot, epoch, and visibility fences.
#[derive(Clone, Debug)]
pub struct CharacterStateLiveReader {
    pub(super) catalog: CharacterStateCatalog,
    pub(super) snapshot: CharacterStateLiveSnapshot,
    pub(super) scope: CharacterStateVisibilityScope,
}

impl CharacterStateLiveReader {
    /// Reads and validates one source snapshot under an expected identity fence.
    pub fn from_source<S: CharacterStateLiveSource>(
        catalog: &CharacterStateCatalog,
        expected: &CharacterStateLiveBinding,
        source: &S,
    ) -> Result<Self, CharacterStateLiveError> {
        Self::from_source_with_scope(
            catalog,
            expected,
            source,
            CharacterStateVisibilityScope::Public,
        )
    }

    /// Reads one source snapshot with explicit visibility authorization.
    pub fn from_source_with_scope<S: CharacterStateLiveSource>(
        catalog: &CharacterStateCatalog,
        expected: &CharacterStateLiveBinding,
        source: &S,
        scope: CharacterStateVisibilityScope,
    ) -> Result<Self, CharacterStateLiveError> {
        let input = source.read_live(expected).map_err(map_live_source_error)?;
        if input.binding != *expected {
            return Err(CharacterStateLiveError::StaleReference);
        }
        Self::new_with_scope(
            catalog,
            CharacterStateLiveSnapshot::from_input(input)?,
            scope,
        )
    }

    /// Joins a live snapshot to the exact static catalog it references.
    pub fn new(
        catalog: &CharacterStateCatalog,
        snapshot: CharacterStateLiveSnapshot,
    ) -> Result<Self, CharacterStateLiveError> {
        Self::new_with_scope(catalog, snapshot, CharacterStateVisibilityScope::Public)
    }

    /// Joins a live snapshot with explicit visibility authorization.
    pub fn new_with_scope(
        catalog: &CharacterStateCatalog,
        snapshot: CharacterStateLiveSnapshot,
        scope: CharacterStateVisibilityScope,
    ) -> Result<Self, CharacterStateLiveError> {
        if snapshot.binding.catalog != *catalog.binding() {
            return Err(CharacterStateLiveError::CatalogMismatch);
        }
        if snapshot.binding.catalog.producer_version
            != super::model::CHARACTER_STATE_PRODUCER_VERSION
        {
            return Err(CharacterStateLiveError::ProducerVersionMismatch);
        }
        validate_snapshot(catalog, &snapshot, scope)?;
        Ok(Self {
            catalog: catalog.clone(),
            snapshot,
            scope,
        })
    }

    /// Returns the current coherent live binding.
    #[must_use]
    pub fn binding(&self) -> &CharacterStateLiveBinding {
        self.snapshot.binding()
    }

    /// Returns the requested visibility scope.
    #[must_use]
    pub const fn scope(&self) -> CharacterStateVisibilityScope {
        self.scope
    }

    /// Returns explicit support coverage for this snapshot's character and mode.
    pub fn coverage(
        &self,
        character_id: &str,
    ) -> Result<super::model::CharacterMechanicCoverage, CharacterStateLiveError> {
        self.catalog
            .coverage(character_id, &self.binding().mode_id)
            .cloned()
            .ok_or_else(|| CharacterStateLiveError::MissingCoverage {
                character_id: character_id.to_owned(),
                mode_id: self.binding().mode_id.clone(),
            })
    }

    /// Returns resource references in deterministic live-instance order.
    pub fn resource_references(
        &self,
    ) -> Result<Vec<CharacterResourceReference>, CharacterStateLiveError> {
        for resource in self.snapshot.resources.values() {
            ensure_state_for_owner(
                &self.catalog,
                &resource.owner,
                &self.snapshot.binding.mode_id,
                CharacterStateDefinitionKind::Resource,
            )?;
        }
        Ok(self
            .snapshot
            .resources
            .values()
            .map(|resource| self.resource_reference_for(resource))
            .collect())
    }

    /// Returns resource references for one character after checking explicit coverage.
    pub fn resource_references_for(
        &self,
        character_id: &str,
    ) -> Result<Vec<CharacterResourceReference>, CharacterStateLiveError> {
        ensure_state_for_character(
            &self.catalog,
            character_id,
            &self.snapshot.binding.mode_id,
            CharacterStateDefinitionKind::Resource,
        )?;
        Ok(self
            .snapshot
            .resources
            .values()
            .filter(|resource| resource.owner.character_id == character_id)
            .map(|resource| self.resource_reference_for(resource))
            .collect())
    }

    /// Returns secondary-entity references in deterministic live-instance order.
    pub fn secondary_entity_references(
        &self,
    ) -> Result<Vec<CharacterSecondaryEntityReference>, CharacterStateLiveError> {
        for entity in self.snapshot.secondary_entities.values() {
            ensure_state_for_owner(
                &self.catalog,
                &entity.owner,
                &self.snapshot.binding.mode_id,
                CharacterStateDefinitionKind::SecondaryEntity,
            )?;
        }
        Ok(self
            .snapshot
            .secondary_entities
            .values()
            .map(|entity| self.secondary_reference_for(entity))
            .collect())
    }

    /// Returns secondary-entity references for one character after checking explicit coverage.
    pub fn secondary_entity_references_for(
        &self,
        character_id: &str,
    ) -> Result<Vec<CharacterSecondaryEntityReference>, CharacterStateLiveError> {
        ensure_state_for_character(
            &self.catalog,
            character_id,
            &self.snapshot.binding.mode_id,
            CharacterStateDefinitionKind::SecondaryEntity,
        )?;
        Ok(self
            .snapshot
            .secondary_entities
            .values()
            .filter(|entity| entity.owner.character_id == character_id)
            .map(|entity| self.secondary_reference_for(entity))
            .collect())
    }

    fn resource_reference_for(
        &self,
        resource: &CharacterResourceInput,
    ) -> CharacterResourceReference {
        CharacterResourceReference {
            live: self.snapshot.binding.clone(),
            instance_id: resource.instance_id.clone(),
            definition_id: resource.definition_id.clone(),
            owner_id: resource.owner.id.clone(),
        }
    }

    fn secondary_reference_for(
        &self,
        entity: &SecondaryEntityInput,
    ) -> CharacterSecondaryEntityReference {
        CharacterSecondaryEntityReference {
            live: self.snapshot.binding.clone(),
            instance_id: entity.instance_id.clone(),
            definition_id: entity.definition_id.clone(),
            owner_id: entity.owner.id.clone(),
        }
    }
}
