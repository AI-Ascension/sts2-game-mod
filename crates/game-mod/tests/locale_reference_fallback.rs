// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/locale_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    LocaleCatalogError, LocaleCompleteness, LocaleDirection, LocalePluralCategory,
    LocaleRenderRequest, LocaleRenderedSegment,
};

fn request(locale: &str, kind: &str, id: &str) -> LocaleRenderRequest {
    LocaleRenderRequest {
        locale: locale.to_owned(),
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
        plural: None,
        placeholders: Vec::new(),
    }
}

#[test]
fn requested_locale_hit_is_complete() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let rendered = catalog
        .render(&request(EN, "card", "strike"))
        .expect("english");
    assert_eq!(rendered.completeness, LocaleCompleteness::Complete);
    assert_eq!(rendered.fallback_chain, vec![EN.to_owned()]);
    assert!(rendered.unresolved_placeholders.is_empty());
}

#[test]
fn missing_translation_uses_the_declared_fallback_and_reports_it() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let rendered = catalog
        .render(&request(AR, "card", "strike"))
        .expect("fallback");
    assert_eq!(rendered.requested_locale, AR);
    assert_eq!(rendered.effective_locale, EN);
    assert_eq!(rendered.fallback_chain, vec![AR.to_owned(), EN.to_owned()]);
    assert_eq!(rendered.completeness, LocaleCompleteness::Partial);
    assert_eq!(rendered.direction, LocaleDirection::LeftToRight);
    assert_eq!(rendered.text_revision, "text-en-US-strike");
}

#[test]
fn an_rtl_locale_reports_its_own_direction_when_it_has_text() {
    let manifest = base_manifest();
    let mut entries = baseline_entries();
    entries.push(entry(
        "text",
        "shop_title",
        AR,
        LocalePluralCategory::Other,
        vec![text("متجر")],
        &[],
    ));
    let catalog = catalog(&manifest, entries);
    let rendered = catalog
        .render(&request(AR, "text", "shop_title"))
        .expect("arabic");
    assert_eq!(rendered.effective_locale, AR);
    assert_eq!(rendered.fallback_chain, vec![AR.to_owned()]);
    assert_eq!(rendered.direction, LocaleDirection::RightToLeft);
    assert_eq!(rendered.completeness, LocaleCompleteness::Complete);
    assert_eq!(
        rendered.segments,
        vec![LocaleRenderedSegment::Text("متجر".to_owned())]
    );
}

#[test]
fn an_exhausted_chain_is_never_filled_with_an_invented_value() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(
        catalog
            .render(&request(EN, "card", "defend"))
            .expect_err("expected an error"),
        LocaleCatalogError::NotFound
    );
}

#[test]
fn an_unknown_requested_locale_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(
        catalog
            .render(&request("fr-FR", "card", "strike"))
            .expect_err("expected an error"),
        LocaleCatalogError::UnsupportedLocale("fr-FR".to_owned())
    );
}

#[test]
fn fallback_chain_is_exposed_for_every_supported_locale() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(catalog.fallback_chain(EN), Some([EN.to_owned()].as_slice()));
    assert_eq!(
        catalog.fallback_chain(DE),
        Some([DE.to_owned(), EN.to_owned()].as_slice())
    );
    assert_eq!(catalog.fallback_chain("ja-JP"), None);
}
