// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/content_index.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentIndexDefinitionInput, ContentIndexError, ContentIndexProducer, ContentIndexSourceError,
    ContentManifestProducer, ContentQueryLocale, ContentQueryScope, ContentRarity,
    ContentReferenceVisibilityPolicy, ContentSearchQuery, ContentUnlockState,
};

#[test]
fn every_manifest_family_is_enumerated_and_pages_are_deterministic() {
    let index = index();
    assert_eq!(index.families().len(), 2);
    let relic = index
        .families()
        .iter()
        .find(|family| family.entity_kind == "relic")
        .expect("relic family");
    assert!(!relic.handled);
    assert_eq!(relic.definition_count, 1);

    let mut reader = index.reader();
    let first_query = locale().list_query(ContentQueryScope::Public, 1);
    let first = reader.list(&first_query).expect("first page");
    assert_eq!(first.total, 2);
    assert_eq!(
        first.entries[0].reference.namespaced_id,
        "base:ironclad:strike"
    );
    let continuation = first.continuation.clone().expect("second page");

    let second_query = sts2_game_mod::ContentListQuery {
        continuation: Some(continuation.clone()),
        ..first_query
    };
    let second = reader.list(&second_query).expect("second page");
    assert_eq!(second.entries.len(), 1);
    assert_eq!(
        second.entries[0].reference.namespaced_id,
        "mod:synthetic:eclair"
    );
    assert_eq!(
        second.completeness,
        sts2_game_mod::ContentCompleteness::Complete
    );
    assert_eq!(
        reader.list(&sts2_game_mod::ContentListQuery {
            continuation: Some(continuation),
            ..second_query
        }),
        Err(ContentIndexError::InvalidContinuation)
    );
}

#[test]
fn literal_search_is_case_insensitive_non_ascii_and_tie_deterministic() {
    let index = index();
    let mut reader = index.reader();

    let strike = locale().search_query("strike", ContentQueryScope::Public, 8);
    let result = reader.search(&strike).expect("strike search");
    assert_eq!(result.total, 1);
    assert_eq!(result.entries[0].rank, 0);
    assert_eq!(
        result.entries[0].summary.reference.namespaced_id,
        "base:ironclad:strike"
    );

    let éclair = locale().search_query("ÉCLAIR", ContentQueryScope::Public, 8);
    let result = reader.search(&éclair).expect("non-ascii search");
    assert_eq!(result.total, 1);
    assert_eq!(
        result.entries[0].summary.reference.namespaced_id,
        "mod:synthetic:eclair"
    );

    let empty = locale().search_query("", ContentQueryScope::Public, 8);
    let result = reader.search(&empty).expect("empty search");
    assert_eq!(
        result
            .entries
            .iter()
            .map(|entry| entry.summary.reference.namespaced_id.as_str())
            .collect::<Vec<_>>(),
        vec!["base:ironclad:strike", "mod:synthetic:eclair"]
    );
}

#[test]
fn filters_are_typed_and_unsupported_filtering_fails_closed() {
    let index = index();
    let mut reader = index.reader();
    let mut query = locale().list_query(ContentQueryScope::Public, 8);
    query.filters.entity_kind = Some("card".to_owned());
    query.filters.rarity = Some(ContentRarity::new("uncommon").expect("rarity"));
    let page = reader.list(&query).expect("rarity filter");
    assert_eq!(page.total, 1);
    assert_eq!(
        page.entries[0].reference.namespaced_id,
        "mod:synthetic:eclair"
    );

    let mut origin_query = locale().list_query(ContentQueryScope::Public, 8);
    origin_query.filters.entity_kind = Some("card".to_owned());
    origin_query.filters.origin_package = Some("base:game".to_owned());
    let origin_page = reader.list(&origin_query).expect("origin filter");
    assert_eq!(origin_page.total, 1);
    assert_eq!(
        origin_page.entries[0].reference.namespaced_id,
        "base:ironclad:strike"
    );

    let mut unsupported_kind = locale().list_query(ContentQueryScope::Reference, 8);
    unsupported_kind.filters.entity_kind = Some("relic".to_owned());
    assert_eq!(
        reader.list(&unsupported_kind),
        Err(ContentIndexError::UnsupportedKind)
    );

    let mut unsupported_filter = locale().list_query(ContentQueryScope::Reference, 8);
    unsupported_filter.filters.rarity = Some(ContentRarity::new("rare").expect("rarity"));
    assert_eq!(
        reader.list(&unsupported_filter),
        Err(ContentIndexError::UnsupportedFilter(
            sts2_game_mod::ContentFilterKind::Rarity
        ))
    );
}

#[test]
fn exact_lookup_distinguishes_scope_stale_and_unsupported_results() {
    let index = index();
    let mut reader = index.reader();
    let mut query = locale().list_query(ContentQueryScope::Reference, 8);
    query.filters.entity_kind = Some("card".to_owned());
    let page = reader.list(&query).expect("reference page");
    let locked_reference = page
        .entries
        .iter()
        .find(|entry| entry.unlock_state == ContentUnlockState::Locked)
        .expect("locked entry")
        .reference
        .clone();
    let detail = reader
        .get(&locked_reference, ContentQueryScope::Reference)
        .expect("locked reference");
    assert_eq!(detail.aliases, vec!["Hit".to_owned()]);

    assert_eq!(
        reader.get(&locked_reference, ContentQueryScope::Public),
        Err(ContentIndexError::ExcludedByScope)
    );
    let mut missing = locked_reference.clone();
    missing.namespaced_id = "card:missing".to_owned();
    assert_eq!(
        reader.get(&missing, ContentQueryScope::Reference),
        Err(ContentIndexError::NotFound)
    );

    let mut relic = locked_reference;
    relic.entity_kind = "relic".to_owned();
    relic.namespaced_id = "mod:synthetic:badge".to_owned();
    assert_eq!(
        reader.get(&relic, ContentQueryScope::Reference),
        Err(ContentIndexError::UnsupportedKind)
    );

    let mut stale = detail.reference;
    stale.manifest.catalog_generation += 1;
    assert_eq!(
        reader.get(&stale, ContentQueryScope::Reference),
        Err(ContentIndexError::StaleReference)
    );
}

#[test]
fn source_and_manifest_failures_never_publish_a_partial_index() {
    let manifest = manifest();
    let before = source_snapshot(&manifest);
    let unavailable = IndexSource {
        snapshot: Err(ContentIndexSourceError::NoActiveContentSource),
    };
    assert_eq!(
        ContentIndexProducer::new(
            registry(&manifest),
            ContentReferenceVisibilityPolicy::AllowLockedReferences,
        )
        .produce(&manifest, &unavailable),
        Err(ContentIndexError::NoActiveContentSource)
    );

    let mut missing = before.clone();
    missing.definitions.pop();
    assert_eq!(
        ContentIndexProducer::new(
            registry(&manifest),
            ContentReferenceVisibilityPolicy::AllowLockedReferences,
        )
        .produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(missing),
            },
        ),
        Err(ContentIndexError::MissingDefinitionRecord {
            entity_kind: "card".to_owned(),
            namespaced_id: "mod:synthetic:eclair".to_owned(),
        })
    );

    let mut wrong_manifest = before.clone();
    wrong_manifest.manifest.catalog_generation += 1;
    assert_eq!(
        ContentIndexProducer::new(
            registry(&manifest),
            ContentReferenceVisibilityPolicy::AllowLockedReferences,
        )
        .produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(wrong_manifest),
            },
        ),
        Err(ContentIndexError::ManifestMismatch)
    );
}

#[test]
fn old_continuations_expire_after_manifest_reload_and_read_is_read_only() {
    let manifest = manifest();
    let source = IndexSource {
        snapshot: Ok(source_snapshot(&manifest)),
    };
    let producer = ContentIndexProducer::new(
        registry(&manifest),
        ContentReferenceVisibilityPolicy::AllowLockedReferences,
    );
    let first_index = producer.produce(&manifest, &source).expect("first index");
    let original_source = source.clone();
    let mut reader = first_index.reader();
    let first = reader
        .list(&locale().list_query(ContentQueryScope::Public, 1))
        .expect("first page");
    let continuation = first.continuation.expect("continuation");

    let mut reloaded_catalog = catalog_snapshot();
    reloaded_catalog.generation_before += 1;
    reloaded_catalog.generation_after += 1;
    let reloaded_manifest = ContentManifestProducer::new("adapter-v1", ["card".to_owned()])
        .expect("producer")
        .produce(&ManifestSource {
            snapshot: reloaded_catalog,
        })
        .expect("reloaded manifest");
    let reloaded_source = IndexSource {
        snapshot: Ok(source_snapshot(&reloaded_manifest)),
    };
    let reloaded_index = ContentIndexProducer::new(
        registry(&reloaded_manifest),
        ContentReferenceVisibilityPolicy::AllowLockedReferences,
    )
    .produce(&reloaded_manifest, &reloaded_source)
    .expect("reloaded index");
    reader.replace_index(reloaded_index);
    let mut next = locale().list_query(ContentQueryScope::Public, 1);
    next.continuation = Some(continuation);
    assert_eq!(
        reader.list(&next),
        Err(ContentIndexError::InvalidContinuation)
    );
    assert_eq!(source, original_source);
}

#[test]
fn duplicate_aliases_and_unsupported_source_records_are_rejected() {
    let manifest = manifest();
    let mut duplicate = source_snapshot(&manifest);
    duplicate.definitions[0].aliases.push("Hit".to_owned());
    assert_eq!(
        ContentIndexProducer::new(
            registry(&manifest),
            ContentReferenceVisibilityPolicy::AllowLockedReferences,
        )
        .produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(duplicate),
            },
        ),
        Err(ContentIndexError::DuplicateAlias)
    );

    let mut unsupported = source_snapshot(&manifest);
    unsupported
        .definitions
        .push(ContentIndexDefinitionInput::new(
            "relic",
            "mod:synthetic:badge",
        ));
    assert_eq!(
        ContentIndexProducer::new(
            registry(&manifest),
            ContentReferenceVisibilityPolicy::AllowLockedReferences,
        )
        .produce(
            &manifest,
            &IndexSource {
                snapshot: Ok(unsupported),
            },
        ),
        Err(ContentIndexError::UnsupportedKindRecord {
            entity_kind: "relic".to_owned(),
        })
    );
}

#[test]
fn search_pagination_binds_literal_locale_scope_and_filters() {
    let index = index();
    let mut reader = index.reader();
    let query = ContentSearchQuery {
        locale: locale(),
        literal: "hit".to_owned(),
        scope: ContentQueryScope::Reference,
        filters: sts2_game_mod::ContentQueryFilters {
            entity_kind: Some("card".to_owned()),
            ..Default::default()
        },
        limit: 1,
        continuation: None,
    };
    let first = reader.search(&query).expect("first alias match");
    assert_eq!(first.total, 2);
    assert_eq!(
        first.entries[0].summary.reference.namespaced_id,
        "base:ironclad:strike"
    );
    let continuation = first.continuation.clone().expect("second alias match");
    let second = reader
        .search(&ContentSearchQuery {
            continuation: Some(continuation),
            ..query.clone()
        })
        .expect("second alias match");
    assert_eq!(
        second.entries[0].summary.reference.namespaced_id,
        "mod:synthetic:strike"
    );

    let mut wrong_locale = query;
    wrong_locale.locale = ContentQueryLocale::new("fr-FR").expect("locale");
    assert_eq!(
        reader.search(&wrong_locale),
        Err(ContentIndexError::LocaleMismatch)
    );
}
