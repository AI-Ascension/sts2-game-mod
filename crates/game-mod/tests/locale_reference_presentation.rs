// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/locale_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    LocaleCatalogError, LocalePlaceholderValue, LocalePluralCategory, LocaleRenderRequest,
    LocaleRenderedSegment,
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

fn one_text(value: &str) -> Vec<sts2_game_mod::LocaleEntryInput> {
    vec![entry(
        "text",
        "shop_title",
        EN,
        LocalePluralCategory::Other,
        vec![text(value)],
        &[],
    )]
}

#[test]
fn executable_script_presentation_is_rejected() {
    let manifest = base_manifest();
    let err = sts2_game_mod::LocaleCatalogProducer::new()
        .produce(
            &manifest,
            &Source {
                snapshot: snapshot(&manifest, locales(), one_text("<script>alert(1)</script>")),
            },
        )
        .expect_err("expected an error");
    assert_eq!(err, LocaleCatalogError::UnsafePresentation("text"));
}

#[test]
fn a_javascript_scheme_is_rejected() {
    let manifest = base_manifest();
    let err = sts2_game_mod::LocaleCatalogProducer::new()
        .produce(
            &manifest,
            &Source {
                snapshot: snapshot(&manifest, locales(), one_text("click javascript:alert(1)")),
            },
        )
        .expect_err("expected an error");
    assert_eq!(err, LocaleCatalogError::UnsafePresentation("text"));
}

#[test]
fn an_embedded_control_character_is_rejected() {
    let manifest = base_manifest();
    let err = sts2_game_mod::LocaleCatalogProducer::new()
        .produce(
            &manifest,
            &Source {
                snapshot: snapshot(&manifest, locales(), one_text("a\u{7}b")),
            },
        )
        .expect_err("expected an error");
    assert_eq!(err, LocaleCatalogError::UnsafePresentation("text"));
}

#[test]
fn non_latin_and_rtl_text_is_preserved_exactly() {
    let manifest = base_manifest();
    let value = "价格 12 伤害";
    let catalog = catalog(&manifest, one_text(value));
    let rendered = catalog
        .render(&request(EN, "text", "shop_title"))
        .expect("render");
    assert_eq!(
        rendered.segments,
        vec![LocaleRenderedSegment::Text(value.to_owned())]
    );
}

#[test]
fn ordinary_markup_is_preserved_and_never_executed() {
    let manifest = base_manifest();
    let value = "Use <b>bold</b> &amp; careful O'Brien text";
    let catalog = catalog(&manifest, one_text(value));
    let rendered = catalog
        .render(&request(EN, "text", "shop_title"))
        .expect("render");
    assert_eq!(
        rendered.segments,
        vec![LocaleRenderedSegment::Text(value.to_owned())]
    );
}

#[test]
fn supplied_placeholder_text_is_normalized_too() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain "), placeholder("amount"), text(" Block.")],
        &["amount"],
    )];
    let catalog = catalog(&manifest, entries);
    let mut render_request = request(EN, "card", "defend");
    render_request.placeholders = vec![value(
        "amount",
        LocalePlaceholderValue::Text("<script>x</script>".to_owned()),
    )];
    assert_eq!(
        catalog
            .render(&render_request)
            .expect_err("expected an error"),
        LocaleCatalogError::UnsafePresentation("placeholder_value")
    );
}
