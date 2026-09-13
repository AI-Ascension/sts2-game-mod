// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::revisions::{
    content_set_revision, definition_semantic_revision, definition_text_revision,
    inventory_revision, localized_text_revision,
};
use super::validation::validate_snapshot;
use super::{
    ContentCatalogSource, ContentDefinition, ContentFamily, ContentManifest, ContentManifestError,
    ContentPackage, validate_identity,
};

/// Source-only content manifest producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentManifestProducer {
    adapter_compatibility: String,
    supported_entity_kinds: BTreeSet<String>,
}

impl ContentManifestProducer {
    /// Creates a producer with an explicit adapter compatibility and handled-family set.
    pub fn new(
        adapter_compatibility: impl Into<String>,
        supported_entity_kinds: impl IntoIterator<Item = String>,
    ) -> Result<Self, ContentManifestError> {
        let adapter_compatibility = adapter_compatibility.into();
        validate_identity(&adapter_compatibility, "adapter_compatibility")?;

        let mut supported = BTreeSet::new();
        for entity_kind in supported_entity_kinds {
            validate_identity(&entity_kind, "supported_entity_kind")?;
            if !supported.insert(entity_kind) {
                return Err(ContentManifestError::DuplicateEntityKind);
            }
        }
        Ok(Self {
            adapter_compatibility,
            supported_entity_kinds: supported,
        })
    }

    /// Produces one immutable manifest from an owner-owned catalog snapshot.
    pub fn produce<S: ContentCatalogSource>(
        &self,
        source: &S,
    ) -> Result<ContentManifest, ContentManifestError> {
        let snapshot = source
            .read_catalog()
            .map_err(ContentManifestError::Source)?;
        if snapshot.generation_before != snapshot.generation_after {
            return Err(ContentManifestError::CatalogChanged {
                before: snapshot.generation_before,
                after: snapshot.generation_after,
            });
        }
        validate_snapshot(&snapshot)?;

        let mut packages = snapshot
            .packages
            .iter()
            .map(|package| ContentPackage {
                package_id: package.package_id.clone(),
                package_version: package.package_version.clone(),
                order: package.order,
            })
            .collect::<Vec<_>>();
        packages.sort_by_key(|package| package.order);

        let available_kinds = snapshot
            .available_entity_kinds
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut counts = BTreeMap::<String, usize>::new();
        for definition in &snapshot.definitions {
            *counts.entry(definition.entity_kind.clone()).or_default() += 1;
        }

        let families = available_kinds
            .iter()
            .map(|entity_kind| ContentFamily {
                entity_kind: entity_kind.clone(),
                handled: self.supported_entity_kinds.contains(entity_kind),
                definition_count: counts.get(entity_kind).copied().unwrap_or(0),
            })
            .collect::<Vec<_>>();

        let mut definitions = snapshot
            .definitions
            .iter()
            .map(|definition| {
                let semantic_revision = definition_semantic_revision(definition);
                let localized_text_revision =
                    definition_text_revision(&snapshot.locale, definition);
                ContentDefinition {
                    entity_kind: definition.entity_kind.clone(),
                    namespaced_id: definition.namespaced_id.clone(),
                    handled: self
                        .supported_entity_kinds
                        .contains(&definition.entity_kind),
                    origin: definition.origin.clone(),
                    override_chain: definition.override_chain.clone(),
                    semantic_revision,
                    localized_text_revision,
                }
            })
            .collect::<Vec<_>>();
        definitions.sort_by(|left, right| {
            (&left.entity_kind, &left.namespaced_id)
                .cmp(&(&right.entity_kind, &right.namespaced_id))
        });

        let content_set_revision =
            content_set_revision(&snapshot.game_build, &packages, &families, &definitions);
        let localized_text_revision = localized_text_revision(&snapshot.locale, &definitions);
        let inventory_revision =
            inventory_revision(&snapshot.game_build, &packages, &families, &definitions);

        Ok(ContentManifest {
            game_build: snapshot.game_build,
            adapter_compatibility: self.adapter_compatibility.clone(),
            catalog_generation: snapshot.generation_after,
            locale: snapshot.locale,
            packages,
            families,
            definitions,
            content_set_revision,
            localized_text_revision,
            inventory_revision,
        })
    }
}
