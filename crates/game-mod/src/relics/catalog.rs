// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    catalog_reader::RelicCatalog,
    definition::{RelicDefinition, RelicDefinitionInput, RelicFamilyCoverage, RelicFamilyState},
    definition_validation::{definition_bytes, validate_definition},
    error::{RelicCatalogError, RelicSourceError, map_source_error},
    model::{
        RELIC_ENTITY_KIND, RELIC_MAX_DEFINITION_BYTES, RELIC_PRODUCER_VERSION, RelicCatalogBinding,
        RelicDefinitionReference,
    },
};

/// Bounded source snapshot used to construct one immutable relic catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized title and description values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the relic family.
    pub family: RelicFamilyCoverage,
    /// Typed source-owned definition records.
    pub definitions: Vec<RelicDefinitionInput>,
}

/// Owner-local static relic source boundary.
pub trait RelicCatalogSource {
    /// Copies bounded definition records without constructing playable objects.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<RelicCatalogSnapshot, RelicSourceError>;
}

/// Producer that binds relic definitions to one immutable content manifest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RelicCatalogProducer;

impl RelicCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: RelicCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<RelicCatalog, RelicCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        if snapshot.manifest != manifest.cursor_binding() {
            return Err(RelicCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(RelicCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != RELIC_PRODUCER_VERSION {
            return Err(RelicCatalogError::ProducerVersionMismatch);
        }
        if snapshot.family.entity_kind != RELIC_ENTITY_KIND {
            return Err(RelicCatalogError::FamilyIdentityMismatch);
        }
        let manifest_ids = manifest
            .definitions
            .iter()
            .filter(|definition| definition.entity_kind == RELIC_ENTITY_KIND)
            .map(|definition| definition.namespaced_id.clone())
            .collect::<BTreeSet<_>>();
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == RELIC_ENTITY_KIND)
        {
            return Err(RelicCatalogError::MissingFamily);
        }
        if snapshot.family.definition_count != manifest_ids.len() {
            return Err(RelicCatalogError::FamilyCountMismatch);
        }
        let family = RelicFamilyCoverage {
            entity_kind: RELIC_ENTITY_KIND.to_owned(),
            state: snapshot.family.state,
            definition_count: manifest_ids.len(),
        };
        if snapshot.family.state != RelicFamilyState::Handled {
            if !snapshot.definitions.is_empty() {
                return Err(RelicCatalogError::UnknownDefinition(
                    snapshot
                        .definitions
                        .first()
                        .map(|definition| definition.relic_id.clone())
                        .unwrap_or_default(),
                ));
            }
            return Ok(RelicCatalog::from_parts(
                RelicCatalogBinding {
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
            return Err(RelicCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|relic_id| !manifest_ids.contains(*relic_id))
        {
            return Err(RelicCatalogError::UnknownDefinition(unknown.clone()));
        }
        let binding = RelicCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        let mut definitions = BTreeMap::new();
        for relic_id in manifest_ids {
            let Some(input) = records.get(&relic_id) else {
                return Err(RelicCatalogError::MissingDefinition(relic_id));
            };
            let reference = RelicDefinitionReference {
                catalog: binding.clone(),
                relic_id: relic_id.clone(),
            };
            definitions.insert(
                relic_id,
                RelicDefinition {
                    reference,
                    title: input.title.clone(),
                    description: input.description.clone(),
                    rarity: input.rarity.clone(),
                    tier: input.tier.clone(),
                    pool: input.pool.clone(),
                    origin: input.origin.clone(),
                    acquisition: input.acquisition.clone(),
                    references: input.references.clone(),
                    variants: input.variants.clone(),
                    parameters: input.parameters.clone(),
                    counters: input.counters.clone(),
                    activation: input.activation.clone(),
                    triggers: input.triggers.clone(),
                },
            );
        }
        Ok(RelicCatalog::from_parts(binding, family, definitions))
    }
}

fn collect_records(
    inputs: Vec<RelicDefinitionInput>,
) -> Result<BTreeMap<String, RelicDefinitionInput>, RelicCatalogError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input).map_err(RelicCatalogError::InvalidInput)?;
        let bytes = definition_bytes(&input);
        if bytes > RELIC_MAX_DEFINITION_BYTES {
            return Err(RelicCatalogError::DefinitionTooLarge {
                limit: RELIC_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let relic_id = input.relic_id.clone();
        if records.insert(relic_id.clone(), input).is_some() {
            return Err(RelicCatalogError::DuplicateDefinition(relic_id));
        }
    }
    Ok(records)
}
