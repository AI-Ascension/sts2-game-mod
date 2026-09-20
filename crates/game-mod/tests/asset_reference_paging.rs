// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/asset_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/asset_reference.rs"]
mod support;
#[path = "support/asset_reference_port.rs"]
mod support_port;

use manifest_fixture::fixture_manifest;
use sts2_game_mod::{
    ASSET_MAX_PAGE_ITEMS, AssetClassState, AssetEntryInput, AssetMediaClass, AssetMediaKind,
    AssetReferenceError, AssetRenditionAuthority, AssetRetrievalState, AssetRevision,
    AssetVisibilityScope,
};
use support::*;
use support_port::*;

#[test]
fn a_complete_page_reports_every_visible_asset() {
    let (_, catalog) = fixture_catalog();
    let page = catalog
        .list(&query("en-US", AssetVisibilityScope::Owner, 64))
        .expect("page");

    assert!(page.complete);
    assert!(!page.is_partial());
    assert_eq!(page.total, 8);
    assert_eq!(page.assets.len(), 8);
    assert!(page.continuation.is_none());
    assert_eq!(page.binding, *catalog.binding());
    assert_eq!(page.revision, catalog.revision());
    assert_eq!(page.authority, AssetRenditionAuthority::NotGranted);

    let handles = page
        .assets
        .iter()
        .map(|summary| summary.reference.handle.as_str())
        .collect::<Vec<_>>();
    assert_eq!(handles, FIXTURE_HANDLES);
    assert!(page.assets.iter().all(|summary| !summary.carries_binary()));
}

#[test]
fn a_partial_page_needs_a_retained_reader() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog.list(&query("en-US", AssetVisibilityScope::Owner, 2)),
        Err(AssetReferenceError::PartialPageRequiresReader)
    );

    let mut reader = catalog.reader();
    let page = reader
        .list(&query("en-US", AssetVisibilityScope::Owner, 2))
        .expect("page");
    assert!(page.is_partial());
    assert!(!page.complete);
    assert_eq!(page.assets.len(), 2);
    assert!(page.continuation.is_some());
}

#[test]
fn continuing_a_page_walks_every_visible_asset_once() {
    let (_, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let mut seen: Vec<String> = Vec::new();
    let mut continuation = None;

    loop {
        let mut request = query("en-US", AssetVisibilityScope::Owner, 3);
        request.continuation = continuation.take();
        let page = reader.list(&request).expect("page");
        seen.extend(
            page.assets
                .iter()
                .map(|summary| summary.reference.handle.as_str().to_owned()),
        );
        match page.continuation {
            Some(next) => continuation = Some(next),
            None => break,
        }
    }

    assert_eq!(seen, FIXTURE_HANDLES);
    assert_eq!(seen.len(), 8);
}

#[test]
fn a_continuation_is_single_use() {
    let (_, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list(&query("en-US", AssetVisibilityScope::Owner, 2))
        .expect("page");
    let continuation = page.continuation.expect("continuation");

    let mut first_use = query("en-US", AssetVisibilityScope::Owner, 2);
    first_use.continuation = Some(continuation.clone());
    let second = reader.list(&first_use).expect("page");
    assert_eq!(second.assets.len(), 2);

    let mut reused = query("en-US", AssetVisibilityScope::Owner, 2);
    reused.continuation = Some(continuation);
    assert_eq!(
        reader.list(&reused),
        Err(AssetReferenceError::InvalidContinuation)
    );
}

#[test]
fn a_continuation_from_another_reader_is_refused() {
    let (_, catalog) = fixture_catalog();
    let mut first = catalog.reader();
    let mut second = catalog.reader();
    let page = first
        .list(&query("en-US", AssetVisibilityScope::Owner, 2))
        .expect("page");

    let mut request = query("en-US", AssetVisibilityScope::Owner, 2);
    request.continuation = page.continuation;
    assert_eq!(
        second.list(&request),
        Err(AssetReferenceError::InvalidContinuation)
    );
}

#[test]
fn a_continuation_bound_to_another_query_is_refused() {
    let (_, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list(&query("en-US", AssetVisibilityScope::Owner, 2))
        .expect("page");
    let continuation = page.continuation.expect("continuation");

    let mut wider = query("en-US", AssetVisibilityScope::Owner, 3);
    wider.continuation = Some(continuation.clone());
    assert_eq!(
        reader.list(&wider),
        Err(AssetReferenceError::InvalidContinuation)
    );

    let mut narrower_scope = query("en-US", AssetVisibilityScope::Public, 2);
    narrower_scope.media_kind = Some(AssetMediaKind::Icon);
    narrower_scope.continuation = Some(continuation);
    assert_eq!(
        reader.list(&narrower_scope),
        Err(AssetReferenceError::InvalidContinuation)
    );
}

#[test]
fn a_zero_or_oversized_page_limit_is_refused() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog.list(&query("en-US", AssetVisibilityScope::Owner, 0)),
        Err(AssetReferenceError::InvalidPageSize)
    );
    assert_eq!(
        catalog.list(&query(
            "en-US",
            AssetVisibilityScope::Owner,
            ASSET_MAX_PAGE_ITEMS + 1
        )),
        Err(AssetReferenceError::InvalidPageSize)
    );
}

#[test]
fn a_query_in_another_locale_is_refused() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(
        catalog.list(&query("de-DE", AssetVisibilityScope::Owner, 64)),
        Err(AssetReferenceError::LocaleMismatch)
    );
}

#[test]
fn a_query_at_another_revision_is_refused() {
    let (_, catalog) = fixture_catalog();
    let mut stale = query("en-US", AssetVisibilityScope::Owner, 64);
    stale.revision = Some(AssetRevision {
        content_revision: "revision.other".to_owned(),
        generation: catalog.binding().generation,
    });
    assert_eq!(
        catalog.list(&stale),
        Err(AssetReferenceError::ContentRevisionMismatch)
    );

    let mut current = query("en-US", AssetVisibilityScope::Owner, 64);
    current.revision = Some(catalog.revision());
    assert!(catalog.list(&current).is_ok());
}

#[test]
fn filters_narrow_the_page_without_hiding_the_total() {
    let (_, catalog) = fixture_catalog();

    let mut audio = query("en-US", AssetVisibilityScope::Owner, 64);
    audio.media_kind = Some(AssetMediaKind::Audio);
    let page = catalog.list(&audio).expect("page");
    assert_eq!(page.total, 2);
    assert!(
        page.assets
            .iter()
            .all(|summary| summary.media_kind == AssetMediaKind::Audio)
    );

    let mut metadata_only = query("en-US", AssetVisibilityScope::Owner, 64);
    metadata_only.retrieval_state = Some(AssetRetrievalState::MetadataOnly);
    assert_eq!(catalog.list(&metadata_only).expect("page").total, 3);

    let mut missing = query("en-US", AssetVisibilityScope::Owner, 64);
    missing.retrieval_state = Some(AssetRetrievalState::AssetMissing);
    let page = catalog.list(&missing).expect("page");
    assert_eq!(page.total, 1);
    assert!(!page.assets[0].media_properties.is_available());
    assert!(!page.assets[0].byte_size.is_available());
}

#[test]
fn scope_rules_decide_which_assets_are_visible() {
    let (_, catalog) = fixture_catalog();
    let public = catalog
        .list(&query("en-US", AssetVisibilityScope::Public, 64))
        .expect("page");
    let reference = catalog
        .list(&query("en-US", AssetVisibilityScope::Reference, 64))
        .expect("page");
    let owner = catalog
        .list(&query("en-US", AssetVisibilityScope::Owner, 64))
        .expect("page");

    assert_eq!(public.total, 6);
    assert_eq!(reference.total, 7);
    assert_eq!(owner.total, 8);
    let handles = |page: &sts2_game_mod::AssetPage| {
        page.assets
            .iter()
            .map(|summary| summary.reference.handle.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    assert!(!handles(&public).contains(&"asset.owner.ambience".to_owned()));
    assert!(handles(&reference).contains(&"asset.owner.ambience".to_owned()));
    assert!(!handles(&reference).contains(&"asset.hidden.marker".to_owned()));
    assert!(handles(&owner).contains(&"asset.hidden.marker".to_owned()));
}

#[test]
fn a_class_the_source_does_not_project_cannot_be_listed() {
    let manifest = fixture_manifest();
    let entries: Vec<AssetEntryInput> = fixture_assets(&manifest)
        .into_iter()
        .filter(|entry| entry.media_kind.media_class() == AssetMediaClass::Image)
        .collect();
    let mut snapshot = snapshot(&manifest, entries);
    let audio = snapshot
        .classes
        .iter_mut()
        .find(|row| row.class == AssetMediaClass::Audio)
        .expect("audio row");
    audio.state = AssetClassState::Unsupported;
    assert_eq!(audio.asset_count, 0);
    let catalog = produce_with(&manifest, &FixturePort::answering(snapshot)).expect("catalog");

    let mut filtered = query("en-US", AssetVisibilityScope::Owner, 64);
    filtered.media_kind = Some(AssetMediaKind::Audio);
    assert_eq!(
        catalog.list(&filtered),
        Err(AssetReferenceError::UnavailableClass)
    );
    assert!(
        catalog
            .list(&query("en-US", AssetVisibilityScope::Owner, 64))
            .is_ok()
    );
}

#[test]
fn an_exact_lookup_refuses_a_reference_from_another_catalog() {
    let (_, catalog) = fixture_catalog();
    let mut reference = entry_reference(&catalog, "asset.icon.strike");
    reference.catalog.generation += 1;
    assert_eq!(
        catalog.get(&reference, AssetVisibilityScope::Owner, None),
        Err(AssetReferenceError::StaleReference)
    );
}

#[test]
fn a_lookup_at_another_revision_or_handle_is_refused() {
    let (_, catalog) = fixture_catalog();
    let reference = entry_reference(&catalog, "asset.icon.strike");
    let stale = AssetRevision {
        content_revision: "revision.other".to_owned(),
        generation: catalog.binding().generation,
    };
    assert_eq!(
        catalog.get(&reference, AssetVisibilityScope::Owner, Some(&stale)),
        Err(AssetReferenceError::ContentRevisionMismatch)
    );
    assert_eq!(
        catalog.get(
            &entry_reference(&catalog, "asset.absent.ghost"),
            AssetVisibilityScope::Owner,
            None
        ),
        Err(AssetReferenceError::NotFound)
    );
    assert_eq!(
        catalog.get(&reference, AssetVisibilityScope::Public, None),
        Ok(catalog
            .get(&reference, AssetVisibilityScope::Owner, None)
            .expect("entry"))
    );
}
