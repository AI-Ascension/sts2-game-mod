// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    ActReferenceError, ActSemanticReference, ActSemanticReferenceKind, ActSourceError,
    catalog_reader::ActCatalog,
    definition::{
        ActDefinition, ActDefinitionInput, EncounterDefinitionInput, EncounterEnemyGroup,
        MapGenerationConstraint, definition_bytes,
    },
    error::map_source_error,
    model::{
        ACT_MAX_DEFINITION_BYTES, ACT_MAX_DEFINITIONS, ACT_REFERENCE_ENCOUNTER_KIND,
        ACT_REFERENCE_ENEMY_KIND, ACT_REFERENCE_ENTITY_KIND, ACT_REFERENCE_PRODUCER_VERSION,
        ActCatalogBinding, ActFamilyCoverage, ActFamilyState,
    },
    validation::validate_definition,
};

/// Bounded source snapshot used to construct one immutable act catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized act values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the act family.
    pub family: ActFamilyCoverage,
    /// Typed source-owned act records.
    pub definitions: Vec<ActDefinitionInput>,
}

/// Owner-local source boundary for copied act, encounter, and map-generation records.
pub trait ActCatalogSource {
    /// Copies bounded act definitions without constructing maps or evaluating generation.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<ActCatalogSnapshot, ActSourceError>;
}

/// Producer that binds act definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActCatalogProducer;

impl ActCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: ActCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<ActCatalog, ActReferenceError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let manifest_ids = manifest_act_ids(manifest);
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == ACT_REFERENCE_ENTITY_KIND)
        {
            return Err(ActReferenceError::MissingFamily);
        }
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = ActCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != ActFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(ActReferenceError::UnknownDefinition(first.act_id.clone()));
            }
            return Ok(ActCatalog::from_parts(binding, family, BTreeMap::new()));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(ActReferenceError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|act_id| !manifest_ids.contains(*act_id))
        {
            return Err(ActReferenceError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(ActCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &ActCatalogSnapshot,
) -> Result<(), ActReferenceError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(ActReferenceError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(ActReferenceError::LocaleMismatch);
    }
    if snapshot.producer_version != ACT_REFERENCE_PRODUCER_VERSION {
        return Err(ActReferenceError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != ACT_REFERENCE_ENTITY_KIND {
        return Err(ActReferenceError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > ACT_MAX_DEFINITIONS {
        return Err(ActReferenceError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_act_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == ACT_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &ActFamilyCoverage,
    count: usize,
) -> Result<ActFamilyCoverage, ActReferenceError> {
    if family.definition_count != count {
        return Err(ActReferenceError::FamilyCountMismatch);
    }
    Ok(ActFamilyCoverage {
        entity_kind: ACT_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<ActDefinitionInput>,
) -> Result<BTreeMap<String, ActDefinitionInput>, ActReferenceError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input)?;
        let bytes = definition_bytes(&input);
        if bytes > ACT_MAX_DEFINITION_BYTES {
            return Err(ActReferenceError::DefinitionTooLarge {
                limit: ACT_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let act_id = input.act_id.clone();
        if records.insert(act_id.clone(), input).is_some() {
            return Err(ActReferenceError::DuplicateDefinition(act_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &ActCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, ActDefinitionInput>,
) -> Result<BTreeMap<String, ActDefinition>, ActReferenceError> {
    let mut definitions = BTreeMap::new();
    for act_id in manifest_ids {
        let Some(input) = records.get(act_id) else {
            return Err(ActReferenceError::MissingDefinition(act_id.clone()));
        };
        definitions.insert(
            act_id.clone(),
            ActDefinition::from_input(binding, input.clone()),
        );
    }
    Ok(definitions)
}

fn validate_manifest_references(
    input: &ActDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), ActReferenceError> {
    for category in &input.room_categories {
        validate_reference_list(&category.references, manifest)?;
    }
    for encounter in &input.encounters {
        validate_encounter_references(encounter, manifest)?;
    }
    if let Some(pools) = input.pools.value() {
        for pool in pools {
            validate_reference_list(&pool.references, manifest)?;
        }
    }
    if let Some(constraints) = input.constraints.value() {
        for constraint in constraints {
            validate_constraint_references(constraint, manifest)?;
        }
    }
    validate_reference_list(&input.references, manifest)
}

fn validate_encounter_references(
    encounter: &EncounterDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), ActReferenceError> {
    validate_reference_list(&encounter.references, manifest)?;
    for group in &encounter.groups {
        validate_group_references(group, manifest)?;
    }
    for condition in &encounter.eligibility {
        validate_reference_list(&condition.references, manifest)?;
    }
    Ok(())
}

fn validate_group_references(
    group: &EncounterEnemyGroup,
    manifest: &ContentManifest,
) -> Result<(), ActReferenceError> {
    validate_reference_list(&group.references, manifest)?;
    for enemy in &group.enemies {
        ensure_manifest_reference(manifest, ACT_REFERENCE_ENEMY_KIND, &enemy.enemy_id)?;
        validate_reference_list(&enemy.references, manifest)?;
    }
    Ok(())
}

fn validate_constraint_references(
    constraint: &MapGenerationConstraint,
    manifest: &ContentManifest,
) -> Result<(), ActReferenceError> {
    validate_reference_list(&constraint.references, manifest)
}

fn validate_reference_list(
    references: &[ActSemanticReference],
    manifest: &ContentManifest,
) -> Result<(), ActReferenceError> {
    for reference in references {
        let Some(entity_kind) = manifest_entity_kind(reference) else {
            continue;
        };
        ensure_manifest_reference(manifest, entity_kind, &reference.id)?;
    }
    Ok(())
}

fn manifest_entity_kind(reference: &ActSemanticReference) -> Option<&str> {
    match &reference.kind {
        ActSemanticReferenceKind::Enemy => Some(ACT_REFERENCE_ENEMY_KIND),
        ActSemanticReferenceKind::Encounter => Some(ACT_REFERENCE_ENCOUNTER_KIND),
        ActSemanticReferenceKind::Act => Some(ACT_REFERENCE_ENTITY_KIND),
        ActSemanticReferenceKind::Content { entity_kind } => Some(entity_kind.as_str()),
        ActSemanticReferenceKind::RoomCategory
        | ActSemanticReferenceKind::Effect
        | ActSemanticReferenceKind::Rule
        | ActSemanticReferenceKind::Condition
        | ActSemanticReferenceKind::Unknown => None,
    }
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), ActReferenceError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(ActReferenceError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}
