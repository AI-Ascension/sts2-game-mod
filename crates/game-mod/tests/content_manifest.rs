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
fn locale_changes_text_revision_without_changing_semantic_identity() {
    let source = FakeCatalog {
        snapshot: base_snapshot(),
    };
    let english = producer("adapter-v1")
        .produce(&source)
        .expect("English catalog should produce");

    let mut french_snapshot = source.snapshot.clone();
    french_snapshot.locale = "fr-FR".to_owned();
    french_snapshot.definitions[0].localized_text = Some("Frappe".to_owned());
    french_snapshot.definitions[1].localized_text = Some("Frappe".to_owned());
    french_snapshot.definitions[2].localized_text = Some("Frappe".to_owned());
    let french = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: french_snapshot,
        })
        .expect("French catalog should produce");

    assert_eq!(english.content_set_revision, french.content_set_revision);
    assert_eq!(english.inventory_revision, french.inventory_revision);
    assert_ne!(
        english.localized_text_revision,
        french.localized_text_revision
    );
    assert_eq!(
        english
            .definitions
            .iter()
            .map(|definition| definition.semantic_revision.clone())
            .collect::<Vec<_>>(),
        french
            .definitions
            .iter()
            .map(|definition| definition.semantic_revision.clone())
            .collect::<Vec<_>>()
    );
    assert_ne!(
        english
            .definitions
            .iter()
            .map(|definition| definition.localized_text_revision.clone())
            .collect::<Vec<_>>(),
        french
            .definitions
            .iter()
            .map(|definition| definition.localized_text_revision.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn package_add_remove_and_order_changes_expire_old_cursor_witnesses() {
    let base = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: base_snapshot(),
        })
        .expect("base catalog should produce");
    let cursor = base.cursor_binding();

    let mut added_snapshot = base_snapshot();
    added_snapshot
        .packages
        .push(package("mod:extra", Some("1.0.0"), 2));
    let added = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: added_snapshot,
        })
        .expect("added package catalog should produce");
    assert_ne!(base.content_set_revision, added.content_set_revision);
    assert_ne!(base.inventory_revision, added.inventory_revision);
    assert!(!added.accepts_cursor(&cursor));

    let mut reordered_snapshot = base_snapshot();
    reordered_snapshot.packages[0].order = 1;
    reordered_snapshot.packages[1].order = 0;
    let reordered = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: reordered_snapshot,
        })
        .expect("reordered package catalog should produce");
    assert_ne!(base.content_set_revision, reordered.content_set_revision);
    assert_ne!(base.inventory_revision, reordered.inventory_revision);
    assert!(!reordered.accepts_cursor(&cursor));
}

#[test]
fn override_changes_definition_semantics_and_provenance_but_not_text() {
    let base = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: base_snapshot(),
        })
        .expect("base catalog should produce");
    let base_definition = base
        .definitions
        .iter()
        .find(|definition| definition.namespaced_id == "mod:synthetic:strike")
        .expect("synthetic definition");

    let mut overridden_snapshot = base_snapshot();
    overridden_snapshot.definitions[1].semantic_inputs = "cost=0;damage=7".to_owned();
    overridden_snapshot.definitions[1].override_chain = vec![
        "base:ironclad:strike".to_owned(),
        "mod:synthetic:prior-strike".to_owned(),
    ];
    let overridden = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: overridden_snapshot,
        })
        .expect("overridden catalog should produce");
    let overridden_definition = overridden
        .definitions
        .iter()
        .find(|definition| definition.namespaced_id == "mod:synthetic:strike")
        .expect("overridden definition");

    assert_ne!(
        base_definition.semantic_revision,
        overridden_definition.semantic_revision
    );
    assert_eq!(
        base_definition.localized_text_revision,
        overridden_definition.localized_text_revision
    );
    assert_ne!(base.content_set_revision, overridden.content_set_revision);
    assert_ne!(base.inventory_revision, overridden.inventory_revision);
    assert_eq!(
        overridden_definition.override_chain,
        vec![
            "base:ironclad:strike".to_owned(),
            "mod:synthetic:prior-strike".to_owned()
        ]
    );
}

#[test]
fn duplicate_display_text_keeps_namespaced_definitions_distinct_and_reports_unhandled_family() {
    let manifest = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: base_snapshot(),
        })
        .expect("base catalog should produce");

    let cards = manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == "card")
        .collect::<Vec<_>>();
    assert_eq!(cards.len(), 2);
    assert_ne!(cards[0].namespaced_id, cards[1].namespaced_id);

    let relic_family = manifest
        .families
        .iter()
        .find(|family| family.entity_kind == "relic")
        .expect("unhandled family should be present");
    assert!(!relic_family.handled);
    assert_eq!(relic_family.definition_count, 1);
    assert!(
        !manifest
            .definitions
            .iter()
            .find(|definition| definition.entity_kind == "relic")
            .expect("unhandled definition should not be omitted")
            .handled
    );
}

#[test]
fn unknown_package_version_is_preserved_without_guessing() {
    let manifest = producer("adapter-v1")
        .produce(&FakeCatalog {
            snapshot: base_snapshot(),
        })
        .expect("base catalog should produce");

    let synthetic = manifest
        .definitions
        .iter()
        .find(|definition| definition.namespaced_id == "mod:synthetic:strike")
        .expect("synthetic definition");
    assert_eq!(
        synthetic.origin.package_id.as_deref(),
        Some("mod:synthetic")
    );
    assert_eq!(synthetic.origin.package_version, None);
    assert_eq!(manifest.packages[1].package_version, None);
}

#[test]
fn catalog_change_during_extraction_fails_closed_and_does_not_publish_a_manifest() {
    let mut changed = base_snapshot();
    changed.generation_after = changed.generation_before + 1;
    let result = producer("adapter-v1").produce(&FakeCatalog { snapshot: changed });
    assert_eq!(
        result,
        Err(ContentManifestError::CatalogChanged {
            before: 7,
            after: 8
        })
    );
}

#[test]
fn source_errors_are_not_converted_into_empty_content() {
    #[derive(Debug)]
    struct FailingSource;

    impl ContentCatalogSource for FailingSource {
        fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
            Err(ContentSourceError::Unavailable)
        }
    }

    assert_eq!(
        producer("adapter-v1").produce(&FailingSource),
        Err(ContentManifestError::Source(
            ContentSourceError::Unavailable
        ))
    );
}

#[test]
fn adapter_compatibility_and_generation_invalidate_cursors_even_when_content_is_equal() {
    let source = FakeCatalog {
        snapshot: base_snapshot(),
    };
    let first = producer("adapter-v1")
        .produce(&source)
        .expect("first manifest");
    let same_content_different_adapter = producer("adapter-v2")
        .produce(&source)
        .expect("second manifest");
    assert_eq!(
        first.content_set_revision,
        same_content_different_adapter.content_set_revision
    );
    assert!(!same_content_different_adapter.accepts_cursor(&first.cursor_binding()));

    let mut reloaded = source.snapshot.clone();
    reloaded.generation_before += 1;
    reloaded.generation_after += 1;
    let after_reload = producer("adapter-v1")
        .produce(&FakeCatalog { snapshot: reloaded })
        .expect("reloaded manifest");
    assert_eq!(
        first.content_set_revision,
        after_reload.content_set_revision
    );
    assert!(!after_reload.accepts_cursor(&first.cursor_binding()));
}

#[test]
fn malformed_provenance_and_registry_values_fail_without_silent_omission() {
    let mut unknown_kind = base_snapshot();
    unknown_kind.definitions[0].entity_kind = "enemy".to_owned();
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: unknown_kind,
        }),
        Err(ContentManifestError::UnknownEntityKind)
    );

    let mut unknown_package = base_snapshot();
    unknown_package.definitions[0].origin.package_id = Some("missing:package".to_owned());
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: unknown_package,
        }),
        Err(ContentManifestError::UnknownOriginPackage)
    );

    let mut duplicate = base_snapshot();
    duplicate.definitions.push(duplicate.definitions[0].clone());
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: duplicate
        }),
        Err(ContentManifestError::DuplicateDefinition)
    );
}

#[test]
fn oversized_semantic_or_localized_values_fail_closed_and_source_is_read_only() {
    let source = FakeCatalog {
        snapshot: base_snapshot(),
    };
    let before = source.clone();
    let manifest = producer("adapter-v1")
        .produce(&source)
        .expect("valid source should produce");
    assert_eq!(source, before);
    assert_eq!(manifest.catalog_generation, 7);

    let mut oversized_semantic = base_snapshot();
    oversized_semantic.definitions[0].semantic_inputs =
        "x".repeat(sts2_game_mod::CONTENT_MANIFEST_MAX_SEMANTIC_BYTES + 1);
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: oversized_semantic,
        }),
        Err(ContentManifestError::InvalidSemanticInput)
    );

    let mut oversized_text = base_snapshot();
    oversized_text.definitions[0].localized_text =
        Some("x".repeat(sts2_game_mod::CONTENT_MANIFEST_MAX_TEXT_BYTES + 1));
    assert_eq!(
        producer("adapter-v1").produce(&FakeCatalog {
            snapshot: oversized_text,
        }),
        Err(ContentManifestError::InvalidLocalizedText)
    );
}
