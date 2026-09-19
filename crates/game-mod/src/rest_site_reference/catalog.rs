// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    definition::{RestSiteCatalog, RestSiteDefinition, RestSiteDefinitionInput},
    encoding::definition_bytes,
    error::{RestSiteError, RestSourceError, map_source_error},
    identity::validate_identity,
    model::{
        REST_MAX_DEFINITIONS, REST_REFERENCE_ENTITY_KIND, REST_REFERENCE_PRODUCER_VERSION,
        RestCatalogBinding, RestFamilyCoverage, RestFamilyState,
    },
    option::RestOption,
    validation::{manifest::validate_manifest_references, target_map, validate_definition},
};

/// Bounded source snapshot used to construct one immutable rest-site catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestSiteCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized rest-site values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the rest-site family.
    pub family: RestFamilyCoverage,
    /// Typed source-owned rest-site records.
    pub definitions: Vec<RestSiteDefinitionInput>,
}

/// Owner-local source boundary for copied rest-site definitions.
///
/// The trait is deliberately read-only: no method here can rest, heal, smith, upgrade, transform,
/// mend, or select anything, and none can admit a run or observe a live rest site.
pub trait RestSiteCatalogSource {
    /// Copies bounded rest-site definitions without changing any card, relic, or run state.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<RestSiteCatalogSnapshot, RestSourceError>;
}

/// Producer that binds rest-site definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RestSiteCatalogProducer;

impl RestSiteCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: RestSiteCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<RestSiteCatalog, RestSiteError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == REST_REFERENCE_ENTITY_KIND)
        {
            return Err(RestSiteError::MissingFamily);
        }
        let manifest_ids = manifest_site_ids(manifest);
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = RestCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != RestFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(RestSiteError::UnknownDefinition(first.site_id.clone()));
            }
            return Ok(RestSiteCatalog::from_parts(
                binding,
                family,
                BTreeMap::new(),
            ));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(RestSiteError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|site_id| !manifest_ids.contains(*site_id))
        {
            return Err(RestSiteError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(RestSiteCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &RestSiteCatalogSnapshot,
) -> Result<(), RestSiteError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(RestSiteError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(RestSiteError::LocaleMismatch);
    }
    if snapshot.producer_version != REST_REFERENCE_PRODUCER_VERSION {
        return Err(RestSiteError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != REST_REFERENCE_ENTITY_KIND {
        return Err(RestSiteError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > REST_MAX_DEFINITIONS {
        return Err(RestSiteError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_site_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == REST_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &RestFamilyCoverage,
    count: usize,
) -> Result<RestFamilyCoverage, RestSiteError> {
    if family.definition_count != count {
        return Err(RestSiteError::FamilyCountMismatch);
    }
    Ok(RestFamilyCoverage {
        entity_kind: REST_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<RestSiteDefinitionInput>,
) -> Result<BTreeMap<String, RestSiteDefinitionInput>, RestSiteError> {
    let targets = target_map(&inputs);
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_identity(&input.site_id, "site_id")?;
        validate_definition(&input, &targets)?;
        let bytes = definition_bytes(&input);
        if bytes > super::model::REST_MAX_DEFINITION_BYTES {
            return Err(RestSiteError::DefinitionTooLarge {
                limit: super::model::REST_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let site_id = input.site_id.clone();
        if records.insert(site_id.clone(), input).is_some() {
            return Err(RestSiteError::DuplicateDefinition(site_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &RestCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, RestSiteDefinitionInput>,
) -> Result<BTreeMap<String, RestSiteDefinition>, RestSiteError> {
    let mut definitions = BTreeMap::new();
    for site_id in manifest_ids {
        let Some(input) = records.get(site_id) else {
            return Err(RestSiteError::MissingDefinition(site_id.clone()));
        };
        definitions.insert(site_id.clone(), bind_definition(binding, input));
    }
    Ok(definitions)
}

fn bind_definition(
    binding: &RestCatalogBinding,
    input: &RestSiteDefinitionInput,
) -> RestSiteDefinition {
    let site_reference = super::model::RestSiteDefinitionReference {
        catalog: binding.clone(),
        site_id: input.site_id.clone(),
    };
    let options = input
        .options
        .iter()
        .map(|option| {
            let reference = super::model::RestOptionReference {
                catalog: binding.clone(),
                site_id: input.site_id.clone(),
                option_id: option.option_id.clone(),
            };
            (
                option.option_id.clone(),
                RestOption::from_input(reference, option.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    RestSiteDefinition {
        reference: site_reference,
        label: input.label.clone(),
        description: input.description.clone(),
        kind: input.kind.clone(),
        visibility: input.visibility,
        evidence: input.evidence,
        option_set_generation: input.option_set_generation,
        observed_option_count: input.observed_options.len(),
        options,
        coverage: input.coverage.clone(),
        options_status: super::field::RestFieldStatus::Available,
        coverage_status: super::field::RestFieldStatus::Available,
        references: input.references.clone(),
    }
}
