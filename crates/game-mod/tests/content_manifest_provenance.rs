// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifestError,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct FakeCatalog {
    snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for FakeCatalog {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

fn producer(adapter: &str) -> ContentManifestProducer {
    ContentManifestProducer::new(adapter, [String::from("card")]).expect("valid producer")
}

fn package(package_id: &str, version: Option<&str>, order: u32) -> ContentPackageInput {
    ContentPackageInput {
        package_id: package_id.to_owned(),
        package_version: version.map(str::to_owned),
        order,
    }
}

fn definition(
    kind: &str,
    id: &str,
    semantic: &str,
    text: &str,
    package_id: Option<&str>,
    package_version: Option<&str>,
    override_chain: &[&str],
) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: semantic.to_owned(),
        localized_text: Some(text.to_owned()),
        origin: ContentOriginInput {
            package_id: package_id.map(str::to_owned),
            package_version: package_version.map(str::to_owned),
        },
        override_chain: override_chain
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    }
}

fn base_snapshot() -> ContentCatalogSnapshot {
    ContentCatalogSnapshot {
        generation_before: 7,
        generation_after: 7,
        game_build: "sts2-build:0.107.1".to_owned(),
        locale: "en-US".to_owned(),
        packages: vec![
            package("base:game", Some("0.107.1"), 0),
            package("mod:synthetic", None, 1),
        ],
        available_entity_kinds: vec!["card".to_owned(), "relic".to_owned()],
        definitions: vec![
            definition(
                "card",
                "base:ironclad:strike",
                "cost=1;damage=6",
                "Strike",
                Some("base:game"),
                Some("0.107.1"),
                &[],
            ),
            definition(
                "card",
                "mod:synthetic:strike",
                "cost=1;damage=7",
                "Strike",
                Some("mod:synthetic"),
                None,
                &["base:ironclad:strike"],
            ),
            definition(
                "relic",
                "mod:synthetic:badge",
                "trigger=room_start",
                "Strike",
                Some("mod:synthetic"),
                None,
                &[],
            ),
        ],
    }
}

#[test]
fn missing_text_and_literal_unknown_text_have_distinct_revisions_and_cursors() {
    let mut missing_snapshot = base_snapshot();
    missing_snapshot.definitions[0].localized_text = None;
    let missing = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: missing_snapshot,
        })
        .expect("missing text catalog should produce");

    let mut literal_snapshot = base_snapshot();
    literal_snapshot.definitions[0].localized_text = Some("<unknown>".to_owned());
    let literal = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: literal_snapshot,
        })
        .expect("literal text catalog should produce");

    assert_eq!(missing.content_set_revision, literal.content_set_revision);
    assert_ne!(
        missing.localized_text_revision,
        literal.localized_text_revision
    );
    assert_ne!(
        missing.definitions[0].localized_text_revision,
        literal.definitions[0].localized_text_revision
    );
    assert!(!literal.accepts_cursor(&missing.cursor_binding()));
}

#[test]
fn origin_versions_are_bounded_and_match_known_active_packages() {
    let mut invalid_origin_version = base_snapshot();
    invalid_origin_version.definitions[0].origin.package_version = Some("bad\nversion".to_owned());
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: invalid_origin_version,
        }),
        Err(ContentManifestError::InvalidPackageVersion)
    );

    let mut mismatched_origin_version = base_snapshot();
    mismatched_origin_version.definitions[0]
        .origin
        .package_version = Some("0.107.0".to_owned());
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: mismatched_origin_version,
        }),
        Err(ContentManifestError::OriginPackageVersionMismatch)
    );

    let mut matching_origin_version = base_snapshot();
    matching_origin_version.definitions[0]
        .origin
        .package_version = Some("0.107.1".to_owned());
    assert!(
        producer("adapter-v1")
            .produce(&FakeCatalog {
                snapshot: matching_origin_version,
            })
            .is_ok()
    );
}
