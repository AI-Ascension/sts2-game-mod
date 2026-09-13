// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::{
    error::{CharacterStateLiveError, CharacterStateSourceError},
    live_model::{CharacterResourceInput, SecondaryEntityInput},
    live_shape::{validate_entity_shape, validate_resource_shape_public},
    live_validation::validate_binding,
    model::{
        CHARACTER_STATE_MAX_LIVE_RESOURCES, CHARACTER_STATE_MAX_LIVE_SECONDARY_ENTITIES,
        CharacterStateLiveBinding,
    },
};

/// Source-owned coherent live snapshot input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateLiveSnapshotInput {
    /// Manifest/game/run/mode/snapshot/epoch witness.
    pub binding: CharacterStateLiveBinding,
    /// Owned resource records in source order.
    pub resources: Vec<CharacterResourceInput>,
    /// Owned secondary-entity records in source order.
    pub secondary_entities: Vec<SecondaryEntityInput>,
}

/// Owner-local live source boundary.
pub trait CharacterStateLiveSource {
    /// Copies one coherent, bounded live snapshot for the expected identity.
    fn read_live(
        &self,
        expected: &CharacterStateLiveBinding,
    ) -> Result<CharacterStateLiveSnapshotInput, CharacterStateSourceError>;
}

/// Immutable coherent live snapshot retaining resources and entities by live identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateLiveSnapshot {
    pub(super) binding: CharacterStateLiveBinding,
    pub(super) resources: BTreeMap<String, CharacterResourceInput>,
    pub(super) secondary_entities: BTreeMap<String, SecondaryEntityInput>,
}

impl CharacterStateLiveSnapshot {
    /// Validates source-owned snapshot shape without attaching a static catalog.
    pub fn from_input(
        input: CharacterStateLiveSnapshotInput,
    ) -> Result<Self, CharacterStateLiveError> {
        validate_binding(&input.binding)?;
        if input.resources.len() > CHARACTER_STATE_MAX_LIVE_RESOURCES {
            return Err(CharacterStateLiveError::InvalidInput("resource_count"));
        }
        if input.secondary_entities.len() > CHARACTER_STATE_MAX_LIVE_SECONDARY_ENTITIES {
            return Err(CharacterStateLiveError::InvalidInput(
                "secondary_entity_count",
            ));
        }
        let mut resources = BTreeMap::new();
        let mut ids = BTreeSet::new();
        for resource in input.resources {
            validate_resource_shape_public(&resource)?;
            if !ids.insert(resource.instance_id.clone()) {
                return Err(CharacterStateLiveError::DuplicateInstance(
                    resource.instance_id,
                ));
            }
            resources.insert(resource.instance_id.clone(), resource);
        }
        let mut secondary_entities = BTreeMap::new();
        for entity in input.secondary_entities {
            validate_entity_shape(&entity)?;
            if !ids.insert(entity.instance_id.clone()) {
                return Err(CharacterStateLiveError::DuplicateInstance(
                    entity.instance_id,
                ));
            }
            secondary_entities.insert(entity.instance_id.clone(), entity);
        }
        Ok(Self {
            binding: input.binding,
            resources,
            secondary_entities,
        })
    }

    /// Returns the current coherent live binding.
    #[must_use]
    pub fn binding(&self) -> &CharacterStateLiveBinding {
        &self.binding
    }
}
