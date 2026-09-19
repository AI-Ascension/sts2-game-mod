// SPDX-License-Identifier: MIT

use std::cell::Cell;

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    RunConfigurationLink, SETTINGS_REFERENCE_ENTITY_KIND, SETTINGS_REFERENCE_PRODUCER_VERSION,
    SETTINGS_REFERENCE_RUN_KIND, SettingConstraint, SettingDefinitionInput, SettingOption,
    SettingValue, SettingValueField, SettingsCatalogBinding, SettingsCatalogProducer,
    SettingsCatalogSnapshot, SettingsCatalogSource, SettingsCategory, SettingsEvidence,
    SettingsFamilyCoverage, SettingsFamilyState, SettingsLevel, SettingsProfile,
    SettingsProfileKind, SettingsReadSeam, SettingsRestartState, SettingsSemanticReference,
    SettingsSemanticReferenceKind, SettingsSensitivity, SettingsSourceError, SettingsText,
    SettingsUnavailableReason, SettingsValueType, SettingsVisibility,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

fn content_definition(entity_kind: &str, id: &str) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: entity_kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: format!("{entity_kind}={id}"),
        localized_text: Some(id.to_owned()),
        origin: ContentOriginInput {
            package_id: Some("base:synthetic".to_owned()),
            package_version: Some("1".to_owned()),
        },
        override_chain: Vec::new(),
    }
}

pub fn manifest_source(extra: &[(&str, &str)], setting_ids: &[&str]) -> ManifestSource {
    let mut definitions = Vec::new();
    let mut kinds = vec![SETTINGS_REFERENCE_ENTITY_KIND.to_owned()];
    let mut counts = vec![(SETTINGS_REFERENCE_ENTITY_KIND.to_owned(), setting_ids.len())];
    definitions.extend(
        setting_ids
            .iter()
            .map(|id| content_definition(SETTINGS_REFERENCE_ENTITY_KIND, id)),
    );
    for (entity_kind, id) in extra {
        definitions.push(content_definition(entity_kind, id));
        let slot = counts.iter_mut().find(|(kind, _)| kind == entity_kind);
        match slot {
            Some((_, count)) => *count += 1,
            None => {
                kinds.push((*entity_kind).to_owned());
                counts.push(((*entity_kind).to_owned(), 1));
            }
        }
    }
    ManifestSource {
        snapshot: ContentCatalogSnapshot {
            generation_before: 11,
            generation_after: 11,
            game_build: "sts2-build:synthetic".to_owned(),
            locale: "en-US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:synthetic".to_owned(),
                package_version: Some("1".to_owned()),
                order: 0,
            }],
            available_entity_kinds: kinds,
            registry_definition_counts: counts.into_iter().collect(),
            definitions,
        },
    }
}

pub fn manifest(extra: &[(&str, &str)], setting_ids: &[&str]) -> ContentManifest {
    let mut definitions: Vec<(&str, &str)> = setting_ids
        .iter()
        .map(|id| (SETTINGS_REFERENCE_ENTITY_KIND, *id))
        .collect();
    definitions.extend(extra.iter().copied());
    manifest_of(&definitions)
}

/// Manifest that declares the settings family as handled with zero definitions.
pub fn empty_family_manifest() -> ContentManifest {
    ContentManifestProducer::new("adapter-v1", [SETTINGS_REFERENCE_ENTITY_KIND.to_owned()])
        .expect("producer")
        .produce(&manifest_source(&[], &[]))
        .expect("manifest")
}

pub fn manifest_of(definitions: &[(&str, &str)]) -> ContentManifest {
    let mut kinds: Vec<String> = Vec::new();
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (entity_kind, _) in definitions {
        if !kinds.iter().any(|kind| kind == entity_kind) {
            kinds.push((*entity_kind).to_owned());
        }
        *counts.entry((*entity_kind).to_owned()).or_insert(0) += 1;
    }
    let snapshot = ContentCatalogSnapshot {
        generation_before: 11,
        generation_after: 11,
        game_build: "sts2-build:synthetic".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![ContentPackageInput {
            package_id: "base:synthetic".to_owned(),
            package_version: Some("1".to_owned()),
            order: 0,
        }],
        available_entity_kinds: kinds.clone(),
        registry_definition_counts: counts,
        definitions: definitions
            .iter()
            .map(|(entity_kind, id)| content_definition(entity_kind, id))
            .collect(),
    };
    ContentManifestProducer::new("adapter-v1", kinds)
        .expect("producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

pub fn profile(profile_id: &str, kind: SettingsProfileKind) -> SettingsProfile {
    SettingsProfile {
        profile_id: profile_id.to_owned(),
        kind,
    }
}

pub fn seam(level: SettingsLevel) -> SettingsReadSeam {
    match level {
        SettingsLevel::Global => SettingsReadSeam::OwnerSettingsApi,
        SettingsLevel::Profile => SettingsReadSeam::ProfilePreferenceApi,
        SettingsLevel::Addon => SettingsReadSeam::AddonSettingsApi,
        SettingsLevel::Unknown => SettingsReadSeam::Unknown,
    }
}

pub fn absent(level: SettingsLevel) -> SettingValueField {
    SettingValueField::unavailable(
        SettingsUnavailableReason::NotObserved,
        SettingsEvidence::SourceDerived,
        seam(level),
    )
}

pub fn observed(level: SettingsLevel, value: SettingValue) -> SettingValueField {
    SettingValueField::available(value, SettingsEvidence::SourceDerived, seam(level))
}

pub fn setting(
    setting_id: &str,
    category: SettingsCategory,
    level: SettingsLevel,
    value_type: SettingsValueType,
) -> SettingDefinitionInput {
    SettingDefinitionInput {
        setting_id: setting_id.to_owned(),
        label: setting_id.to_owned(),
        description: None,
        category,
        level,
        value_type,
        sensitivity: SettingsSensitivity::Public,
        visibility: SettingsVisibility::Visible,
        restart: SettingsRestartState::NotRequired,
        run_link: RunConfigurationLink::NotRunAffecting,
        default_value: absent(level),
        stored_value: absent(level),
        effective_value: absent(level),
        constraints: Vec::new(),
        references: Vec::new(),
    }
}

pub fn with_values(
    mut input: SettingDefinitionInput,
    stored: SettingValue,
    effective: SettingValue,
) -> SettingDefinitionInput {
    let level = input.level;
    input.default_value = observed(level, stored.clone());
    input.stored_value = observed(level, stored);
    input.effective_value = observed(level, effective);
    input
}

pub fn with_options(mut input: SettingDefinitionInput, options: &[&str]) -> SettingDefinitionInput {
    input.constraints = vec![SettingConstraint::Options(
        options
            .iter()
            .map(|option| SettingOption::new(option, option).expect("option"))
            .collect(),
    )];
    input
}

pub fn with_range(
    mut input: SettingDefinitionInput,
    min: i64,
    max: i64,
    step: Option<i64>,
) -> SettingDefinitionInput {
    input.constraints = vec![SettingConstraint::Range { min, max, step }];
    input
}

pub fn with_run_link(
    mut input: SettingDefinitionInput,
    configuration_id: &str,
) -> SettingDefinitionInput {
    input.run_link = RunConfigurationLink::RunAffecting {
        configuration_id: configuration_id.to_owned(),
    };
    input.references.push(SettingsSemanticReference {
        kind: SettingsSemanticReferenceKind::RunConfiguration,
        id: configuration_id.to_owned(),
        label: SettingsText::available(configuration_id).expect("label"),
    });
    input
}

pub fn withheld(input: SettingDefinitionInput, private: bool) -> SettingDefinitionInput {
    let mut input = input;
    let level = input.level;
    if private {
        input.sensitivity = SettingsSensitivity::Private;
        input.visibility = SettingsVisibility::OwnerOnly;
    } else {
        input.visibility = SettingsVisibility::Hidden;
    }
    let reason = SettingsUnavailableReason::Withheld;
    let evidence = SettingsEvidence::Authoritative;
    input.default_value = SettingValueField::unavailable(reason, evidence, seam(level));
    input.stored_value = SettingValueField::unavailable(reason, evidence, seam(level));
    input.effective_value = SettingValueField::unavailable(reason, evidence, seam(level));
    input
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<SettingDefinitionInput>,
    profile: SettingsProfile,
) -> SettingsCatalogSnapshot {
    SettingsCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        profile,
        producer_version: SETTINGS_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: SettingsFamilyCoverage {
            entity_kind: SETTINGS_REFERENCE_ENTITY_KIND.to_owned(),
            state: SettingsFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

#[derive(Debug)]
pub struct SettingsFixtureSource {
    pub snapshot: SettingsCatalogSnapshot,
    reads: Cell<usize>,
}

impl SettingsFixtureSource {
    pub fn new(snapshot: SettingsCatalogSnapshot) -> Self {
        Self {
            snapshot,
            reads: Cell::new(0),
        }
    }

    pub fn reads(&self) -> usize {
        self.reads.get()
    }
}

impl SettingsCatalogSource for SettingsFixtureSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<SettingsCatalogSnapshot, SettingsSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Failure {
    NoActiveSource,
    AccessDenied,
    Malformed,
}

#[derive(Debug)]
pub struct FailingSource(pub Failure);

impl SettingsCatalogSource for FailingSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<SettingsCatalogSnapshot, SettingsSourceError> {
        Err(match self.0 {
            Failure::NoActiveSource => SettingsSourceError::NoActiveSource,
            Failure::AccessDenied => SettingsSourceError::AccessDenied,
            Failure::Malformed => SettingsSourceError::Malformed,
        })
    }
}

pub fn catalog_binding(
    manifest: &ContentManifest,
    profile: SettingsProfile,
) -> SettingsCatalogBinding {
    SettingsCatalogBinding {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        profile,
        producer_version: SETTINGS_REFERENCE_PRODUCER_VERSION.to_owned(),
    }
}

pub fn produce<S: SettingsCatalogSource>(
    manifest: &ContentManifest,
    source: &S,
) -> Result<sts2_game_mod::SettingsCatalog, sts2_game_mod::SettingsReferenceError> {
    SettingsCatalogProducer::new().produce(manifest, source)
}

pub fn run_kind() -> &'static str {
    SETTINGS_REFERENCE_RUN_KIND
}
