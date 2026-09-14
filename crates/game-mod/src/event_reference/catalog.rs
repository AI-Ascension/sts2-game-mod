// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::{
    EVENT_MAX_DEFINITION_BYTES, EVENT_REFERENCE_ACT_KIND, EVENT_REFERENCE_CARD_KIND,
    EVENT_REFERENCE_ENCOUNTER_KIND, EVENT_REFERENCE_ENEMY_KIND, EVENT_REFERENCE_ENTITY_KIND,
    EVENT_REFERENCE_POTION_KIND, EVENT_REFERENCE_PRODUCER_VERSION, EVENT_REFERENCE_RELIC_KIND,
    EventCatalogBinding, EventCatalogError, EventDefinition, EventDefinitionInput,
    EventFamilyCoverage, EventFamilyState, EventSemanticReference, EventSemanticReferenceKind,
    EventSourceError, EventVisibility, catalog_reader::EventCatalog, definition::definition_bytes,
    error::map_source_error, model::EVENT_MAX_DEFINITIONS, validation::validate_definition,
};

/// Bounded source snapshot used to construct one immutable event catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale for localized event values.
    pub locale: String,
    /// Exact source producer compatibility.
    pub producer_version: String,
    /// Explicit support state for the event family.
    pub family: EventFamilyCoverage,
    /// Typed source-owned event records.
    pub definitions: Vec<EventDefinitionInput>,
}

/// Owner-local source boundary for copied event definitions.
pub trait EventCatalogSource {
    /// Copies bounded event definitions without reading a run or evaluating hidden RNG.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<EventCatalogSnapshot, EventSourceError>;
}

/// Producer that binds event definitions to one immutable content manifest and locale.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EventCatalogProducer;

impl EventCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: EventCatalogSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<EventCatalog, EventCatalogError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let manifest_ids = manifest_event_ids(manifest);
        if !manifest
            .families
            .iter()
            .any(|family| family.entity_kind == EVENT_REFERENCE_ENTITY_KIND)
        {
            return Err(EventCatalogError::MissingFamily);
        }
        let family = coverage(&snapshot.family, manifest_ids.len())?;
        let binding = EventCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            producer_version: snapshot.producer_version,
        };
        if snapshot.family.state != EventFamilyState::Handled {
            if let Some(first) = snapshot.definitions.first() {
                return Err(EventCatalogError::UnknownDefinition(first.event_id.clone()));
            }
            return Ok(EventCatalog::from_parts(binding, family, BTreeMap::new()));
        }
        let records = collect_records(snapshot.definitions)?;
        if records.len() != manifest_ids.len() {
            return Err(EventCatalogError::FamilyCountMismatch);
        }
        if let Some(unknown) = records
            .keys()
            .find(|event_id| !manifest_ids.contains(*event_id))
        {
            return Err(EventCatalogError::UnknownDefinition(unknown.clone()));
        }
        for definition in records.values() {
            validate_manifest_references(definition, manifest)?;
        }
        let definitions = bind_definitions(&binding, &manifest_ids, records)?;
        Ok(EventCatalog::from_parts(binding, family, definitions))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &EventCatalogSnapshot,
) -> Result<(), EventCatalogError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(EventCatalogError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(EventCatalogError::LocaleMismatch);
    }
    if snapshot.producer_version != EVENT_REFERENCE_PRODUCER_VERSION {
        return Err(EventCatalogError::ProducerVersionMismatch);
    }
    if snapshot.family.entity_kind != EVENT_REFERENCE_ENTITY_KIND {
        return Err(EventCatalogError::FamilyIdentityMismatch);
    }
    if snapshot.definitions.len() > EVENT_MAX_DEFINITIONS {
        return Err(EventCatalogError::InvalidInput("definitions"));
    }
    Ok(())
}

fn manifest_event_ids(manifest: &ContentManifest) -> BTreeSet<String> {
    manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == EVENT_REFERENCE_ENTITY_KIND)
        .map(|definition| definition.namespaced_id.clone())
        .collect()
}

fn coverage(
    family: &EventFamilyCoverage,
    count: usize,
) -> Result<EventFamilyCoverage, EventCatalogError> {
    if family.definition_count != count {
        return Err(EventCatalogError::FamilyCountMismatch);
    }
    Ok(EventFamilyCoverage {
        entity_kind: EVENT_REFERENCE_ENTITY_KIND.to_owned(),
        state: family.state,
        definition_count: count,
    })
}

fn collect_records(
    inputs: Vec<EventDefinitionInput>,
) -> Result<BTreeMap<String, EventDefinitionInput>, EventCatalogError> {
    let event_visibility: BTreeMap<String, EventVisibility> = inputs
        .iter()
        .map(|input| (input.event_id.clone(), input.visibility))
        .collect();
    let mut records = BTreeMap::new();
    for input in inputs {
        validate_definition(&input, &event_visibility)?;
        let bytes = definition_bytes(&input);
        if bytes > EVENT_MAX_DEFINITION_BYTES {
            return Err(EventCatalogError::DefinitionTooLarge {
                limit: EVENT_MAX_DEFINITION_BYTES,
                actual: bytes,
            });
        }
        let event_id = input.event_id.clone();
        if records.insert(event_id.clone(), input).is_some() {
            return Err(EventCatalogError::DuplicateDefinition(event_id));
        }
    }
    Ok(records)
}

fn bind_definitions(
    binding: &EventCatalogBinding,
    manifest_ids: &BTreeSet<String>,
    records: BTreeMap<String, EventDefinitionInput>,
) -> Result<BTreeMap<String, EventDefinition>, EventCatalogError> {
    let mut definitions = BTreeMap::new();
    for event_id in manifest_ids {
        let Some(input) = records.get(event_id) else {
            return Err(EventCatalogError::MissingDefinition(event_id.clone()));
        };
        definitions.insert(
            event_id.clone(),
            EventDefinition::from_input(binding, input.clone()),
        );
    }
    Ok(definitions)
}

fn validate_manifest_references(
    input: &EventDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), EventCatalogError> {
    validate_reference_list(&input.references, manifest)?;
    for page in &input.pages {
        validate_reference_list(&page.references, manifest)?;
    }
    for requirement in &input.eligibility {
        validate_reference_list(&requirement.references, manifest)?;
    }
    for option in &input.options {
        validate_reference_list(&option.references, manifest)?;
        for requirement in &option.requirements {
            validate_reference_list(&requirement.references, manifest)?;
        }
        for cost in &option.costs {
            validate_reference_list(&cost.references, manifest)?;
        }
        for outcome in &option.outcomes {
            validate_reference_list(&outcome.references, manifest)?;
            for effect in &outcome.effects {
                validate_reference_list(&effect.references, manifest)?;
            }
        }
    }
    Ok(())
}

fn validate_reference_list(
    references: &[EventSemanticReference],
    manifest: &ContentManifest,
) -> Result<(), EventCatalogError> {
    for reference in references {
        let Some(entity_kind) = manifest_entity_kind(reference) else {
            continue;
        };
        ensure_manifest_reference(manifest, entity_kind, &reference.id)?;
    }
    Ok(())
}

fn manifest_entity_kind(reference: &EventSemanticReference) -> Option<&str> {
    match &reference.kind {
        EventSemanticReferenceKind::Event => Some(EVENT_REFERENCE_ENTITY_KIND),
        EventSemanticReferenceKind::Encounter => Some(EVENT_REFERENCE_ENCOUNTER_KIND),
        EventSemanticReferenceKind::Enemy => Some(EVENT_REFERENCE_ENEMY_KIND),
        EventSemanticReferenceKind::Relic => Some(EVENT_REFERENCE_RELIC_KIND),
        EventSemanticReferenceKind::Card => Some(EVENT_REFERENCE_CARD_KIND),
        EventSemanticReferenceKind::Potion => Some(EVENT_REFERENCE_POTION_KIND),
        EventSemanticReferenceKind::Act => Some(EVENT_REFERENCE_ACT_KIND),
        EventSemanticReferenceKind::Content { entity_kind } => Some(entity_kind.as_str()),
        EventSemanticReferenceKind::Page
        | EventSemanticReferenceKind::Option
        | EventSemanticReferenceKind::Effect
        | EventSemanticReferenceKind::Rule
        | EventSemanticReferenceKind::Condition
        | EventSemanticReferenceKind::Unknown => None,
    }
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), EventCatalogError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(EventCatalogError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}
