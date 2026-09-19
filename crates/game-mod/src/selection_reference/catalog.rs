// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    candidate::SelectionCandidate,
    definition::{SelectionCatalog, SelectionDefinition, SelectionDefinitionInput},
    encoding::definition_bytes,
    error::{SelectionError, SelectionSourceError, map_source_error},
    field::SelectionFieldStatus,
    identity::validate_identity,
    model::{
        SEL_MAX_DEFINITIONS, SELECTION_REFERENCE_ENTITY_KIND, SELECTION_REFERENCE_PRODUCER_VERSION,
        SelectionCandidateReference, SelectionCatalogBinding, SelectionFamilyCoverage,
        SelectionFamilyState, SelectionReference,
    },
    validation::{manifest::validate_manifest_references, target_map, validate_definition},
};

/// Bounded source snapshot used to construct one immutable selection catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized selection values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the selection family.
    pub family: SelectionFamilyCoverage,
    /// Typed source-owned selection records.
    pub definitions: Vec<SelectionDefinitionInput>,
}

/// Owner-local source boundary for copied selection definitions.
///
/// The trait is deliberately read-only: no method here can click, confirm, cancel, or advance a
/// prompt, and none can observe a live selector or admit a run.
pub trait SelectionCatalogSource {
    /// Copies bounded selection definitions without changing any card, player, or run state.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<SelectionCatalogSnapshot, SelectionSourceError>;
}

/// Producer that binds selection definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectionCatalogProducer;

impl SelectionCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: SelectionCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<SelectionCatalog, SelectionError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == SELECTION_REFERENCE_ENTITY_KIND)
        {
            return Err(SelectionError::MissingFamily);
        }
        let manifest_ids = manifest_selection_ids(manifest);
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = SelectionCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != SelectionFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(SelectionError::UnknownDefinition(
                    first.selection_id.clone(),
                ));
            }
            return Ok(SelectionCatalog::from_parts(
                binding,
                family,
                BTreeMap::new(),
            ));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(SelectionError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|selection_id| !manifest_ids.contains(*selection_id))
        {
            return Err(SelectionError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(SelectionCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &SelectionCatalogSnapshot,
) -> Result<(), SelectionError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(SelectionError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(SelectionError::LocaleMismatch);
    }
    if snapshot.producer_version != SELECTION_REFERENCE_PRODUCER_VERSION {
        return Err(SelectionError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != SELECTION_REFERENCE_ENTITY_KIND {
        return Err(SelectionError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > SEL_MAX_DEFINITIONS {
        return Err(SelectionError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_selection_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == SELECTION_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &SelectionFamilyCoverage,
    count: usize,
) -> Result<SelectionFamilyCoverage, SelectionError> {
    if family.definition_count != count {
        return Err(SelectionError::FamilyCountMismatch);
    }
    Ok(SelectionFamilyCoverage {
        entity_kind: SELECTION_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<SelectionDefinitionInput>,
) -> Result<BTreeMap<String, SelectionDefinitionInput>, SelectionError> {
    let targets = target_map(&inputs);
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_identity(&input.selection_id, "selection_id")?;
        validate_definition(&input, &targets)?;
        let bytes = definition_bytes(&input);
        if bytes > super::model::SEL_MAX_DEFINITION_BYTES {
            return Err(SelectionError::DefinitionTooLarge {
                limit: super::model::SEL_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let selection_id = input.selection_id.clone();
        if records.insert(selection_id.clone(), input).is_some() {
            return Err(SelectionError::DuplicateDefinition(selection_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &SelectionCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, SelectionDefinitionInput>,
) -> Result<BTreeMap<String, SelectionDefinition>, SelectionError> {
    let mut definitions = BTreeMap::new();
    for selection_id in manifest_ids {
        let Some(input) = records.get(selection_id) else {
            return Err(SelectionError::MissingDefinition(selection_id.clone()));
        };
        definitions.insert(selection_id.clone(), bind_definition(binding, input));
    }
    Ok(definitions)
}

fn bind_definition(
    binding: &SelectionCatalogBinding,
    input: &SelectionDefinitionInput,
) -> SelectionDefinition {
    let reference = SelectionReference {
        catalog: binding.clone(),
        selection_id: input.selection_id.clone(),
    };
    let candidates = input
        .candidates
        .iter()
        .map(|candidate| {
            let candidate_reference = SelectionCandidateReference {
                catalog: binding.clone(),
                selection_id: input.selection_id.clone(),
                candidate_id: candidate.candidate_id.clone(),
            };
            (
                candidate.candidate_id.clone(),
                SelectionCandidate::from_input(candidate_reference, candidate.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    SelectionDefinition {
        reference,
        parent: input.parent.clone(),
        kind: input.kind.clone(),
        prompt: input.prompt.clone(),
        visibility: input.visibility,
        evidence: input.evidence,
        selector_generation: input.selector_generation,
        picks: input.picks.clone(),
        ordering: input.ordering.clone(),
        duplicate: input.duplicate,
        confirmation: input.confirmation,
        cancellation: input.cancellation.clone(),
        steps: input.steps.clone(),
        next: input.next.clone(),
        observed_candidate_count: input.observed_candidates.len(),
        candidates,
        coverage: input.coverage.clone(),
        candidates_status: SelectionFieldStatus::Available,
        coverage_status: SelectionFieldStatus::Available,
        action_kind: input.action_kind.clone(),
        references: input.references.clone(),
    }
}
