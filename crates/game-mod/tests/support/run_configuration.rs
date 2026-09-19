// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    FixtureRunConfigurationSource, RUN_CONFIGURATION_ENTITY_KIND,
    RUN_CONFIGURATION_PRODUCER_VERSION, RunConfigurationCatalog, RunConfigurationCatalogProducer,
    RunConfigurationCatalogSnapshot, RunConfigurationError, RunConfigurationFamilyCoverage,
    RunConfigurationFamilyState, RunConfigurationField, RunConfigurationRecordInput,
    RunConfigurationUnavailableReason, RunDifficulty, RunFieldKind, RunFieldRecord, RunMode,
    RunModifierInput, RunModifierState, RunMutability, RunProfile, RunProvenance, RunSeedPolicy,
    RunSensitivity, RunValue, RunVisibility,
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

fn catalog_snapshot(definitions: &[(&str, &str)], kinds: Vec<String>) -> ContentCatalogSnapshot {
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for (entity_kind, _) in definitions {
        *counts.entry((*entity_kind).to_owned()).or_insert(0) += 1;
    }
    for kind in &kinds {
        counts.entry(kind.clone()).or_insert(0);
    }
    ContentCatalogSnapshot {
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
        registry_definition_counts: counts,
        definitions: definitions
            .iter()
            .map(|(entity_kind, id)| content_definition(entity_kind, id))
            .collect(),
    }
}

pub fn manifest_of(definitions: &[(&str, &str)]) -> ContentManifest {
    let mut kinds: Vec<String> = Vec::new();
    for (entity_kind, _) in definitions {
        if !kinds.iter().any(|kind| kind == entity_kind) {
            kinds.push((*entity_kind).to_owned());
        }
    }
    ContentManifestProducer::new("adapter-v1", kinds.clone())
        .expect("producer")
        .produce(&ManifestSource {
            snapshot: catalog_snapshot(definitions, kinds),
        })
        .expect("manifest")
}

pub fn manifest(run_ids: &[&str]) -> ContentManifest {
    let definitions: Vec<(&str, &str)> = run_ids
        .iter()
        .map(|run_id| (RUN_CONFIGURATION_ENTITY_KIND, *run_id))
        .collect();
    manifest_of(&definitions)
}

/// Manifest that declares the run-configuration family as handled with zero definitions.
pub fn empty_family_manifest() -> ContentManifest {
    ContentManifestProducer::new("adapter-v1", [RUN_CONFIGURATION_ENTITY_KIND.to_owned()])
        .expect("producer")
        .produce(&ManifestSource {
            snapshot: catalog_snapshot(&[], vec![RUN_CONFIGURATION_ENTITY_KIND.to_owned()]),
        })
        .expect("manifest")
}

pub fn profile(profile_id: &str) -> RunProfile {
    RunProfile {
        profile_id: profile_id.to_owned(),
        kind: sts2_game_mod::RunProfileKind::Named,
    }
}

pub fn unavailable(reason: RunConfigurationUnavailableReason) -> RunConfigurationField<RunValue> {
    RunConfigurationField::unavailable(reason)
}

pub fn field_record(
    kind: RunFieldKind,
    settled: RunConfigurationField<RunValue>,
) -> RunFieldRecord {
    RunFieldRecord {
        kind,
        requested: settled.clone(),
        settled,
        mutability: RunMutability::FixedAtStart,
        provenance: RunProvenance::SettledHost,
        sensitivity: RunSensitivity::Public,
        visibility: RunVisibility::Public,
    }
}

pub fn modified_field(
    mut record: RunFieldRecord,
    provenance: RunProvenance,
    mutability: RunMutability,
) -> RunFieldRecord {
    record.provenance = provenance;
    record.mutability = mutability;
    record
}

pub fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

pub fn required_fields(
    mode: RunMode,
    difficulty: RunDifficulty,
    character: &str,
    acts: &[&str],
) -> Vec<RunFieldRecord> {
    vec![
        field_record(
            RunFieldKind::Mode,
            RunConfigurationField::available(RunValue::Mode(mode)),
        ),
        field_record(
            RunFieldKind::Difficulty,
            RunConfigurationField::available(RunValue::Difficulty(difficulty)),
        ),
        field_record(
            RunFieldKind::Character,
            RunConfigurationField::available(RunValue::Identifier(character.to_owned())),
        ),
        field_record(
            RunFieldKind::ActSequence,
            RunConfigurationField::available(RunValue::Identifiers(ids(acts))),
        ),
        field_record(
            RunFieldKind::ActiveContent,
            RunConfigurationField::available(RunValue::Identifiers(ids(&["content:base"]))),
        ),
    ]
}

pub fn public_seed(seed: &str) -> RunFieldRecord {
    field_record(
        RunFieldKind::Seed,
        RunConfigurationField::available(RunValue::Seed(seed.to_owned())),
    )
}

/// Seed record whose value exists but must not be revealed outside the owner scope.
pub fn withheld_seed(seed: &str) -> RunFieldRecord {
    RunFieldRecord {
        kind: RunFieldKind::Seed,
        requested: RunConfigurationField::available(RunValue::Seed(seed.to_owned())),
        settled: unavailable(RunConfigurationUnavailableReason::Withheld),
        mutability: RunMutability::FixedAtStart,
        provenance: RunProvenance::SettledHost,
        sensitivity: RunSensitivity::Public,
        visibility: RunVisibility::OwnerOnly,
    }
}

pub fn modifier(
    modifier_id: &str,
    state: RunModifierState,
    alters: &[RunFieldKind],
) -> RunModifierInput {
    RunModifierInput {
        modifier_id: modifier_id.to_owned(),
        label: format!("label:{modifier_id}"),
        state,
        alters: alters.to_vec(),
    }
}

pub fn record(
    run_id: &str,
    revision: u64,
    seed_policy: RunSeedPolicy,
    fields: Vec<RunFieldRecord>,
    modifiers: Vec<RunModifierInput>,
) -> RunConfigurationRecordInput {
    RunConfigurationRecordInput {
        run_id: run_id.to_owned(),
        instance_id: format!("instance:{run_id}"),
        epoch: 1,
        revision,
        seed_policy,
        fields,
        modifiers,
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    records: Vec<RunConfigurationRecordInput>,
    profile: RunProfile,
) -> RunConfigurationCatalogSnapshot {
    RunConfigurationCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        profile,
        producer_version: RUN_CONFIGURATION_PRODUCER_VERSION.to_owned(),
        family: RunConfigurationFamilyCoverage {
            entity_kind: RUN_CONFIGURATION_ENTITY_KIND.to_owned(),
            state: RunConfigurationFamilyState::Handled,
            definition_count: records.len(),
        },
        records,
    }
}

pub fn produce(
    manifest: &ContentManifest,
    snapshot: RunConfigurationCatalogSnapshot,
) -> Result<RunConfigurationCatalog, RunConfigurationError> {
    RunConfigurationCatalogProducer::new()
        .produce(manifest, &FixtureRunConfigurationSource::new(snapshot))
}
