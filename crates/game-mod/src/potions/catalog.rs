// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    catalog_reader::PotionCatalog,
    definition::{
        PotionDefinition, PotionDefinitionInput, PotionFamilyCoverage, PotionFamilyState,
    },
    definition_validation::{definition_bytes, validate_definition},
    error::{PotionCatalogError, PotionSourceError, map_source_error},
    model::{
        POTION_ENTITY_KIND, POTION_MAX_DEFINITION_BYTES, POTION_PRODUCER_VERSION,
        PotionCatalogBinding, PotionDefinitionReference,
    },
};

/// Bounded source snapshot used to construct one immutable potion catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized title, description, and labels.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the potion family.
    pub family: PotionFamilyCoverage,
    /// Typed source-owned definition records.
    pub definitions: Vec<PotionDefinitionInput>,
}

/// Owner-local static potion source boundary.
pub trait PotionCatalogSource {
    /// Copies bounded definition records without constructing playable objects.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<PotionCatalogSnapshot, PotionSourceError>;
}

/// Producer that binds potion definitions to one immutable content manifest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PotionCatalogProducer;

impl PotionCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: PotionCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<PotionCatalog, PotionCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        if snapshot.manifest != manifest.cursor_binding() {
            return Err(PotionCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(PotionCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != POTION_PRODUCER_VERSION {
            return Err(PotionCatalogError::ProducerVersionMismatch);
        }
        if snapshot.family.entity_kind != POTION_ENTITY_KIND {
            return Err(PotionCatalogError::FamilyIdentityMismatch);
        }
        let manifest_ids = manifest
            .definitions
            .iter()
            .filter(|definition| definition.entity_kind == POTION_ENTITY_KIND)
            .map(|definition| definition.namespaced_id.clone())
            .collect::<BTreeSet<_>>();
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == POTION_ENTITY_KIND)
        {
            return Err(PotionCatalogError::MissingFamily);
        }
        if snapshot.family.definition_count != manifest_ids.len() {
            return Err(PotionCatalogError::FamilyCountMismatch);
        }
        let family = PotionFamilyCoverage {
            entity_kind: POTION_ENTITY_KIND.to_owned(),
            state: snapshot.family.state,
            definition_count: manifest_ids.len(),
        };
        if snapshot.family.state != PotionFamilyState::Handled {
            if !snapshot.definitions.is_empty() {
                return Err(PotionCatalogError::UnknownDefinition(
                    snapshot
                        .definitions
                        .first()
                        .map(|definition| definition.potion_id.clone())
                        .unwrap_or_default(),
                ));
            }
            return Ok(PotionCatalog::from_parts(
                PotionCatalogBinding {
                    manifest: snapshot.manifest,
                    locale: snapshot.locale,
                    producer_version: snapshot.producer_version,
                },
                family,
                BTreeMap::new(),
            ));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(PotionCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|potion_id| !manifest_ids.contains(*potion_id))
        {
            return Err(PotionCatalogError::UnknownDefinition(unknown.clone()));
        }
        let binding = PotionCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        let mut definitions = BTreeMap::new();
        for potion_id in manifest_ids {
            let Some(input) = records.get(&potion_id) else {
                return Err(PotionCatalogError::MissingDefinition(potion_id));
            };
            let reference = PotionDefinitionReference {
                catalog: binding.clone(),
                potion_id: potion_id.clone(),
            };
            definitions.insert(
                potion_id,
                PotionDefinition {
                    reference,
                    title: input.title.clone(),
                    description: input.description.clone(),
                    rarity: input.rarity.clone(),
                    pool: input.pool.clone(),
                    origin: input.origin.clone(),
                    acquisition: input.acquisition.clone(),
                    target_mode: input.target_mode,
                    use_rule: input.use_rule.clone(),
                    effects: input.effects.clone(),
                    parameters: input.parameters.clone(),
                    references: input.references.clone(),
                },
            );
        }
        Ok(PotionCatalog::from_parts(binding, family, definitions))
    }
}

fn collect_records(
    inputs: Vec<PotionDefinitionInput>,
) -> Result<BTreeMap<String, PotionDefinitionInput>, PotionCatalogError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input).map_err(PotionCatalogError::InvalidInput)?;
        let bytes = definition_bytes(&input);
        if bytes > POTION_MAX_DEFINITION_BYTES {
            return Err(PotionCatalogError::DefinitionTooLarge {
                limit: POTION_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let potion_id = input.potion_id.clone();
        if records.insert(potion_id.clone(), input).is_some() {
            return Err(PotionCatalogError::DuplicateDefinition(potion_id));
        }
    }
    Ok(records)
}
