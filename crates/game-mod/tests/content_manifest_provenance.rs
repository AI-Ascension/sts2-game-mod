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
        registry_definition_counts: [("card".to_owned(), 2), ("relic".to_owned(), 1)].into(),
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

#[test]
fn adapter_support_changes_inventory_but_not_semantic_content_identity() {
    let source = FakeCatalog {
        snapshot: base_snapshot(),
    };
    let cards = producer("adapter-v1").produce(&source).expect("cards");
    let all = ContentManifestProducer::new("adapter-v1", ["card".to_owned(), "relic".to_owned()])
        .expect("producer")
        .produce(&source)
        .expect("all families");
    assert_eq!(cards.content_set_revision, all.content_set_revision);
    assert_eq!(cards.localized_text_revision, all.localized_text_revision);
    for (before, after) in cards.definitions.iter().zip(&all.definitions) {
        assert_eq!(before.semantic_revision, after.semantic_revision);
    }
    assert_ne!(cards.inventory_revision, all.inventory_revision);
    assert!(!all.accepts_cursor(&cards.cursor_binding()));
}

#[test]
fn independent_registry_counts_reject_missing_extra_and_partial_inventory() {
    let mut missing = base_snapshot();
    missing.registry_definition_counts.remove("relic");
    let mut extra = base_snapshot();
    extra
        .registry_definition_counts
        .insert("unknown".to_owned(), 0);
    for snapshot in [missing, extra] {
        assert_eq!(
            producer("adapter-v1").produce(&FakeCatalog { snapshot }),
            Err(ContentManifestError::RegistryCountCoverageMismatch),
        );
    }
    for family in ["card", "relic"] {
        let mut partial = base_snapshot();
        partial
            .definitions
            .retain(|definition| definition.entity_kind != family);
        assert_eq!(
            producer("adapter-v1").produce(&FakeCatalog { snapshot: partial }),
            Err(ContentManifestError::RegistryDefinitionCountMismatch),
        );
        let mut undercounted = base_snapshot();
        undercounted
            .registry_definition_counts
            .insert(family.to_owned(), 0);
        assert_eq!(
            producer("adapter-v1").produce(&FakeCatalog {
                snapshot: undercounted
            }),
            Err(ContentManifestError::RegistryDefinitionCountMismatch),
        );
    }
}

#[test]
fn verified_empty_unhandled_family_is_distinct_from_missing_registry_evidence() {
    let mut snapshot = base_snapshot();
    snapshot
        .available_entity_kinds
        .push("unsupported".to_owned());
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: snapshot.clone()
        }),
        Err(ContentManifestError::RegistryCountCoverageMismatch),
    );
    snapshot
        .registry_definition_counts
        .insert("unsupported".to_owned(), 0);
    let manifest = producer("adapter-v1")
        .produce(&FakeCatalog { snapshot })
        .expect("independently verified empty family");
    let family = manifest
        .families
        .iter()
        .find(|family| family.entity_kind == "unsupported")
        .expect("unsupported family retained");
    assert_eq!(family.definition_count, 0);
    assert!(!family.handled);
}

#[test]
fn canonical_input_order_is_stable_and_package_removal_expires_cursors() {
    let snapshot = base_snapshot();
    let base = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: snapshot.clone(),
        })
        .expect("base");
    let mut reordered = snapshot.clone();
    reordered.packages.reverse();
    reordered.available_entity_kinds.reverse();
    reordered.definitions.reverse();
    assert_eq!(
        base,
        producer("adapter-v1")
            .produce(&FakeCatalog {
                snapshot: reordered
            })
            .expect("reordered")
    );
    let mut added = snapshot;
    added.packages.push(package("mod:unused", None, 2));
    let with_package = producer("adapter-v1")
        .produce(&FakeCatalog { snapshot: added })
        .expect("unused active package");
    assert_ne!(with_package.content_set_revision, base.content_set_revision);
    assert!(!base.accepts_cursor(&with_package.cursor_binding()));
}
