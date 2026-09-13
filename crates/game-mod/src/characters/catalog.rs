// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::catalog_reader::CharacterCatalog;
use super::definition::{
    CharacterDefinition, CharacterDefinitionInput, CharacterFamilyCoverage, CharacterFamilyState,
    definition_bytes,
};
use super::error::{CharacterCatalogError, CharacterSourceError, map_source_error};
use super::model::{
    CHARACTER_ENTITY_KIND, CHARACTER_MAX_DEFINITION_BYTES, CHARACTER_MAX_DEFINITIONS,
    CHARACTER_PRODUCER_VERSION, CharacterCatalogBinding, CharacterField,
};
use super::validation::validate_definition;

/// Bounded source snapshot used to construct one immutable character catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized character values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the character family.
    pub family: CharacterFamilyCoverage,
    /// Typed source-owned character records.
    pub definitions: Vec<CharacterDefinitionInput>,
}

/// Owner-local source boundary for copied character records.
pub trait CharacterCatalogSource {
    /// Copies bounded character definitions without constructing a run or mutating progress.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<CharacterCatalogSnapshot, CharacterSourceError>;
}

/// Producer that binds character definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CharacterCatalogProducer;

impl CharacterCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: CharacterCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<CharacterCatalog, CharacterCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        if snapshot.manifest != manifest.cursor_binding() {
            return Err(CharacterCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(CharacterCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != CHARACTER_PRODUCER_VERSION {
            return Err(CharacterCatalogError::ProducerVersionMismatch);
        }
        if snapshot.family.entity_kind != CHARACTER_ENTITY_KIND {
            return Err(CharacterCatalogError::FamilyIdentityMismatch);
        }
        if snapshot.definitions.len() > CHARACTER_MAX_DEFINITIONS {
            return Err(CharacterCatalogError::InvalidInput("definitions"));
        }

        let manifest_ids = manifest
            .definitions
            .iter()
            .filter(|definition| definition.entity_kind == CHARACTER_ENTITY_KIND)
            .map(|definition| definition.namespaced_id.clone())
            .collect::<BTreeSet<_>>();
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == CHARACTER_ENTITY_KIND)
        {
            return Err(CharacterCatalogError::MissingFamily);
        }
        if snapshot.family.definition_count != manifest_ids.len() {
            return Err(CharacterCatalogError::FamilyCountMismatch);
        }
        let family = CharacterFamilyCoverage {
            entity_kind: CHARACTER_ENTITY_KIND.to_owned(),
            state: snapshot.family.state,
            definition_count: manifest_ids.len(),
        };
        let binding = CharacterCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != CharacterFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(CharacterCatalogError::UnknownDefinition(
                    first.character_id.clone(),
                ));
            }
            return Ok(CharacterCatalog::from_parts(
                binding,
                family,
                BTreeMap::new(),
            ));
        }

        let mut records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(CharacterCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|character_id| !manifest_ids.contains(*character_id))
        {
            return Err(CharacterCatalogError::UnknownDefinition(unknown.clone()));
        }
        for character in records.values() {
            validate_loadout_references(character, manifest)?;
            validate_origin_against_manifest(character, manifest)?;
        }

        let mut definitions = BTreeMap::new();
        for character_id in manifest_ids {
            let Some(input) = records.remove(&character_id) else {
                return Err(CharacterCatalogError::MissingDefinition(character_id));
            };
            definitions.insert(
                character_id,
                CharacterDefinition::from_input(&binding, input),
            );
        }
        Ok(CharacterCatalog::from_parts(binding, family, definitions))
    }
}

fn collect_records(
    inputs: Vec<CharacterDefinitionInput>,
) -> Result<BTreeMap<String, CharacterDefinitionInput>, CharacterCatalogError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input)?;
        let bytes = definition_bytes(&input);
        if bytes > CHARACTER_MAX_DEFINITION_BYTES {
            return Err(CharacterCatalogError::DefinitionTooLarge {
                limit: CHARACTER_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let character_id = input.character_id.clone();
        if records.insert(character_id.clone(), input).is_some() {
            return Err(CharacterCatalogError::DuplicateDefinition(character_id));
        }
    }
    Ok(records)
}

fn validate_loadout_references(
    input: &CharacterDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), CharacterCatalogError> {
    for loadout in &input.loadouts {
        for references in [&loadout.starting_deck, &loadout.starting_relics] {
            let CharacterField::Available(references) = references else {
                continue;
            };
            for reference in references {
                let exists = manifest.definitions.iter().any(|definition| {
                    definition.entity_kind == reference.entity_kind
                        && definition.namespaced_id == reference.namespaced_id
                });
                if !exists {
                    return Err(CharacterCatalogError::UnknownLoadoutReference {
                        entity_kind: reference.entity_kind.clone(),
                        namespaced_id: reference.namespaced_id.clone(),
                    });
                }
            }
        }
    }
    Ok(())
}

fn validate_origin_against_manifest(
    input: &CharacterDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), CharacterCatalogError> {
    let Some(manifest_definition) = manifest.definitions.iter().find(|definition| {
        definition.entity_kind == CHARACTER_ENTITY_KIND
            && definition.namespaced_id == input.character_id
    }) else {
        return Err(CharacterCatalogError::UnknownDefinition(
            input.character_id.clone(),
        ));
    };
    if let Some(expected) = manifest_definition.origin.package_id.as_deref()
        && input.origin.package_id.as_deref() != Some(expected)
    {
        return Err(CharacterCatalogError::OriginMismatch(
            input.character_id.clone(),
        ));
    }
    if let Some(expected) = manifest_definition.origin.package_version.as_deref()
        && input.origin.package_version.as_deref() != Some(expected)
    {
        return Err(CharacterCatalogError::OriginMismatch(
            input.character_id.clone(),
        ));
    }
    Ok(())
}
