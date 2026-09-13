// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    catalog_reader::PowerStatusCatalog,
    definition::{
        PowerStatusDefinition, PowerStatusDefinitionInput, PowerStatusFamilyCoverage,
        PowerStatusFamilyState,
    },
    definition_validation::{definition_bytes, validate_definition},
    error::{PowerStatusCatalogError, PowerStatusSourceError, map_source_error},
    model::{
        POWER_STATUS_ENTITY_KIND, POWER_STATUS_MAX_DEFINITION_BYTES, POWER_STATUS_MAX_DEFINITIONS,
        POWER_STATUS_PRODUCER_VERSION, PowerStatusCatalogBinding, PowerStatusDefinitionReference,
    },
};

/// Bounded source snapshot used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized title and description values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the family.
    pub family: PowerStatusFamilyCoverage,
    /// Typed source-owned definitions.
    pub definitions: Vec<PowerStatusDefinitionInput>,
}

/// Owner-local static power/status source boundary.
pub trait PowerStatusCatalogSource {
    /// Copies bounded definition records without constructing playable objects.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<PowerStatusCatalogSnapshot, PowerStatusSourceError>;
}

/// Producer that binds definitions to one immutable content manifest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PowerStatusCatalogProducer;

impl PowerStatusCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces a catalog or rejects the entire source snapshot.
    pub fn produce<S: PowerStatusCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<PowerStatusCatalog, PowerStatusCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        if snapshot.manifest != manifest.cursor_binding() {
            return Err(PowerStatusCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(PowerStatusCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != POWER_STATUS_PRODUCER_VERSION {
            return Err(PowerStatusCatalogError::ProducerVersionMismatch);
        }
        if snapshot.family.entity_kind != POWER_STATUS_ENTITY_KIND {
            return Err(PowerStatusCatalogError::FamilyIdentityMismatch);
        }
        let manifest_ids = manifest
            .definitions
            .iter()
            .filter(|definition| definition.entity_kind == POWER_STATUS_ENTITY_KIND)
            .map(|definition| definition.namespaced_id.clone())
            .collect::<BTreeSet<_>>();
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == POWER_STATUS_ENTITY_KIND)
        {
            return Err(PowerStatusCatalogError::MissingFamily);
        }
        if snapshot.family.definition_count != manifest_ids.len() {
            return Err(PowerStatusCatalogError::FamilyCountMismatch);
        }
        if manifest_ids.len() > POWER_STATUS_MAX_DEFINITIONS {
            return Err(PowerStatusCatalogError::FamilyCountMismatch);
        }
        let family = PowerStatusFamilyCoverage {
            entity_kind: POWER_STATUS_ENTITY_KIND.to_owned(),
            state: snapshot.family.state,
            definition_count: manifest_ids.len(),
        };
        if snapshot.family.state != PowerStatusFamilyState::Handled {
            if !snapshot.definitions.is_empty() {
                return Err(PowerStatusCatalogError::UnknownDefinition(
                    snapshot
                        .definitions
                        .first()
                        .map(|definition| definition.definition_id.clone())
                        .unwrap_or_default(),
                ));
            }
            return Ok(PowerStatusCatalog::from_parts(
                PowerStatusCatalogBinding {
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
            return Err(PowerStatusCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|definition_id| !manifest_ids.contains(*definition_id))
        {
            return Err(PowerStatusCatalogError::UnknownDefinition(unknown.clone()));
        }
        let binding = PowerStatusCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        let mut definitions = BTreeMap::new();
        for definition_id in manifest_ids {
            let Some(input) = records.get(&definition_id) else {
                return Err(PowerStatusCatalogError::MissingDefinition(definition_id));
            };
            definitions.insert(
                definition_id.clone(),
                PowerStatusDefinition {
                    reference: PowerStatusDefinitionReference {
                        catalog: binding.clone(),
                        definition_id,
                    },
                    kind: input.kind.clone(),
                    category: input.category.clone(),
                    title: input.title.clone(),
                    description: input.description.clone(),
                    origin: input.origin.clone(),
                    visibility: input.visibility,
                    unlock_state: input.unlock_state,
                    amount: input.amount.clone(),
                    stacking: input.stacking.clone(),
                    duration: input.duration.clone(),
                    decay: input.decay.clone(),
                    references: input.references.clone(),
                },
            );
        }
        Ok(PowerStatusCatalog::from_parts(binding, family, definitions))
    }
}

fn collect_records(
    inputs: Vec<PowerStatusDefinitionInput>,
) -> Result<BTreeMap<String, PowerStatusDefinitionInput>, PowerStatusCatalogError> {
    if inputs.len() > POWER_STATUS_MAX_DEFINITIONS {
        return Err(PowerStatusCatalogError::FamilyCountMismatch);
    }
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input).map_err(PowerStatusCatalogError::InvalidInput)?;
        let bytes = definition_bytes(&input);
        if bytes > POWER_STATUS_MAX_DEFINITION_BYTES {
            return Err(PowerStatusCatalogError::DefinitionTooLarge {
                limit: POWER_STATUS_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let definition_id = input.definition_id.clone();
        if records.insert(definition_id.clone(), input).is_some() {
            return Err(PowerStatusCatalogError::DuplicateDefinition(definition_id));
        }
    }
    Ok(records)
}
