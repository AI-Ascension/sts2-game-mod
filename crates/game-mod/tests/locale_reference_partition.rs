// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/locale_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    LOCALE_REFERENCE_MAX_PAGE_ITEMS, LocaleCatalogError, LocaleEntryListQuery,
    LocalePluralCategory, LocaleRenderRequest,
};

fn query(locale: Option<&str>, kind: Option<&str>, page_size: usize) -> LocaleEntryListQuery {
    LocaleEntryListQuery {
        locale: locale.map(str::to_owned),
        entity_kind: kind.map(str::to_owned),
        page_size,
    }
}

#[test]
fn paging_is_partitioned_by_locale() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let english = catalog
        .page(&query(Some(EN), None, 10), None)
        .expect("english page");
    assert_eq!(english.total, 1);
    assert_eq!(english.items[0].locale, EN);
    assert!(english.next.is_none());
    let german = catalog
        .page(&query(Some(DE), None, 10), None)
        .expect("german page");
    assert_eq!(german.total, 1);
    assert_eq!(german.items[0].locale, DE);
    let all = catalog
        .page(&query(None, None, 10), None)
        .expect("all page");
    assert_eq!(all.total, 2);
}

#[test]
fn paging_walks_forward_with_a_bound_continuation() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let first = catalog
        .page(&query(None, None, 1), None)
        .expect("first page");
    assert_eq!(first.items.len(), 1);
    assert_eq!(first.total, 2);
    let token = first.next.expect("continuation");
    assert_eq!(token.locale_signature, "*");
    let second = catalog
        .page(&query(None, None, 1), Some(&token))
        .expect("second page");
    assert_eq!(second.items.len(), 1);
    assert!(second.next.is_none());
    assert_ne!(first.items[0], second.items[0]);
}

#[test]
fn a_continuation_from_another_locale_is_rejected() {
    let manifest = base_manifest();
    let single_page = catalog(&manifest, baseline_entries());
    let token = single_page
        .page(&query(Some(EN), None, 1), None)
        .expect("english page")
        .next;
    assert!(token.is_none(), "one english entry fits in one page");
    let mut entries = baseline_entries();
    entries.push(entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain Block.")],
        &[],
    ));
    let widened = catalog(&manifest, entries);
    let token = widened
        .page(&query(Some(EN), None, 1), None)
        .expect("english page")
        .next
        .expect("continuation");
    assert_eq!(
        widened
            .page(&query(Some(DE), None, 1), Some(&token))
            .expect_err("expected an error"),
        LocaleCatalogError::LocaleMismatch
    );
}

#[test]
fn a_continuation_across_a_query_change_is_rejected() {
    let manifest = base_manifest();
    let mut entries = baseline_entries();
    entries.push(entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain Block.")],
        &[],
    ));
    let catalog = catalog(&manifest, entries);
    let token = catalog
        .page(&query(Some(EN), None, 1), None)
        .expect("english page")
        .next
        .expect("continuation");
    assert_eq!(
        catalog
            .page(&query(Some(EN), Some("card"), 1), Some(&token))
            .expect_err("expected an error"),
        LocaleCatalogError::InvalidContinuation
    );
}

#[test]
fn an_invalid_page_size_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(
        catalog
            .page(&query(None, None, 0), None)
            .expect_err("expected an error"),
        LocaleCatalogError::InvalidPageSize
    );
    assert_eq!(
        catalog
            .page(
                &query(None, None, LOCALE_REFERENCE_MAX_PAGE_ITEMS + 1),
                None
            )
            .expect_err("expected an error"),
        LocaleCatalogError::InvalidPageSize
    );
}

#[test]
fn an_unsupported_page_locale_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(
        catalog
            .page(&query(Some("fr-FR"), None, 10), None)
            .expect_err("expected an error"),
        LocaleCatalogError::UnsupportedLocale("fr-FR".to_owned())
    );
}

#[test]
fn a_catalog_revision_change_invalidates_a_cached_page() {
    let first_manifest = base_manifest();
    let mut entries = baseline_entries();
    entries.push(entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain Block.")],
        &[],
    ));
    let first = catalog(&first_manifest, entries.clone());
    let token = first
        .page(&query(Some(EN), None, 1), None)
        .expect("first page")
        .next
        .expect("continuation");
    let second_manifest = manifest(&[
        ("card", "defend"),
        ("card", "strike"),
        ("relic", "burning_blood"),
        ("text", "shop_title"),
        ("card", "bash"),
    ]);
    let second = catalog(&second_manifest, entries);
    assert_ne!(
        first.binding().manifest.content_set_revision,
        second.binding().manifest.content_set_revision
    );
    assert_eq!(
        second
            .page(&query(Some(EN), None, 1), Some(&token))
            .expect_err("expected an error"),
        LocaleCatalogError::StaleReference
    );
}

#[test]
fn a_reference_from_another_catalog_is_stale() {
    let first_manifest = base_manifest();
    let first = catalog(&first_manifest, baseline_entries());
    let second_manifest = manifest(&[
        ("card", "defend"),
        ("card", "strike"),
        ("relic", "burning_blood"),
        ("text", "shop_title"),
        ("card", "bash"),
    ]);
    let second = catalog(&second_manifest, baseline_entries());
    let stale = first
        .reference(EN, &reference("card", "strike"))
        .expect("reference");
    assert_eq!(
        second.resolve(&stale).expect_err("expected an error"),
        LocaleCatalogError::StaleReference
    );
    let resolved = second
        .resolve(
            &second
                .reference(EN, &reference("card", "strike"))
                .expect("reference"),
        )
        .expect("resolve");
    assert_eq!(resolved.namespaced_id, "strike");
}

#[test]
fn a_reference_for_an_unknown_locale_or_absent_definition_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(
        catalog
            .reference("fr-FR", &reference("card", "strike"))
            .expect_err("expected an error"),
        LocaleCatalogError::UnsupportedLocale("fr-FR".to_owned())
    );
    assert_eq!(
        catalog
            .reference(EN, &reference("card", "defend"))
            .expect_err("expected an error"),
        LocaleCatalogError::NotFound
    );
}

#[test]
fn render_never_mutates_the_catalog_between_concurrent_locales() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let english_first = catalog
        .render(&LocaleRenderRequest {
            locale: EN.to_owned(),
            entity_kind: "card".to_owned(),
            namespaced_id: "strike".to_owned(),
            plural: None,
            placeholders: Vec::new(),
        })
        .expect("english");
    let _german = catalog
        .render(&LocaleRenderRequest {
            locale: DE.to_owned(),
            entity_kind: "card".to_owned(),
            namespaced_id: "strike".to_owned(),
            plural: None,
            placeholders: Vec::new(),
        })
        .expect("german");
    let english_second = catalog
        .render(&LocaleRenderRequest {
            locale: EN.to_owned(),
            entity_kind: "card".to_owned(),
            namespaced_id: "strike".to_owned(),
            plural: None,
            placeholders: Vec::new(),
        })
        .expect("english again");
    assert_eq!(english_first, english_second);
}

#[test]
fn a_continuation_replayed_with_a_different_page_size_is_rejected() {
    let manifest = base_manifest();
    let mut entries = baseline_entries();
    entries.push(entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain Block.")],
        &[],
    ));
    let catalog = catalog(&manifest, entries);
    let token = catalog
        .page(&query(Some(EN), None, 1), None)
        .expect("english page")
        .next
        .expect("continuation");
    assert_eq!(
        catalog
            .page(&query(Some(EN), None, 2), Some(&token))
            .expect_err("expected an error"),
        LocaleCatalogError::InvalidContinuation
    );
}
