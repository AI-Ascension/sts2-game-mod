// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::reader::{CharacterStateCatalogReader, CharacterStateDefinitionKind};
use super::{
    catalog_validation::{
        collect_coverage, collect_entities, collect_resources, validate_definition_coverage,
    },
    definition::{
        CharacterResourceDefinition, CharacterResourceDefinitionInput,
        CharacterResourceDefinitionReference, SecondaryEntityDefinition,
        SecondaryEntityDefinitionInput, SecondaryEntityDefinitionReference,
    },
    error::{CharacterStateCatalogError, CharacterStateSourceError, map_source_error},
    model::{
        CHARACTER_STATE_PRODUCER_VERSION, CharacterMechanicCoverage, CharacterMechanicState,
        CharacterStateCatalogBinding, CharacterStateVisibilityScope,
    },
};

/// Bounded source snapshot used to construct one immutable character-state catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized labels.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for every selected character/mode.
    pub coverage: Vec<CharacterMechanicCoverage>,
    /// Typed source-owned resource definitions.
    pub resource_definitions: Vec<CharacterResourceDefinitionInput>,
    /// Typed source-owned secondary-entity definitions.
    pub secondary_entity_definitions: Vec<SecondaryEntityDefinitionInput>,
}

/// Owner-local static character-state source boundary.
pub trait CharacterStateCatalogSource {
    /// Copies bounded definitions without constructing playable objects.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<CharacterStateCatalogSnapshot, CharacterStateSourceError>;
}

/// Producer that binds character-state definitions to one immutable content manifest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CharacterStateCatalogProducer;

impl CharacterStateCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces a catalog or rejects the entire source snapshot.
    pub fn produce<S: CharacterStateCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<CharacterStateCatalog, CharacterStateCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        if snapshot.manifest != manifest.cursor_binding() {
            return Err(CharacterStateCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(CharacterStateCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != CHARACTER_STATE_PRODUCER_VERSION {
            return Err(CharacterStateCatalogError::ProducerVersionMismatch);
        }
        let coverage = collect_coverage(snapshot.coverage)?;
        let binding = CharacterStateCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        let resources = collect_resources(snapshot.resource_definitions, &binding)?;
        let entities = collect_entities(snapshot.secondary_entity_definitions, &binding)?;
        validate_definition_coverage(&coverage, &resources, &entities)?;
        Ok(CharacterStateCatalog::from_parts(
            binding, coverage, resources, entities,
        ))
    }
}

/// Immutable static character-state catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStateCatalog {
    pub(super) binding: CharacterStateCatalogBinding,
    pub(super) coverage: BTreeMap<(String, String), CharacterMechanicCoverage>,
    pub(super) resources: BTreeMap<String, CharacterResourceDefinition>,
    pub(super) entities: BTreeMap<String, SecondaryEntityDefinition>,
}

impl CharacterStateCatalog {
    fn from_parts(
        binding: CharacterStateCatalogBinding,
        coverage: BTreeMap<(String, String), CharacterMechanicCoverage>,
        resources: BTreeMap<String, CharacterResourceDefinition>,
        entities: BTreeMap<String, SecondaryEntityDefinition>,
    ) -> Self {
        Self {
            binding,
            coverage,
            resources,
            entities,
        }
    }

    /// Returns the catalog identity fence.
    #[must_use]
    pub fn binding(&self) -> &CharacterStateCatalogBinding {
        &self.binding
    }

    /// Returns explicit support coverage for one character and mode.
    pub fn coverage(
        &self,
        character_id: &str,
        mode_id: &str,
    ) -> Option<&CharacterMechanicCoverage> {
        self.coverage
            .get(&(character_id.to_owned(), mode_id.to_owned()))
    }

    /// Returns an immutable reader with an independent bounded cursor set.
    #[must_use]
    pub fn reader(&self) -> CharacterStateCatalogReader {
        CharacterStateCatalogReader::new(self.clone())
    }

    /// Performs one exact resource-definition lookup.
    pub fn resource(
        &self,
        reference: &CharacterResourceDefinitionReference,
        scope: CharacterStateVisibilityScope,
    ) -> Result<CharacterResourceDefinition, CharacterStateCatalogError> {
        self.reader().resource(reference, scope)
    }

    /// Performs one exact secondary-entity definition lookup.
    pub fn secondary_entity(
        &self,
        reference: &SecondaryEntityDefinitionReference,
        scope: CharacterStateVisibilityScope,
    ) -> Result<SecondaryEntityDefinition, CharacterStateCatalogError> {
        self.reader().secondary_entity(reference, scope)
    }

    pub(super) fn resource_definition(
        &self,
        definition_id: &str,
    ) -> Option<&CharacterResourceDefinition> {
        self.resources.get(definition_id)
    }

    pub(super) fn secondary_definition(
        &self,
        definition_id: &str,
    ) -> Option<&SecondaryEntityDefinition> {
        self.entities.get(definition_id)
    }

    pub(super) fn mechanic_state(
        &self,
        character_id: &str,
        mode_id: &str,
        kind: CharacterStateDefinitionKind,
    ) -> Result<CharacterMechanicState, CharacterStateCatalogError> {
        let coverage = self.coverage(character_id, mode_id).ok_or_else(|| {
            CharacterStateCatalogError::MissingCoverage {
                character_id: character_id.to_owned(),
                mode_id: mode_id.to_owned(),
            }
        })?;
        Ok(match kind {
            CharacterStateDefinitionKind::Resource => coverage.resources,
            CharacterStateDefinitionKind::SecondaryEntity => coverage.secondary_entities,
        })
    }
}
