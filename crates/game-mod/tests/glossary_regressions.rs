// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/glossary.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentQueryLocale, GLOSSARY_MAX_ALIAS_COUNT, GLOSSARY_MAX_DETAIL_BYTES,
    GLOSSARY_MAX_RELATED_TERM_COUNT, GLOSSARY_MAX_TEXT_BYTES, GlossaryCatalogError,
    GlossaryProducer, GlossaryQueryScope, GlossaryReferenceVisibilityPolicy, GlossarySearchQuery,
    GlossarySnapshot, GlossarySourceError, GlossaryTermInput, GlossaryTermReference,
};

fn source_with(snapshot: GlossarySnapshot) -> GlossaryFixtureSource {
    GlossaryFixtureSource {
        snapshot: Ok(snapshot),
    }
}

#[test]
fn stale_identity_locale_scope_and_source_errors_fail_closed() {
    let manifest = manifest();
    let producer = GlossaryProducer::new(GlossaryReferenceVisibilityPolicy::AllowReferenceTerms);
    let source = source_with(snapshot(&manifest));
    let source_before = source.clone();
    producer
        .produce(&manifest, &source)
        .expect("read-only fixture");
    assert_eq!(source, source_before);

    let mut stale = snapshot(&manifest);
    stale.manifest.catalog_generation += 1;
    assert_eq!(
        producer.produce(&manifest, &source_with(stale)),
        Err(GlossaryCatalogError::ManifestMismatch)
    );

    let mut wrong_locale = snapshot(&manifest);
    wrong_locale.locale = "fr-FR".to_owned();
    assert_eq!(
        producer.produce(&manifest, &source_with(wrong_locale)),
        Err(GlossaryCatalogError::LocaleMismatch)
    );

    let mut wrong_version = snapshot(&manifest);
    wrong_version.producer_version = "other-producer-v9".to_owned();
    assert_eq!(
        producer.produce(&manifest, &source_with(wrong_version)),
        Err(GlossaryCatalogError::ProducerVersionMismatch)
    );

    let unavailable = GlossaryFixtureSource {
        snapshot: Err(GlossarySourceError::NoActiveSource),
    };
    assert_eq!(
        producer.produce(&manifest, &unavailable),
        Err(GlossaryCatalogError::NoActiveSource)
    );

    let catalog = catalog();
    let hidden_reference = GlossaryTermReference {
        catalog: catalog.binding().clone(),
        term_id: "keyword:hidden".to_owned(),
    };
    assert_eq!(
        catalog.get(&hidden_reference, GlossaryQueryScope::Reference),
        Err(GlossaryCatalogError::ExcludedByScope)
    );
}

#[test]
fn duplicate_aliases_controls_and_oversized_detail_are_rejected() {
    let manifest = manifest();
    let producer = GlossaryProducer::new(GlossaryReferenceVisibilityPolicy::AllowReferenceTerms);

    let mut duplicate_alias = snapshot(&manifest);
    duplicate_alias.terms[0].aliases = vec!["same".to_owned(), "same".to_owned()];
    assert_eq!(
        producer.produce(&manifest, &source_with(duplicate_alias)),
        Err(GlossaryCatalogError::DuplicateAlias)
    );

    let mut too_many_aliases = snapshot(&manifest);
    too_many_aliases.terms[0].aliases = (0..=GLOSSARY_MAX_ALIAS_COUNT)
        .map(|index| format!("alias-{index}"))
        .collect();
    assert_eq!(
        producer.produce(&manifest, &source_with(too_many_aliases)),
        Err(GlossaryCatalogError::CollectionTooLarge {
            field: "aliases",
            limit: GLOSSARY_MAX_ALIAS_COUNT,
            actual: GLOSSARY_MAX_ALIAS_COUNT + 1,
        })
    );

    let mut controls = snapshot(&manifest);
    controls.terms[0].display_name = "bad\nname".to_owned();
    assert_eq!(
        producer.produce(&manifest, &source_with(controls)),
        Err(GlossaryCatalogError::InvalidText)
    );

    let mut oversized = snapshot(&manifest);
    oversized.terms[0].aliases = (0..64)
        .map(|index| format!("{index}-{}", "x".repeat(GLOSSARY_MAX_TEXT_BYTES - 16)))
        .collect();
    let catalog = producer
        .produce(&manifest, &source_with(oversized))
        .expect("source text remains within field bound");
    let reference = GlossaryTermReference {
        catalog: catalog.binding().clone(),
        term_id: "status:strength".to_owned(),
    };
    assert!(matches!(
        catalog.get(&reference, GlossaryQueryScope::Reference),
        Err(GlossaryCatalogError::DetailTooLarge {
            limit: GLOSSARY_MAX_DETAIL_BYTES,
            actual,
        }) if actual > GLOSSARY_MAX_DETAIL_BYTES
    ));
}

#[test]
fn nested_resolved_reference_payloads_count_toward_detail_limit() {
    let manifest = manifest();
    let producer = GlossaryProducer::new(GlossaryReferenceVisibilityPolicy::AllowReferenceTerms);
    let mut unresolved = snapshot(&manifest);
    unresolved.terms[0].aliases = (0..3)
        .map(|index| format!("{index}-{}", "x".repeat(GLOSSARY_MAX_TEXT_BYTES - 4096)))
        .collect();
    unresolved.terms[0].related_terms = (0..GLOSSARY_MAX_RELATED_TERM_COUNT)
        .map(|index| format!("keyword:t{index}-{}", "y".repeat(238)))
        .collect();
    let catalog = producer
        .produce(&manifest, &source_with(unresolved.clone()))
        .expect("unresolved references remain bounded");
    let reference = GlossaryTermReference {
        catalog: catalog.binding().clone(),
        term_id: "status:strength".to_owned(),
    };
    assert!(
        catalog
            .get(&reference, GlossaryQueryScope::Reference)
            .is_ok(),
        "outer unresolved IDs fit below the detail bound"
    );

    let mut resolved = unresolved;
    resolved
        .terms
        .extend((0..GLOSSARY_MAX_RELATED_TERM_COUNT).map(|index| {
            GlossaryTermInput::new(
                format!("keyword:t{index}-{}", "y".repeat(238)),
                format!("Target {index}"),
            )
        }));
    let catalog = producer
        .produce(&manifest, &source_with(resolved))
        .expect("resolved references remain source-valid");
    let reference = GlossaryTermReference {
        catalog: catalog.binding().clone(),
        term_id: "status:strength".to_owned(),
    };
    assert!(matches!(
        catalog.get(&reference, GlossaryQueryScope::Reference),
        Err(GlossaryCatalogError::DetailTooLarge {
            limit: GLOSSARY_MAX_DETAIL_BYTES,
            actual,
        }) if actual > GLOSSARY_MAX_DETAIL_BYTES
    ));
}

#[test]
fn coverage_composition_requires_matching_manifest_and_locale() {
    let producer = GlossaryProducer::new(GlossaryReferenceVisibilityPolicy::AllowReferenceTerms);
    let manifest = manifest();
    let content_index = content_index();
    let catalog = producer
        .produce_with_content_index(&manifest, &source_with(snapshot(&manifest)), &content_index)
        .expect("matching composition");
    assert_eq!(
        catalog.coverage().status,
        sts2_game_mod::GlossaryCoverageStatus::Partial
    );

    let mut stale_manifest = manifest.clone();
    stale_manifest.catalog_generation = stale_manifest.catalog_generation.saturating_add(1);
    let stale_catalog = producer
        .produce(&stale_manifest, &source_with(snapshot(&stale_manifest)))
        .expect("stale glossary snapshot");
    assert_eq!(
        stale_catalog.with_content_index(&content_index),
        Err(GlossaryCatalogError::ManifestMismatch)
    );

    let mut wrong_locale = manifest;
    wrong_locale.locale = "fr-FR".to_owned();
    let wrong_locale_catalog = producer
        .produce(&wrong_locale, &source_with(snapshot(&wrong_locale)))
        .expect("locale-bound glossary snapshot");
    assert_eq!(
        wrong_locale_catalog.with_content_index(&content_index),
        Err(GlossaryCatalogError::LocaleMismatch)
    );
}

#[test]
fn wrong_query_bindings_and_reference_policy_cannot_reuse_cursor() {
    let manifest = manifest();
    let public_catalog = GlossaryProducer::default()
        .produce(&manifest, &source_with(snapshot(&manifest)))
        .expect("catalog");
    let mut reader = public_catalog.reader();
    let first = reader
        .search(&GlossarySearchQuery {
            locale: ContentQueryLocale::new("en-US").expect("locale"),
            literal: "".to_owned(),
            scope: GlossaryQueryScope::Public,
            limit: 1,
            continuation: None,
        })
        .expect("first");
    let continuation = first.continuation.expect("cursor");
    assert_eq!(
        reader.search(&GlossarySearchQuery {
            locale: ContentQueryLocale::new("en-US").expect("locale"),
            literal: "different".to_owned(),
            scope: GlossaryQueryScope::Public,
            limit: 1,
            continuation: Some(continuation.clone()),
        }),
        Err(GlossaryCatalogError::InvalidContinuation)
    );
    assert_eq!(
        reader.search(&GlossarySearchQuery {
            locale: ContentQueryLocale::new("fr-FR").expect("locale"),
            literal: "".to_owned(),
            scope: GlossaryQueryScope::Public,
            limit: 1,
            continuation: Some(continuation),
        }),
        Err(GlossaryCatalogError::LocaleMismatch)
    );

    let strict = GlossaryProducer::new(GlossaryReferenceVisibilityPolicy::PublicOnly)
        .produce(&manifest, &source_with(snapshot(&manifest)))
        .expect("strict catalog");
    let mut strict_reader = strict.reader();
    assert_eq!(
        strict_reader
            .search(&GlossarySearchQuery {
                locale: ContentQueryLocale::new("en-US").expect("locale"),
                literal: "eclair".to_owned(),
                scope: GlossaryQueryScope::Reference,
                limit: 8,
                continuation: None,
            })
            .expect("strict search")
            .total,
        0
    );
}
