// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    catalog_reader::EnemyCatalog,
    definition::definition_bytes,
    definition::{
        EnemyDefinition, EnemyDefinitionInput, EnemyFamilyCoverage, EnemyFamilyState,
        EnemySemanticReferenceKind,
    },
    error::{EnemyCatalogError, EnemySourceError, map_source_error},
    model::{
        ENEMY_ENTITY_KIND, ENEMY_MAX_DEFINITION_BYTES, ENEMY_MAX_DEFINITIONS,
        ENEMY_PRODUCER_VERSION, EnemyCatalogBinding, EnemyField,
    },
    validation::validate_definition,
};

/// Bounded source snapshot used to construct one immutable enemy catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized enemy values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the enemy family.
    pub family: EnemyFamilyCoverage,
    /// Typed source-owned enemy records.
    pub definitions: Vec<EnemyDefinitionInput>,
}

/// Owner-local source boundary for copied enemy records.
pub trait EnemyCatalogSource {
    /// Copies bounded enemy definitions without constructing playable objects or advancing AI.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<EnemyCatalogSnapshot, EnemySourceError>;
}

/// Producer that binds enemy definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EnemyCatalogProducer;

impl EnemyCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: EnemyCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<EnemyCatalog, EnemyCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        if snapshot.manifest != manifest.cursor_binding() {
            return Err(EnemyCatalogError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(EnemyCatalogError::LocaleMismatch);
        }
        if snapshot.producer_version != ENEMY_PRODUCER_VERSION {
            return Err(EnemyCatalogError::ProducerVersionMismatch);
        }
        if snapshot.family.entity_kind != ENEMY_ENTITY_KIND {
            return Err(EnemyCatalogError::FamilyIdentityMismatch);
        }
        if snapshot.definitions.len() > ENEMY_MAX_DEFINITIONS {
            return Err(EnemyCatalogError::InvalidInput("definitions"));
        }

        let manifest_ids = manifest
            .definitions
            .iter()
            .filter(|definition| definition.entity_kind == ENEMY_ENTITY_KIND)
            .map(|definition| definition.namespaced_id.clone())
            .collect::<BTreeSet<_>>();
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == ENEMY_ENTITY_KIND)
        {
            return Err(EnemyCatalogError::MissingFamily);
        }
        if snapshot.family.definition_count != manifest_ids.len() {
            return Err(EnemyCatalogError::FamilyCountMismatch);
        }
        let family = EnemyFamilyCoverage {
            entity_kind: ENEMY_ENTITY_KIND.to_owned(),
            state: snapshot.family.state,
            definition_count: manifest_ids.len(),
        };
        let binding = EnemyCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != EnemyFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(EnemyCatalogError::UnknownDefinition(first.enemy_id.clone()));
            }
            return Ok(EnemyCatalog::from_parts(binding, family, BTreeMap::new()));
        }

        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(EnemyCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|enemy_id| !manifest_ids.contains(*enemy_id))
        {
            return Err(EnemyCatalogError::UnknownDefinition(unknown.clone()));
        }
        for (enemy_id, definition) in &records {
            validate_manifest_references(definition, manifest)?;
            validate_origin_against_manifest(enemy_id, definition, manifest)?;
        }

        let mut definitions = BTreeMap::new();
        for enemy_id in manifest_ids {
            let Some(input) = records.get(&enemy_id) else {
                return Err(EnemyCatalogError::MissingDefinition(enemy_id));
            };
            definitions.insert(
                enemy_id,
                EnemyDefinition::from_input(&binding, input.clone()),
            );
        }
        Ok(EnemyCatalog::from_parts(binding, family, definitions))
    }
}

fn collect_records(
    inputs: Vec<EnemyDefinitionInput>,
) -> Result<BTreeMap<String, EnemyDefinitionInput>, EnemyCatalogError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input)?;
        let bytes = definition_bytes(&input);
        if bytes > ENEMY_MAX_DEFINITION_BYTES {
            return Err(EnemyCatalogError::DefinitionTooLarge {
                limit: ENEMY_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let enemy_id = input.enemy_id.clone();
        if records.insert(enemy_id.clone(), input).is_some() {
            return Err(EnemyCatalogError::DuplicateDefinition(enemy_id));
        }
    }
    Ok(records)
}

fn validate_manifest_references(
    input: &EnemyDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), EnemyCatalogError> {
    if let EnemyField::Available(encounters) = &input.encounters {
        for encounter in encounters {
            ensure_manifest_reference(manifest, "encounter", &encounter.encounter_id)?;
        }
    }
    validate_reference_list(&input.references, manifest)?;
    for movement in &input.moves {
        validate_reference_list(&movement.references, manifest)?;
        for effect in &movement.effects {
            validate_reference_list(&effect.references, manifest)?;
        }
    }
    for transition in &input.transitions {
        validate_reference_list(&transition.references, manifest)?;
    }
    Ok(())
}

fn validate_reference_list(
    references: &[super::definition::EnemySemanticReference],
    manifest: &ContentManifest,
) -> Result<(), EnemyCatalogError> {
    for reference in references {
        let Some(entity_kind) = manifest_entity_kind(reference) else {
            continue;
        };
        ensure_manifest_reference(manifest, entity_kind, &reference.id)?;
    }
    Ok(())
}

fn manifest_entity_kind<'a>(
    reference: &'a super::definition::EnemySemanticReference,
) -> Option<&'a str> {
    match &reference.kind {
        EnemySemanticReferenceKind::Status => Some("power_status"),
        EnemySemanticReferenceKind::Encounter => Some("encounter"),
        EnemySemanticReferenceKind::Content { entity_kind } => Some(entity_kind.as_str()),
        EnemySemanticReferenceKind::Effect
        | EnemySemanticReferenceKind::Rule
        | EnemySemanticReferenceKind::Condition
        | EnemySemanticReferenceKind::Unknown => None,
    }
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), EnemyCatalogError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(EnemyCatalogError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}

fn validate_origin_against_manifest(
    enemy_id: &str,
    input: &EnemyDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), EnemyCatalogError> {
    let Some(manifest_definition) = manifest.definitions.iter().find(|definition| {
        definition.entity_kind == ENEMY_ENTITY_KIND && definition.namespaced_id == enemy_id
    }) else {
        return Err(EnemyCatalogError::UnknownDefinition(enemy_id.to_owned()));
    };
    if let Some(expected) = manifest_definition.origin.package_id.as_deref()
        && input.origin.package_id.as_deref() != Some(expected)
    {
        return Err(EnemyCatalogError::OriginMismatch(enemy_id.to_owned()));
    }
    if let Some(expected) = manifest_definition.origin.package_version.as_deref()
        && input.origin.package_version.as_deref() != Some(expected)
    {
        return Err(EnemyCatalogError::OriginMismatch(enemy_id.to_owned()));
    }
    for variant in &input.origin_variants {
        if let Some(package_id) = &variant.origin.package_id {
            if !manifest
                .packages
                .iter()
                .any(|package| package.package_id == *package_id)
            {
                return Err(EnemyCatalogError::UnknownOriginPackage(package_id.clone()));
            }
        }
    }
    Ok(())
}
