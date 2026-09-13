// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/glossary.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentQueryLocale, GLOSSARY_MAX_ALIAS_COUNT, GLOSSARY_MAX_DETAIL_BYTES,
    GLOSSARY_MAX_TEXT_BYTES, GlossaryCatalogError, GlossaryProducer, GlossaryQueryScope,
    GlossaryReferenceVisibilityPolicy, GlossarySearchQuery, GlossarySnapshot, GlossarySourceError,
    GlossaryTermReference,
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
