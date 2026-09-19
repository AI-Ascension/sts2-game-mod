// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    SettingsCatalog, SettingsFamilyCoverage, SettingsFamilyState, SettingsReferenceError,
    SettingsSemanticReferenceKind, SettingsSourceError,
    definition::{SettingDefinition, SettingDefinitionInput, definition_bytes},
    error::map_source_error,
    model::{
        SETTINGS_MAX_DEFINITION_BYTES, SETTINGS_MAX_DEFINITIONS, SETTINGS_REFERENCE_ENTITY_KIND,
        SETTINGS_REFERENCE_PRODUCER_VERSION, SETTINGS_REFERENCE_RUN_KIND, SettingsCatalogBinding,
        SettingsProfile,
    },
    validation::validate_definition,
};

/// Bounded source snapshot used to construct one immutable settings catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized setting values.
    pub locale: String,
    /// Profile selected when the values were read.
    pub profile: SettingsProfile,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the settings family.
    pub family: SettingsFamilyCoverage,
    /// Typed source-owned setting records.
    pub definitions: Vec<SettingDefinitionInput>,
}

/// Owner-local source boundary for copied settings records.
///
/// The trait is deliberately read-only: no method here can store a preference, select a profile,
/// change resolution, or write any owner file.
pub trait SettingsCatalogSource {
    /// Copies bounded setting definitions without mutating any stored preference.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<SettingsCatalogSnapshot, SettingsSourceError>;
}

/// Producer that binds setting definitions to one immutable content manifest and profile.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsCatalogProducer;

impl SettingsCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: SettingsCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<SettingsCatalog, SettingsReferenceError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == SETTINGS_REFERENCE_ENTITY_KIND)
        {
            return Err(SettingsReferenceError::MissingFamily);
        }
        let manifest_ids = manifest_setting_ids(manifest);
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = SettingsCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            profile: snapshot.profile,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != SettingsFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(SettingsReferenceError::UnknownDefinition(
                    first.setting_id.clone(),
                ));
            }
            return Ok(SettingsCatalog::from_parts(
                binding,
                family,
                BTreeMap::new(),
            ));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(SettingsReferenceError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|setting_id| !manifest_ids.contains(*setting_id))
        {
            return Err(SettingsReferenceError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(SettingsCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &SettingsCatalogSnapshot,
) -> Result<(), SettingsReferenceError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(SettingsReferenceError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(SettingsReferenceError::LocaleMismatch);
    }
    if snapshot.producer_version != SETTINGS_REFERENCE_PRODUCER_VERSION {
        return Err(SettingsReferenceError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != SETTINGS_REFERENCE_ENTITY_KIND {
        return Err(SettingsReferenceError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > SETTINGS_MAX_DEFINITIONS {
        return Err(SettingsReferenceError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_setting_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == SETTINGS_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &SettingsFamilyCoverage,
    count: usize,
) -> Result<SettingsFamilyCoverage, SettingsReferenceError> {
    if family.definition_count != count {
        return Err(SettingsReferenceError::FamilyCountMismatch);
    }
    Ok(SettingsFamilyCoverage {
        entity_kind: SETTINGS_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<SettingDefinitionInput>,
) -> Result<BTreeMap<String, SettingDefinitionInput>, SettingsReferenceError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input)?;
        let bytes = definition_bytes(&input);
        if bytes > SETTINGS_MAX_DEFINITION_BYTES {
            return Err(SettingsReferenceError::DefinitionTooLarge {
                limit: SETTINGS_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let setting_id = input.setting_id.clone();
        if records.insert(setting_id.clone(), input).is_some() {
            return Err(SettingsReferenceError::DuplicateDefinition(setting_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &SettingsCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, SettingDefinitionInput>,
) -> Result<BTreeMap<String, SettingDefinition>, SettingsReferenceError> {
    let mut definitions = BTreeMap::new();
    for setting_id in manifest_ids {
        let Some(input) = records.get(setting_id) else {
            return Err(SettingsReferenceError::MissingDefinition(
                setting_id.clone(),
            ));
        };
        definitions.insert(
            setting_id.clone(),
            SettingDefinition::from_input(binding, input.clone()),
        );
    }
    Ok(definitions)
}

fn validate_manifest_references(
    input: &SettingDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), SettingsReferenceError> {
    for reference in &input.references {
        let entity_kind = match &reference.kind {
            SettingsSemanticReferenceKind::RunConfiguration => SETTINGS_REFERENCE_RUN_KIND,
            SettingsSemanticReferenceKind::Content { entity_kind } => entity_kind.as_str(),
            SettingsSemanticReferenceKind::Rule | SettingsSemanticReferenceKind::Unknown => {
                continue;
            }
        };
        ensure_manifest_reference(manifest, entity_kind, &reference.id)?;
    }
    Ok(())
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), SettingsReferenceError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(SettingsReferenceError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}
