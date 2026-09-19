// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reference_text.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    REFERENCE_TEXT_MAX_PAGE_ITEMS, ReferenceDiscovery, ReferenceFamily, ReferenceListQuery,
    ReferenceTextError,
};

fn query(
    family: Option<ReferenceFamily>,
    keyword: Option<&str>,
    locale: Option<&str>,
    page_size: usize,
) -> ReferenceListQuery {
    ReferenceListQuery {
        family,
        keyword: keyword.map(str::to_owned),
        locale: locale.map(str::to_owned),
        page_size,
    }
}

#[test]
fn listing_is_partitioned_by_family_in_a_deterministic_order() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let tutorial = catalog
        .page(
            &query(Some(ReferenceFamily::Tutorial), None, None, 16),
            None,
        )
        .expect("tutorial page");
    assert_eq!(tutorial.total, 1);
    assert_eq!(tutorial.items[0].namespaced_id, "tutorial_combat");
    assert!(tutorial.next.is_none());

    let all = catalog
        .page(&query(None, None, None, 16), None)
        .expect("all page");
    assert_eq!(all.total, 6);
    let rows: Vec<(ReferenceFamily, &str)> = all
        .items
        .iter()
        .map(|item| (item.family, item.namespaced_id.as_str()))
        .collect();
    assert_eq!(
        rows,
        vec![
            (ReferenceFamily::Tutorial, "tutorial_combat"),
            (ReferenceFamily::Help, "help_keywords"),
            (ReferenceFamily::Lore, "lore_spire"),
            (ReferenceFamily::Credits, "credits_team"),
            (ReferenceFamily::UiText, "ui_hud"),
            (ReferenceFamily::Unknown, "legacy_secret"),
        ]
    );
}

#[test]
fn keyword_search_indexes_document_and_section_keywords() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let by_document = catalog
        .page(&query(None, Some("tutorial"), None, 16), None)
        .expect("document keyword");
    assert_eq!(by_document.total, 1);
    assert_eq!(by_document.items[0].namespaced_id, "tutorial_combat");

    let by_section = catalog
        .page(&query(None, Some("turn"), None, 16), None)
        .expect("section keyword");
    assert_eq!(by_section.total, 1);
    assert_eq!(by_section.items[0].namespaced_id, "tutorial_combat");

    let unknown_family = catalog
        .page(&query(None, Some("legacy"), None, 16), None)
        .expect("unprojected keyword");
    assert_eq!(unknown_family.total, 1);
    assert_eq!(unknown_family.items[0].family, ReferenceFamily::Unknown);
}

#[test]
fn an_empty_page_still_carries_full_coverage() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let empty = catalog
        .page(&query(None, Some("absent_keyword"), None, 16), None)
        .expect("empty page");
    assert_eq!(empty.total, 0);
    assert!(empty.items.is_empty());
    assert!(empty.next.is_none());
    assert_eq!(empty.coverage.families.len(), ReferenceFamily::COUNT);
    assert_eq!(
        empty.coverage.unsupported_documents,
        vec!["legacy_secret".to_owned()]
    );
    let documents: usize = empty
        .coverage
        .families
        .iter()
        .map(|row| row.documents)
        .sum();
    assert_eq!(documents, 6);
}

#[test]
fn the_locale_filter_matches_the_authored_locale() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let english = catalog
        .page(&query(None, None, Some(EN), 16), None)
        .expect("english");
    assert_eq!(english.total, 6);
    let german = catalog
        .page(&query(None, None, Some(DE), 16), None)
        .expect("german");
    assert_eq!(german.total, 0);
    assert!(german.items.is_empty());
    assert_eq!(german.coverage.families.len(), ReferenceFamily::COUNT);
}

#[test]
fn page_size_and_query_shape_are_bounded() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    assert_eq!(
        catalog
            .page(&query(None, None, None, 0), None)
            .expect_err("zero"),
        ReferenceTextError::InvalidPageSize
    );
    assert_eq!(
        catalog
            .page(
                &query(None, None, None, REFERENCE_TEXT_MAX_PAGE_ITEMS + 1),
                None
            )
            .expect_err("too large"),
        ReferenceTextError::InvalidPageSize
    );
    assert_eq!(
        catalog
            .page(&query(None, None, Some("en US"), 16), None)
            .expect_err("locale"),
        ReferenceTextError::InvalidInput("locale")
    );
    assert_eq!(
        catalog
            .page(&query(None, Some(""), None, 16), None)
            .expect_err("keyword"),
        ReferenceTextError::InvalidInput("keyword")
    );
}

#[test]
fn a_bounded_page_walk_visits_every_matching_document_exactly_once() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let mut seen: Vec<String> = Vec::new();
    let mut continuation = None;
    loop {
        let page = catalog
            .page(&query(None, None, None, 2), continuation.as_ref())
            .expect("page");
        for item in &page.items {
            seen.push(item.namespaced_id.clone());
        }
        match page.next {
            Some(next) => continuation = Some(next),
            None => break,
        }
    }
    assert_eq!(
        seen,
        vec![
            "tutorial_combat",
            "help_keywords",
            "lore_spire",
            "credits_team",
            "ui_hud",
            "legacy_secret",
        ]
    );
}

#[test]
fn a_continuation_bound_to_another_family_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let token = catalog
        .page(&query(None, None, None, 2), None)
        .expect("page")
        .next
        .expect("continuation");
    assert_eq!(
        catalog
            .page(
                &query(Some(ReferenceFamily::Tutorial), None, None, 2),
                Some(&token)
            )
            .expect_err("family mismatch"),
        ReferenceTextError::QueryMismatch
    );
}

#[test]
fn a_continuation_bound_to_another_catalog_revision_is_rejected() {
    let manifest = base_manifest();
    let english = catalog(&manifest);
    let german = catalog_for(
        &manifest,
        snapshot_in(&manifest, DE, base_documents(), base_screens()),
    )
    .expect("german catalog");
    assert_eq!(german.locale(), DE);
    let token = english
        .page(&query(None, None, None, 2), None)
        .expect("page")
        .next
        .expect("continuation");
    assert_eq!(
        german
            .page(&query(None, None, None, 2), Some(&token))
            .expect_err("stale revision"),
        ReferenceTextError::StaleReference
    );
}

#[test]
fn a_continuation_bound_to_a_changed_query_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let token = catalog
        .page(&query(None, None, None, 2), None)
        .expect("page")
        .next
        .expect("continuation");
    assert_eq!(
        catalog
            .page(&query(None, None, None, 3), Some(&token))
            .expect_err("changed page size"),
        ReferenceTextError::InvalidContinuation
    );
    assert_eq!(
        catalog
            .page(&query(None, Some("help"), None, 2), Some(&token))
            .expect_err("changed keyword"),
        ReferenceTextError::InvalidContinuation
    );
}

#[test]
fn an_offset_beyond_the_matching_total_is_rejected() {
    let manifest = base_manifest();
    let full = catalog(&manifest);
    let token = full
        .page(&query(None, None, None, 2), None)
        .expect("page")
        .next
        .expect("continuation");
    let trimmed = catalog_for(
        &manifest,
        snapshot(
            &manifest,
            vec![document(
                ReferenceFamily::Help,
                "help_only",
                ReferenceDiscovery::Open,
                vec![text("Only one document")],
                Vec::new(),
                &[],
            )],
            Vec::new(),
        ),
    )
    .expect("trimmed catalog");
    assert_eq!(trimmed.document_count(), 1);
    assert_eq!(
        trimmed
            .page(&query(None, None, None, 2), Some(&token))
            .expect_err("offset beyond total"),
        ReferenceTextError::InvalidContinuation
    );
}
