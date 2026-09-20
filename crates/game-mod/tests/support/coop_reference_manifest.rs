// SPDX-License-Identifier: MIT

//! The manifest and value builders every co-op fixture party resolves its references against.

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    CoopEntry, CoopEntryKind, CoopFieldValue, CoopHealth, CoopQuantity, CoopUnit,
};

/// Manifest families the shared fixture resolves its member references against.
pub const FIXTURE_ENTRIES: &[(&str, &str)] = &[
    ("card", "card.bash"),
    ("card", "card.defend"),
    ("card", "card.strike"),
    ("character", "character.ironclad"),
    ("character", "character.silent"),
    ("effect", "effect.barricade"),
    ("effect", "effect.rage"),
    ("potion", "potion.fire"),
    ("power", "power.strength"),
    ("relic", "relic.burning_blood"),
    ("special_mechanic", "mechanic.replay"),
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

pub fn manifest_source(entries: &[(&str, &str)]) -> ManifestSource {
    let definitions = entries
        .iter()
        .map(|(entity_kind, id)| content_definition(entity_kind, id))
        .collect();
    let mut available_entity_kinds: Vec<String> =
        entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    available_entity_kinds.sort();
    available_entity_kinds.dedup();
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
            registry_definition_counts: available_entity_kinds
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
            available_entity_kinds,
            definitions,
        },
    }
}

pub fn manifest(entries: &[(&str, &str)]) -> ContentManifest {
    let mut available_entity_kinds: Vec<String> =
        entries.iter().map(|(kind, _)| (*kind).to_owned()).collect();
    available_entity_kinds.sort();
    available_entity_kinds.dedup();
    ContentManifestProducer::new("adapter-v1", available_entity_kinds)
        .expect("producer")
        .produce(&manifest_source(entries))
        .expect("manifest")
}

/// The manifest every fixture party resolves its member references against.
pub fn fixture_manifest() -> ContentManifest {
    manifest(FIXTURE_ENTRIES)
}

pub fn unit(name: &str) -> CoopUnit {
    CoopUnit {
        unit: name.to_owned(),
    }
}

pub fn quantity(name: &str, amount: i64) -> CoopQuantity {
    CoopQuantity {
        amount,
        unit: unit(name),
    }
}

pub fn health(current: i64, maximum: i64) -> CoopFieldValue<CoopHealth> {
    CoopFieldValue::present(CoopHealth { current, maximum })
}

pub fn entry(kind: CoopEntryKind, id: &str, count: u32, label: &str) -> CoopEntry {
    CoopEntry {
        entry_id: id.to_owned(),
        kind,
        count,
        label: CoopFieldValue::present(label.to_owned()),
    }
}
