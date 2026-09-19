// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reference_text.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    REFERENCE_TEXT_MAX_PAGE_ITEMS, ReferenceFamily, ReferenceListQuery, ReferenceTextCatalog,
    ReferenceTextCatalogProducer, ReferenceTextError, ReferenceTextSourceError,
    ReferenceUnavailableReason,
};

fn full_query() -> ReferenceListQuery {
    ReferenceListQuery {
        page_size: REFERENCE_TEXT_MAX_PAGE_ITEMS,
        ..ReferenceListQuery::default()
    }
}

fn read_everything(catalog: &ReferenceTextCatalog) {
    let page = catalog.page(&full_query(), None).expect("page");
    for item in &page.items {
        let _ = catalog
            .read_document(item.family, &item.namespaced_id)
            .expect("document");
    }
    for screen_id in catalog.screen_ids() {
        let _ = catalog.read_screen(screen_id).expect("screen");
    }
    let _ = catalog.coverage();
}

fn counting_source(manifest: &sts2_game_mod::ContentManifest) -> Source {
    Source {
        snapshot: Some(snapshot(manifest, base_documents(), base_screens())),
        ..Source::default()
    }
}

#[test]
fn the_source_is_consulted_exactly_once_per_production() {
    let manifest = base_manifest();
    let source = counting_source(&manifest);
    assert_eq!(source.reads.get(), 0);
    let catalog = ReferenceTextCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");
    assert_eq!(source.reads.get(), 1);
    read_everything(&catalog);
    assert_eq!(source.reads.get(), 1);
}

#[test]
fn producing_twice_is_deterministic_and_leaves_the_source_unchanged() {
    let manifest = base_manifest();
    let source = counting_source(&manifest);
    let original = source.snapshot.clone().expect("snapshot");
    let first = ReferenceTextCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("first");
    let second = ReferenceTextCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("second");
    assert_eq!(source.reads.get(), 2);
    assert_eq!(source.snapshot.as_ref(), Some(&original));
    assert_eq!(first.binding(), second.binding());
    assert!(!original.documents.is_empty());
    assert!(!original.screens.is_empty());
}

#[test]
fn reading_never_mutates_the_retained_catalog() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let before = catalog.page(&full_query(), None).expect("before");
    let coverage_before = catalog.coverage();
    read_everything(&catalog);
    let after = catalog.page(&full_query(), None).expect("after");
    assert_eq!(before, after);
    assert_eq!(coverage_before, catalog.coverage());
    assert_eq!(catalog.document_count(), 6);
    assert_eq!(catalog.screen_count(), 4);
    // Every read is satisfied from the retained catalog, in the same availability state as before.
    let locked = catalog
        .read_document(ReferenceFamily::Lore, "lore_spire")
        .expect("lore");
    assert_eq!(
        locked.title,
        vec![sts2_game_mod::ReferenceTextSegment::Unavailable(
            ReferenceUnavailableReason::Locked
        )]
    );
}

#[test]
fn unavailable_sources_fail_closed() {
    let manifest = base_manifest();
    for (failure, expected) in [
        (
            ReferenceTextSourceError::NoActiveSource,
            ReferenceTextError::NoActiveSource,
        ),
        (
            ReferenceTextSourceError::AccessDenied,
            ReferenceTextError::SourceAccessDenied,
        ),
        (
            ReferenceTextSourceError::Malformed,
            ReferenceTextError::MalformedSource,
        ),
    ] {
        let source = Source {
            failure: Some(failure),
            ..Source::default()
        };
        let error = ReferenceTextCatalogProducer::new()
            .produce(&manifest, &source)
            .expect_err("must fail closed");
        assert_eq!(error, expected);
        assert_eq!(source.reads.get(), 1);
    }
}

#[test]
fn every_read_is_available_through_a_shared_reference() {
    // The boundary exposes no setter: a shared borrow is enough to list, read and report coverage,
    // which is what makes the slice read-only by construction rather than by convention.
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let borrowed: &ReferenceTextCatalog = &catalog;
    read_everything(borrowed);
    assert_eq!(borrowed.document_count(), 6);
}
