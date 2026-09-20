// SPDX-License-Identifier: MIT

//! The content manifest every asset fixture record resolves its definitions against.

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
};

/// Manifest definitions the shared asset fixture links its entries to.
pub const FIXTURE_ENTRIES: &[(&str, &str)] = &[
    ("card", "card.bash"),
    ("card", "card.strike"),
    ("relic", "relic.burning_blood"),
];

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

fn available_kinds(entries: &[(&str, &str)]) -> Vec<String> {
    let mut kinds: Vec<String> = entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    kinds.sort();
    kinds.dedup();
    kinds
}

pub fn manifest_source(entries: &[(&str, &str)]) -> ManifestSource {
    let definitions = entries
        .iter()
        .map(|(entity_kind, id)| content_definition(entity_kind, id))
        .collect();
    let kinds = available_kinds(entries);
    ManifestSource {
        snapshot: ContentCatalogSnapshot {
            generation_before: 7,
            generation_after: 7,
            game_build: "sts2-build:synthetic".to_owned(),
            locale: "en-US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:synthetic".to_owned(),
                package_version: Some("1".to_owned()),
                order: 0,
            }],
            registry_definition_counts: kinds
                .iter()
                .map(|kind| {
                    (
                        kind.clone(),
                        entries
                            .iter()
                            .filter(|(entry_kind, _)| entry_kind == kind)
                            .count(),
                    )
                })
                .collect(),
            available_entity_kinds: kinds,
            definitions,
        },
    }
}

pub fn manifest(entries: &[(&str, &str)]) -> ContentManifest {
    ContentManifestProducer::new("asset-adapter-v1", available_kinds(entries))
        .expect("producer")
        .produce(&manifest_source(entries))
        .expect("manifest")
}

/// The manifest every fixture asset resolves its linked definition against.
pub fn fixture_manifest() -> ContentManifest {
    manifest(FIXTURE_ENTRIES)
}

/// A manifest whose asset entries link to no definition the producer handles.
pub fn unhandled_manifest() -> ContentManifest {
    manifest(&[("encounter", "encounter.gremlin")])
}
