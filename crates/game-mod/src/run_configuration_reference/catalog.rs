// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::{ContentCursorBinding, ContentManifest};

use super::{
    binding::{
        RunConfigurationBinding, RunConfigurationCacheKey, RunConfigurationLiveBinding, RunProfile,
        configuration_fingerprint,
    },
    definition::{
        RunConfigurationCatalog, RunConfigurationCompleteness, RunConfigurationDefinition,
        RunConfigurationDefinitionReference,
    },
    error::{RunConfigurationError, RunConfigurationSourceError, map_source_error},
    field::RunText,
    model::{
        RUN_CONFIGURATION_ENTITY_KIND, RUN_CONFIGURATION_MAX_RECORDS,
        RUN_CONFIGURATION_PRODUCER_VERSION, RunConfigurationFamilyCoverage,
        RunConfigurationFamilyState, RunConfigurationRecordInput, RunFieldKind, RunModifier,
        RunModifierInput, RunSeedPolicy,
    },
    validation::validate_record,
    value::{RunValue, active_modifier_ids, altered_kinds, canonical_token},
};

/// Bounded source snapshot used to construct one immutable run-configuration catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale for localized labels.
    pub locale: String,
    /// Profile the run was admitted under.
    pub profile: RunProfile,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the run-configuration family.
    pub family: RunConfigurationFamilyCoverage,
    /// Typed source-owned run configuration records.
    pub records: Vec<RunConfigurationRecordInput>,
}

/// Owner-local source boundary for copied run-configuration records.
///
/// The trait is deliberately read-only: no method here can change a mode, difficulty, modifier,
/// act sequence, content set, or profile, and none can admit a run.
pub trait RunConfigurationCatalogSource {
    /// Copies bounded run-configuration records without altering any run state.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<RunConfigurationCatalogSnapshot, RunConfigurationSourceError>;
}

/// Producer that binds run-configuration records to one manifest, profile, and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RunConfigurationCatalogProducer;

impl RunConfigurationCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: RunConfigurationCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<RunConfigurationCatalog, RunConfigurationError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == RUN_CONFIGURATION_ENTITY_KIND)
        {
            return Err(RunConfigurationError::MissingFamily);
        }
        let manifest_ids = manifest_record_ids(manifest);
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = RunConfigurationBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            profile: snapshot.profile,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != RunConfigurationFamilyState::Handled {
            if let Some(first) = snapshot.records.first() {
                return Err(RunConfigurationError::UnknownDefinition(
                    first.run_id.clone(),
                ));
            }
            return Ok(RunConfigurationCatalog::from_parts(
                binding,
                family,
                BTreeMap::new(),
            ));
        }
        let records = collect_records(snapshot.records)?;
        if records.len() != manifest_ids.len() {
            return Err(RunConfigurationError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|run_id| !manifest_ids.contains(*run_id))
        {
            return Err(RunConfigurationError::UnknownDefinition(unknown.clone()));
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(RunConfigurationCatalog::from_parts(
            binding,
            family,
            definitions,
        ))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &RunConfigurationCatalogSnapshot,
) -> Result<(), RunConfigurationError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(RunConfigurationError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(RunConfigurationError::LocaleMismatch);
    }
    if snapshot.producer_version != RUN_CONFIGURATION_PRODUCER_VERSION {
        return Err(RunConfigurationError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != RUN_CONFIGURATION_ENTITY_KIND {
        return Err(RunConfigurationError::FamilyIdentityMismatch);
    }
    if snapshot.records.len() > RUN_CONFIGURATION_MAX_RECORDS {
        return Err(RunConfigurationError::InvalidInput("records"));
    }
    Ok(())
}

fn manifest_record_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == RUN_CONFIGURATION_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &RunConfigurationFamilyCoverage,
    count: usize,
) -> Result<RunConfigurationFamilyCoverage, RunConfigurationError> {
    if family.definition_count != count {
        return Err(RunConfigurationError::FamilyCountMismatch);
    }
    Ok(RunConfigurationFamilyCoverage {
        entity_kind: RUN_CONFIGURATION_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<RunConfigurationRecordInput>,
) -> Result<BTreeMap<String, RunConfigurationRecordInput>, RunConfigurationError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_record(&input)?;
        let run_id = input.run_id.clone();
        if records.insert(run_id.clone(), input).is_some() {
            return Err(RunConfigurationError::DuplicateDefinition(run_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &RunConfigurationBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, RunConfigurationRecordInput>,
) -> Result<BTreeMap<String, RunConfigurationDefinition>, RunConfigurationError> {
    let mut definitions = BTreeMap::new();
    for run_id in manifest_ids {
        let Some(input) = records.get(run_id) else {
            return Err(RunConfigurationError::MissingDefinition(run_id.clone()));
        };
        definitions.insert(run_id.clone(), bind_definition(binding, input));
    }
    Ok(definitions)
}

fn bind_definition(
    binding: &RunConfigurationBinding,
    input: &RunConfigurationRecordInput,
) -> RunConfigurationDefinition {
    let fields = input
        .fields
        .iter()
        .map(|field| (field.kind, field.clone()))
        .collect::<BTreeMap<_, _>>();
    let missing = RunFieldKind::ALL
        .into_iter()
        .filter(|kind| {
            kind.is_required()
                && fields
                    .get(kind)
                    .is_none_or(|field| !field.settled.is_available())
        })
        .collect::<Vec<_>>();
    let completeness = if missing.is_empty() {
        RunConfigurationCompleteness::Complete
    } else {
        RunConfigurationCompleteness::Partial { missing }
    };
    let seed = match input.seed_policy {
        RunSeedPolicy::Visible => fields
            .get(&RunFieldKind::Seed)
            .and_then(|field| field.settled.value())
            .and_then(|value| match value {
                RunValue::Seed(seed) => Some(seed.as_str()),
                _ => None,
            }),
        RunSeedPolicy::Withheld | RunSeedPolicy::Unknown => None,
    };
    let revision = input.revision;
    let cache = RunConfigurationCacheKey::new(
        revision,
        configuration_fingerprint(&fingerprint_parts(input, seed, false)),
        false,
    );
    let seed_blind_cache = RunConfigurationCacheKey::new(
        revision,
        configuration_fingerprint(&fingerprint_parts(input, None, true)),
        true,
    );
    RunConfigurationDefinition {
        reference: RunConfigurationDefinitionReference {
            catalog: binding.clone(),
            run_id: input.run_id.clone(),
            revision,
        },
        binding: binding.clone(),
        live: RunConfigurationLiveBinding {
            run_id: input.run_id.clone(),
            instance_id: input.instance_id.clone(),
            epoch: input.epoch,
            revision,
        },
        seed_policy: input.seed_policy,
        fields,
        modifiers: input
            .modifiers
            .iter()
            .map(bind_modifier)
            .collect::<Vec<_>>(),
        completeness,
        cache,
        seed_blind_cache,
    }
}

fn bind_modifier(modifier: &RunModifierInput) -> RunModifier {
    RunModifier {
        modifier_id: modifier.modifier_id.clone(),
        label: RunText::from_validated(modifier.label.clone()),
        state: modifier.state,
        alters: modifier.alters.clone(),
    }
}

fn fingerprint_parts(
    input: &RunConfigurationRecordInput,
    seed: Option<&str>,
    seed_blind: bool,
) -> Vec<String> {
    let mut parts = Vec::new();
    for kind in RunFieldKind::ALL {
        if seed_blind && kind == RunFieldKind::Seed {
            continue;
        }
        let Some(field) = input.fields.iter().find(|field| field.kind == kind) else {
            continue;
        };
        let token = field
            .settled
            .value()
            .map_or_else(|| "unavailable".to_owned(), canonical_token);
        parts.push(format!("{}={token}", kind.as_str()));
    }
    parts.push(format!(
        "modifiers={}",
        active_modifier_ids(&input.modifiers).join(",")
    ));
    for kind in altered_kinds(&input.modifiers) {
        parts.push(format!("altered={}", kind.as_str()));
    }
    parts.push(format!("seed={}", seed.unwrap_or("withheld")));
    parts.push(format!(
        "seed_policy={}",
        seed_policy_token(input.seed_policy)
    ));
    parts
}

fn seed_policy_token(policy: RunSeedPolicy) -> &'static str {
    match policy {
        RunSeedPolicy::Visible => "visible",
        RunSeedPolicy::Withheld => "withheld",
        RunSeedPolicy::Unknown => "unknown",
    }
}
