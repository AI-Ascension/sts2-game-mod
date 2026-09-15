// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    CONTENT_INDEX_MAX_ALIAS_COUNT, CONTENT_INDEX_MAX_DEFINITION_BYTES,
    CONTENT_INDEX_MAX_TEXT_BYTES, ContentIndexDefinitionInput, ContentIndexError,
    ContentIndexProducer, ContentListQuery, ContentManifestProducer, ContentQueryFilters,
    ContentQueryScope, ContentReferenceVisibilityPolicy,
};

fn producer(registry_manifest: &sts2_game_mod::ContentManifest) -> ContentIndexProducer {
    ContentIndexProducer::new(
        registry(registry_manifest),
        ContentReferenceVisibilityPolicy::AllowLockedReferences,
    )
}

fn manifest_from_catalog(
    catalog: sts2_game_mod::ContentCatalogSnapshot,
) -> sts2_game_mod::ContentManifest {
    ContentManifestProducer::new("adapter-v1", ["card".to_owned()])
        .expect("producer")
        .produce(&ManifestSource { snapshot: catalog })
        .expect("manifest")
}

#[test]
fn adapter_registry_reconciles_added_and_removed_manifest_families() {
    let base = manifest();
    let base_registry = registry(&base);
    let producer = ContentIndexProducer::new(
        base_registry.clone(),
        ContentReferenceVisibilityPolicy::AllowLockedReferences,
    );

    let mut added_catalog = catalog_snapshot();
    added_catalog
        .available_entity_kinds
        .push("potion".to_owned());
    added_catalog
        .registry_definition_counts
        .insert("potion".to_owned(), 0);
    added_catalog
        .registry_definition_counts
        .insert("relic".to_owned(), 0);
    added_catalog
        .definitions
        .retain(|definition| definition.entity_kind == "card");
    let added_manifest = manifest_from_catalog(added_catalog);
    let added = producer
        .produce(
            &added_manifest,
            &IndexSource {
                snapshot: Ok(source_snapshot(&added_manifest)),
            },
        )
        .expect("added family remains explicitly unsupported");
    assert!(
        added
            .families()
            .iter()
            .any(|family| family.entity_kind == "potion" && !family.handled)
    );

    let mut removed_catalog = catalog_snapshot();
    removed_catalog
        .available_entity_kinds
        .retain(|kind| kind != "relic");
    removed_catalog.registry_definition_counts.remove("relic");
    removed_catalog
        .definitions
        .retain(|definition| definition.entity_kind != "relic");
    let removed_manifest = manifest_from_catalog(removed_catalog);
    let removed = producer
        .produce(
            &removed_manifest,
            &IndexSource {
                snapshot: Ok(source_snapshot(&removed_manifest)),
            },
        )
        .expect("removed family is omitted");
    assert!(
        removed
            .families()
            .iter()
            .all(|family| family.entity_kind != "relic")
    );
    assert_eq!(
        base_registry.get("relic").map(|adapter| adapter.handled),
        Some(false)
    );
}

#[test]
fn unknown_record_identity_is_validated_before_payload_bearing_error() {
    let manifest = manifest();
    let mut source = source_snapshot(&manifest);
    source
        .definitions
        .push(ContentIndexDefinitionInput::new("bogus\nkind", "bogus:id"));
    assert_eq!(
        producer(&manifest).produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(source)
            }
        ),
        Err(ContentIndexError::InvalidIdentity("entity_kind"))
    );

    let mut invalid_id_source = source_snapshot(&manifest);
    invalid_id_source
        .definitions
        .push(ContentIndexDefinitionInput::new("bogus", "bogus\nid"));
    assert_eq!(
        producer(&manifest).produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(invalid_id_source)
            }
        ),
        Err(ContentIndexError::InvalidIdentity("namespaced_id"))
    );
}

#[test]
fn aliases_are_count_bounded_and_exact_get_has_an_aggregate_detail_bound() {
    let manifest = manifest();
    let mut too_many_aliases = source_snapshot(&manifest);
    too_many_aliases.definitions[0].aliases = (0..=CONTENT_INDEX_MAX_ALIAS_COUNT)
        .map(|index| format!("a-{index}"))
        .collect();
    assert_eq!(
        producer(&manifest).produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(too_many_aliases),
            },
        ),
        Err(ContentIndexError::AliasCountTooLarge {
            limit: CONTENT_INDEX_MAX_ALIAS_COUNT,
            actual: CONTENT_INDEX_MAX_ALIAS_COUNT + 1,
        })
    );

    let mut oversized_detail = source_snapshot(&manifest);
    oversized_detail.definitions[0].rendered_description =
        Some("x".repeat(CONTENT_INDEX_MAX_TEXT_BYTES));
    let index = producer(&manifest)
        .produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(oversized_detail),
            },
        )
        .expect("source field itself remains bounded");
    let mut reader = index.reader();
    let mut query = ContentListQuery {
        locale: fixture::locale(),
        scope: ContentQueryScope::Reference,
        filters: Default::default(),
        limit: 8,
        continuation: None,
    };
    query.filters.entity_kind = Some("card".to_owned());
    let reference = reader
        .list(&query)
        .expect("list")
        .entries
        .first()
        .expect("card")
        .reference
        .clone();
    assert!(matches!(
        reader.get(&reference, ContentQueryScope::Reference),
        Err(ContentIndexError::DetailTooLarge {
            limit: CONTENT_INDEX_MAX_DEFINITION_BYTES,
            actual,
        }) if actual > CONTENT_INDEX_MAX_DEFINITION_BYTES
    ));
}

#[test]
fn advertised_capabilities_must_match_source_field_availability() {
    let manifest = manifest();
    let mut missing_name = source_snapshot(&manifest);
    missing_name.definitions[0].display_name = None;
    assert_eq!(
        producer(&manifest).produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(missing_name),
            },
        ),
        Err(ContentIndexError::CapabilityMismatch("display_name"))
    );
}

#[test]
fn glossary_term_references_are_bounded_and_retained_on_definition_detail() {
    let manifest = manifest();
    let mut source = source_snapshot(&manifest);
    source.definitions[0].term_references =
        vec!["status:strength".to_owned(), "keyword:damage".to_owned()];
    let index = producer(&manifest)
        .produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(source.clone()),
            },
        )
        .expect("term references");
    let mut reader = index.reader();
    let page = reader
        .list(&ContentListQuery {
            locale: fixture::locale(),
            scope: ContentQueryScope::Reference,
            filters: ContentQueryFilters {
                entity_kind: Some("card".to_owned()),
                ..Default::default()
            },
            limit: 8,
            continuation: None,
        })
        .expect("card list");
    assert_eq!(
        page.entries[0].term_references,
        vec!["status:strength".to_owned(), "keyword:damage".to_owned()]
    );
    let reference = page.entries[0].reference.clone();
    let detail = reader
        .get(&reference, ContentQueryScope::Reference)
        .expect("detail");
    assert_eq!(
        detail.term_references,
        vec!["status:strength".to_owned(), "keyword:damage".to_owned()]
    );

    let mut duplicate = source;
    duplicate.definitions[0].term_references =
        vec!["status:strength".to_owned(), "status:strength".to_owned()];
    assert_eq!(
        producer(&manifest).produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(duplicate),
            },
        ),
        Err(ContentIndexError::DuplicateTermReference)
    );

    let mut malformed = source_snapshot(&manifest);
    malformed.definitions[0].term_references = vec!["bad\nterm".to_owned()];
    assert_eq!(
        producer(&manifest).produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(malformed),
            },
        ),
        Err(ContentIndexError::InvalidIdentity("term_reference"))
    );
}
