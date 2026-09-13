// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::engine::ContentIndex;
use super::errors::ContentIndexError;
use super::model::{
    ContentDefinition, ContentDefinitionReference, ContentIndexDefinitionInput, ContentIndexFamily,
    ContentIndexSource, ContentIndexSourceError,
};
use super::query::ContentReferenceVisibilityPolicy;
use super::registry::ContentKindAdapterRegistry;

/// Builds an immutable index from one manifest and one owner-local typed source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentIndexProducer {
    registry: ContentKindAdapterRegistry,
    locked_visibility: ContentReferenceVisibilityPolicy,
}

impl ContentIndexProducer {
    /// Creates a producer with explicit family adapters and locked-reference policy.
    #[must_use]
    pub fn new(
        registry: ContentKindAdapterRegistry,
        locked_visibility: ContentReferenceVisibilityPolicy,
    ) -> Self {
        Self {
            registry,
            locked_visibility,
        }
    }

    /// Produces one immutable index without constructing playable objects or mutating profiles.
    pub fn produce<S: ContentIndexSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<ContentIndex, ContentIndexError> {
        let snapshot = source.read_index(manifest).map_err(map_source_error)?;
        let manifest_binding = manifest.cursor_binding();
        if snapshot.manifest != manifest_binding {
            return Err(ContentIndexError::ManifestMismatch);
        }
        if snapshot.locale != manifest.locale {
            return Err(ContentIndexError::LocaleMismatch);
        }
        let registry = self.registry.for_manifest(manifest);
        let mut records = collect_records(snapshot.definitions)?;
        let mut definitions = BTreeMap::new();
        for manifest_definition in &manifest.definitions {
            let Some(adapter) = registry.get(&manifest_definition.entity_kind) else {
                return Err(ContentIndexError::UnsupportedKind);
            };
            let key = (
                manifest_definition.entity_kind.clone(),
                manifest_definition.namespaced_id.clone(),
            );
            if !adapter.handled {
                if records.remove(&key).is_some() {
                    return Err(ContentIndexError::UnsupportedKindRecord {
                        entity_kind: manifest_definition.entity_kind.clone(),
                    });
                }
                continue;
            }
            let Some(input) = records.remove(&key) else {
                return Err(ContentIndexError::MissingDefinitionRecord {
                    entity_kind: manifest_definition.entity_kind.clone(),
                    namespaced_id: manifest_definition.namespaced_id.clone(),
                });
            };
            input.validate()?;
            input.validate_capabilities(&adapter.detail_capabilities)?;
            let reference = ContentDefinitionReference {
                manifest: manifest_binding.clone(),
                entity_kind: manifest_definition.entity_kind.clone(),
                namespaced_id: manifest_definition.namespaced_id.clone(),
            };
            let definition = ContentDefinition {
                reference,
                origin: manifest_definition.origin.clone(),
                override_chain: manifest_definition.override_chain.clone(),
                semantic_revision: manifest_definition.semantic_revision.clone(),
                localized_text_revision: manifest_definition.localized_text_revision.clone(),
                display_name: input.display_name,
                aliases: input.aliases,
                rendered_description: input.rendered_description,
                character_or_pool: input.character_or_pool,
                rarity: input.rarity,
                unlock_state: input.unlock_state,
                term_references: input.term_references,
                detail_capabilities: adapter.detail_capabilities.clone(),
            };
            definitions.insert(key, definition);
        }
        if !records.is_empty() {
            return Err(ContentIndexError::UnknownDefinition {
                entity_kind: records
                    .keys()
                    .next()
                    .map(|key| key.0.clone())
                    .unwrap_or_default(),
                namespaced_id: records
                    .keys()
                    .next()
                    .map(|key| key.1.clone())
                    .unwrap_or_default(),
            });
        }

        let families = registry
            .entries()
            .map(|adapter| {
                let definition_count = manifest
                    .families
                    .iter()
                    .find(|family| family.entity_kind == adapter.entity_kind)
                    .map_or(0, |family| family.definition_count);
                ContentIndexFamily {
                    entity_kind: adapter.entity_kind.clone(),
                    handled: adapter.handled,
                    definition_count,
                    detail_capabilities: adapter.detail_capabilities.clone(),
                    supported_filters: adapter.supported_filters.clone(),
                }
            })
            .collect::<Vec<_>>();

        Ok(ContentIndex::from_parts(
            manifest_binding,
            manifest.locale.clone(),
            registry,
            self.locked_visibility,
            families,
            definitions,
        ))
    }
}

fn collect_records(
    inputs: Vec<ContentIndexDefinitionInput>,
) -> Result<BTreeMap<(String, String), ContentIndexDefinitionInput>, ContentIndexError> {
    let mut records = BTreeMap::new();
    for input in inputs {
        input.validate()?;
        let key = (input.entity_kind.clone(), input.namespaced_id.clone());
        if records.insert(key, input).is_some() {
            return Err(ContentIndexError::DuplicateDefinitionRecord);
        }
    }
    Ok(records)
}

fn map_source_error(error: ContentIndexSourceError) -> ContentIndexError {
    match error {
        ContentIndexSourceError::NoActiveContentSource => ContentIndexError::NoActiveContentSource,
        ContentIndexSourceError::AccessDenied => ContentIndexError::SourceAccessDenied,
        ContentIndexSourceError::Malformed => ContentIndexError::MalformedSource,
    }
}
