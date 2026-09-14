// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    REWARD_MAX_DEFINITION_BYTES, REWARD_MAX_DEFINITIONS, REWARD_REFERENCE_CARD_KIND,
    REWARD_REFERENCE_CURRENCY_KIND, REWARD_REFERENCE_ENTITY_KIND, REWARD_REFERENCE_POTION_KIND,
    REWARD_REFERENCE_PRODUCER_VERSION, REWARD_REFERENCE_RELIC_KIND, RewardCatalogBinding,
    RewardCatalogError, RewardFamilyCoverage, RewardFamilyState, RewardField,
    RewardOfferDefinition, RewardOfferDefinitionInput, RewardSemanticReference,
    RewardSemanticReferenceKind, RewardSourceError, RewardVisibility,
    catalog_reader::RewardCatalog, definition::definition_bytes, error::map_source_error,
    model::validate_identity, validation::validate_definition,
};

/// Bounded source snapshot used to construct one immutable reward catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized reward values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the reward family.
    pub family: RewardFamilyCoverage,
    /// Typed source-owned reward records.
    pub definitions: Vec<RewardOfferDefinitionInput>,
}

/// Owner-local source boundary for copied reward offer definitions.
pub trait RewardCatalogSource {
    /// Copies bounded reward definitions without reading a run or evaluating hidden RNG.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<RewardCatalogSnapshot, RewardSourceError>;
}

/// Producer that binds reward definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RewardCatalogProducer;

impl RewardCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: RewardCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<RewardCatalog, RewardCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let manifest_ids = manifest_reward_ids(manifest);
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == REWARD_REFERENCE_ENTITY_KIND)
        {
            return Err(RewardCatalogError::MissingFamily);
        }
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = RewardCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != RewardFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(RewardCatalogError::UnknownDefinition(
                    first.reward_id.clone(),
                ));
            }
            return Ok(RewardCatalog::from_parts(binding, family, BTreeMap::new()));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(RewardCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|reward_id| !manifest_ids.contains(*reward_id))
        {
            return Err(RewardCatalogError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(RewardCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &RewardCatalogSnapshot,
) -> Result<(), RewardCatalogError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(RewardCatalogError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(RewardCatalogError::LocaleMismatch);
    }
    if snapshot.producer_version != REWARD_REFERENCE_PRODUCER_VERSION {
        return Err(RewardCatalogError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != REWARD_REFERENCE_ENTITY_KIND {
        return Err(RewardCatalogError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > REWARD_MAX_DEFINITIONS {
        return Err(RewardCatalogError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_reward_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == REWARD_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &RewardFamilyCoverage,
    count: usize,
) -> Result<RewardFamilyCoverage, RewardCatalogError> {
    if family.definition_count != count {
        return Err(RewardCatalogError::FamilyCountMismatch);
    }
    Ok(RewardFamilyCoverage {
        entity_kind: REWARD_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<RewardOfferDefinitionInput>,
) -> Result<BTreeMap<String, RewardOfferDefinitionInput>, RewardCatalogError> {
    let reward_visibility: BTreeMap<String, RewardVisibility> = inputs
        .iter()
        .map(|input| (input.reward_id.clone(), input.visibility))
        .collect();
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_identity(&input.reward_id, "reward_id")?;
        validate_definition(&input, &reward_visibility)?;
        let bytes = definition_bytes(&input);
        if bytes > REWARD_MAX_DEFINITION_BYTES {
            return Err(RewardCatalogError::DefinitionTooLarge {
                limit: REWARD_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let reward_id = input.reward_id.clone();
        if records.insert(reward_id.clone(), input).is_some() {
            return Err(RewardCatalogError::DuplicateDefinition(reward_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &RewardCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, RewardOfferDefinitionInput>,
) -> Result<BTreeMap<String, RewardOfferDefinition>, RewardCatalogError> {
    let mut definitions = BTreeMap::new();
    for reward_id in manifest_ids {
        let Some(input) = records.get(reward_id) else {
            return Err(RewardCatalogError::MissingDefinition(reward_id.clone()));
        };
        definitions.insert(
            reward_id.clone(),
            RewardOfferDefinition::from_input(binding, input.clone()),
        );
    }
    Ok(definitions)
}

fn validate_manifest_references(
    input: &RewardOfferDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), RewardCatalogError> {
    validate_reference_list(&input.references, manifest)?;
    validate_reference_list(&input.selection.references, manifest)?;
    for item in &input.items {
        validate_reference_list(std::slice::from_ref(&item.reference), manifest)?;
    }
    for rule in &input.generation {
        if let RewardField::Available(pool) = &rule.pool {
            validate_reference_list(pool, manifest)?;
        }
        for requirement in &rule.eligibility {
            validate_reference_list(&requirement.references, manifest)?;
        }
        for modifier in &rule.modifiers {
            validate_reference_list(&modifier.references, manifest)?;
        }
        validate_reference_list(&rule.references, manifest)?;
    }
    Ok(())
}

fn validate_reference_list(
    references: &[RewardSemanticReference],
    manifest: &ContentManifest,
) -> Result<(), RewardCatalogError> {
    for reference in references {
        let Some(entity_kind) = manifest_entity_kind(reference) else {
            continue;
        };
        ensure_manifest_reference(manifest, entity_kind, &reference.id)?;
    }
    Ok(())
}

fn manifest_entity_kind(reference: &RewardSemanticReference) -> Option<&str> {
    match &reference.kind {
        RewardSemanticReferenceKind::Reward => Some(REWARD_REFERENCE_ENTITY_KIND),
        RewardSemanticReferenceKind::Card => Some(REWARD_REFERENCE_CARD_KIND),
        RewardSemanticReferenceKind::Relic => Some(REWARD_REFERENCE_RELIC_KIND),
        RewardSemanticReferenceKind::Potion => Some(REWARD_REFERENCE_POTION_KIND),
        RewardSemanticReferenceKind::Currency => Some(REWARD_REFERENCE_CURRENCY_KIND),
        RewardSemanticReferenceKind::Content { entity_kind } => Some(entity_kind.as_str()),
        RewardSemanticReferenceKind::Item
        | RewardSemanticReferenceKind::Rule
        | RewardSemanticReferenceKind::Condition
        | RewardSemanticReferenceKind::Modifier
        | RewardSemanticReferenceKind::Unknown => None,
    }
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), RewardCatalogError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(RewardCatalogError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}
