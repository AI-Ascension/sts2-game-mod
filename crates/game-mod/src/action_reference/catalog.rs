// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    definition::{ActionCatalog, ActionDefinition, ActionDefinitionInput},
    encoding::definition_bytes,
    error::{ActionError, ActionSourceError, map_source_error},
    field::{ActionField, ActionFieldStatus},
    identity::validate_identity,
    model::{
        ACTION_MAX_DEFINITIONS, ACTION_REFERENCE_ENTITY_KIND, ACTION_REFERENCE_PRODUCER_VERSION,
        ActionCatalogBinding, ActionFamilyCoverage, ActionFamilyState, ActionPreviewReference,
        ActionReference, ActionTargetReference,
    },
    preview::ActionPreview,
    target::ActionTarget,
    validation::{manifest::validate_manifest_references, target_map, validate_definition},
};

/// Bounded source snapshot used to construct one immutable legal-action catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized action values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the action family.
    pub family: ActionFamilyCoverage,
    /// Typed source-owned legal-action records.
    pub definitions: Vec<ActionDefinitionInput>,
}

/// Owner-local source boundary for copied legal-action definitions.
///
/// The trait is deliberately read-only: no method here can dispatch an action, consume the RNG,
/// apply or undo a real action to approximate a read, or observe a live frame.
pub trait ActionCatalogSource {
    /// Copies bounded legal-action definitions without changing any state, queue, history, or RNG.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<ActionCatalogSnapshot, ActionSourceError>;
}

/// Producer that binds legal-action definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActionCatalogProducer;

impl ActionCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: ActionCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<ActionCatalog, ActionError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == ACTION_REFERENCE_ENTITY_KIND)
        {
            return Err(ActionError::MissingFamily);
        }
        let manifest_ids = manifest_action_ids(manifest);
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = ActionCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != ActionFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(ActionError::UnknownDefinition(first.action_id.clone()));
            }
            return Ok(ActionCatalog::from_parts(binding, family, BTreeMap::new()));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(ActionError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|action_id| !manifest_ids.contains(*action_id))
        {
            return Err(ActionError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(ActionCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &ActionCatalogSnapshot,
) -> Result<(), ActionError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(ActionError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(ActionError::LocaleMismatch);
    }
    if snapshot.producer_version != ACTION_REFERENCE_PRODUCER_VERSION {
        return Err(ActionError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != ACTION_REFERENCE_ENTITY_KIND {
        return Err(ActionError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > ACTION_MAX_DEFINITIONS {
        return Err(ActionError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_action_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == ACTION_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &ActionFamilyCoverage,
    count: usize,
) -> Result<ActionFamilyCoverage, ActionError> {
    if family.definition_count != count {
        return Err(ActionError::FamilyCountMismatch);
    }
    Ok(ActionFamilyCoverage {
        entity_kind: ACTION_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<ActionDefinitionInput>,
) -> Result<BTreeMap<String, ActionDefinitionInput>, ActionError> {
    let targets = target_map(&inputs);
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_identity(&input.action_id, "action_id")?;
        validate_definition(&input, &targets)?;
        let bytes = definition_bytes(&input);
        if bytes > super::model::ACTION_MAX_DEFINITION_BYTES {
            return Err(ActionError::DefinitionTooLarge {
                limit: super::model::ACTION_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let action_id = input.action_id.clone();
        if records.insert(action_id.clone(), input).is_some() {
            return Err(ActionError::DuplicateDefinition(action_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &ActionCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, ActionDefinitionInput>,
) -> Result<BTreeMap<String, ActionDefinition>, ActionError> {
    let mut definitions = BTreeMap::new();
    for action_id in manifest_ids {
        let Some(input) = records.get(action_id) else {
            return Err(ActionError::MissingDefinition(action_id.clone()));
        };
        definitions.insert(action_id.clone(), bind_definition(binding, input));
    }
    Ok(definitions)
}

fn target_reference(
    binding: &ActionCatalogBinding,
    action_id: &str,
    target_id: &str,
) -> ActionTargetReference {
    ActionTargetReference {
        catalog: binding.clone(),
        action_id: action_id.to_owned(),
        target_id: target_id.to_owned(),
    }
}

fn bind_definition(
    binding: &ActionCatalogBinding,
    input: &ActionDefinitionInput,
) -> ActionDefinition {
    let reference = ActionReference {
        catalog: binding.clone(),
        action_id: input.action_id.clone(),
    };
    let targets = input
        .targets
        .iter()
        .map(|target| {
            let reference = target_reference(binding, &input.action_id, &target.target_id);
            (
                target.target_id.clone(),
                ActionTarget::from_input(reference, target.clone()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let previews = input
        .previews
        .iter()
        .map(|preview| {
            let bound =
                ActionPreview {
                    reference: ActionPreviewReference {
                        catalog: binding.clone(),
                        action_id: input.action_id.clone(),
                        preview_id: preview.preview_id.clone(),
                    },
                    label: preview.label.clone(),
                    class: preview.class,
                    target: match &preview.target {
                        ActionField::Available(target_id) => ActionField::Available(
                            target_reference(binding, &input.action_id, target_id),
                        ),
                        ActionField::Unavailable(reason) => ActionField::Unavailable(*reason),
                    },
                    affected: preview
                        .affected
                        .iter()
                        .map(|target_id| target_reference(binding, &input.action_id, target_id))
                        .collect(),
                    changes: preview.changes.clone(),
                    statuses: preview.statuses.clone(),
                    movements: preview.movements.clone(),
                    selections: preview.selections.clone(),
                    assumptions: preview.assumptions.clone(),
                    omissions: preview.omissions.clone(),
                    provenance: preview.provenance.clone(),
                    evidence: preview.evidence,
                    references: preview.references.clone(),
                };
            (preview.preview_id.clone(), bound)
        })
        .collect::<BTreeMap<_, _>>();
    ActionDefinition {
        reference,
        parent: input.parent.clone(),
        kind: input.kind.clone(),
        label: input.label.clone(),
        visibility: input.visibility,
        evidence: input.evidence,
        instance_generation: input.instance_generation,
        eligibility: input.eligibility.clone(),
        costs: input.costs.clone(),
        restrictions: input.restrictions.clone(),
        observed_target_count: input.observed_targets.len(),
        targets,
        targets_status: ActionFieldStatus::Available,
        coverage: input.coverage.clone(),
        coverage_status: ActionFieldStatus::Available,
        preview_classes: input.preview_classes.clone(),
        previews,
        previews_status: ActionFieldStatus::Available,
        references: input.references.clone(),
    }
}
