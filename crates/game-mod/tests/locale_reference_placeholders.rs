// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/locale_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    LocaleCatalogError, LocaleCompleteness, LocalePlaceholderValue, LocalePluralCategory,
    LocaleRenderRequest, LocaleRenderedSegment, LocaleUnavailableReason,
};

fn plural_request(
    locale: &str,
    kind: &str,
    id: &str,
    plural: Option<LocalePluralCategory>,
) -> LocaleRenderRequest {
    LocaleRenderRequest {
        locale: locale.to_owned(),
        entity_kind: kind.to_owned(),
        namespaced_id: id.to_owned(),
        plural,
        placeholders: Vec::new(),
    }
}

fn blocking_entries() -> Vec<sts2_game_mod::LocaleEntryInput> {
    vec![entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain "), placeholder("amount"), text(" Block.")],
        &["amount"],
    )]
}

#[test]
fn dynamic_parameters_substitute_without_changing_numbers() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, blocking_entries());
    let mut request = plural_request(EN, "card", "defend", None);
    request.placeholders = vec![value(
        "amount",
        LocalePlaceholderValue::Number("7".to_owned()),
    )];
    let rendered = catalog.render(&request).expect("render");
    assert_eq!(
        rendered.segments,
        vec![
            LocaleRenderedSegment::Text("Gain ".to_owned()),
            LocaleRenderedSegment::Text("7".to_owned()),
            LocaleRenderedSegment::Text(" Block.".to_owned()),
        ]
    );
    assert!(rendered.unresolved_placeholders.is_empty());
    assert_eq!(rendered.completeness, LocaleCompleteness::Complete);
}

#[test]
fn an_unresolved_placeholder_stays_explicit() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, blocking_entries());
    let rendered = catalog
        .render(&plural_request(EN, "card", "defend", None))
        .expect("render");
    assert_eq!(
        rendered.segments[1],
        LocaleRenderedSegment::UnresolvedPlaceholder("amount".to_owned())
    );
    assert_eq!(rendered.unresolved_placeholders, vec!["amount".to_owned()]);
    assert_eq!(rendered.completeness, LocaleCompleteness::Partial);
}

#[test]
fn an_unavailable_placeholder_value_stays_unresolved() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, blocking_entries());
    let mut request = plural_request(EN, "card", "defend", None);
    request.placeholders = vec![value(
        "amount",
        LocalePlaceholderValue::Unavailable(LocaleUnavailableReason::NotTranslated),
    )];
    let rendered = catalog.render(&request).expect("render");
    assert_eq!(rendered.unresolved_placeholders, vec!["amount".to_owned()]);
    assert_eq!(rendered.completeness, LocaleCompleteness::Partial);
}

#[test]
fn an_undeclared_supplied_placeholder_is_rejected() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, blocking_entries());
    let mut request = plural_request(EN, "card", "defend", None);
    request.placeholders = vec![value("other", LocalePlaceholderValue::Text("x".to_owned()))];
    assert_eq!(
        catalog.render(&request).expect_err("expected an error"),
        LocaleCatalogError::UnknownPlaceholder("other".to_owned())
    );
}

#[test]
fn a_supplied_reference_placeholder_keeps_its_identity() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, blocking_entries());
    let mut request = plural_request(EN, "card", "defend", None);
    request.placeholders = vec![value(
        "amount",
        LocalePlaceholderValue::Reference(reference("card", "strike")),
    )];
    let rendered = catalog.render(&request).expect("render");
    assert_eq!(
        rendered.segments[1],
        LocaleRenderedSegment::Reference(reference("card", "strike"))
    );
    assert_eq!(rendered.completeness, LocaleCompleteness::Complete);
}

#[test]
fn a_plural_form_is_selected_then_falls_back_to_the_other_form() {
    let manifest = base_manifest();
    let entries = vec![
        entry(
            "card",
            "defend",
            EN,
            LocalePluralCategory::One,
            vec![text("Gain 1 Block.")],
            &[],
        ),
        entry(
            "card",
            "defend",
            EN,
            LocalePluralCategory::Other,
            vec![text("Gain some Block.")],
            &[],
        ),
    ];
    let catalog = catalog(&manifest, entries);
    let one = catalog
        .render(&plural_request(
            EN,
            "card",
            "defend",
            Some(LocalePluralCategory::One),
        ))
        .expect("one");
    assert_eq!(
        one.segments,
        vec![LocaleRenderedSegment::Text("Gain 1 Block.".to_owned())]
    );
    let few = catalog
        .render(&plural_request(
            EN,
            "card",
            "defend",
            Some(LocalePluralCategory::Few),
        ))
        .expect("few");
    assert_eq!(
        few.segments,
        vec![LocaleRenderedSegment::Text("Gain some Block.".to_owned())]
    );
}

#[test]
fn a_plural_request_falls_back_to_the_default_locale() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Other,
        vec![text("Gain some Block.")],
        &[],
    )];
    let catalog = catalog(&manifest, entries);
    let rendered = catalog
        .render(&plural_request(
            AR,
            "card",
            "defend",
            Some(LocalePluralCategory::One),
        ))
        .expect("fallback");
    assert_eq!(rendered.effective_locale, EN);
    assert_eq!(rendered.fallback_chain, vec![AR.to_owned(), EN.to_owned()]);
    assert_eq!(rendered.completeness, LocaleCompleteness::Partial);
}

#[test]
fn an_unrecognized_plural_token_is_not_silently_accepted() {
    assert_eq!(LocalePluralCategory::parse("plural"), None);
    assert_eq!(
        LocalePluralCategory::parse("few"),
        Some(LocalePluralCategory::Few)
    );
    assert!(LocalePluralCategory::Other.is_fallback_form());
}

#[test]
fn a_plural_form_absent_from_every_stored_form_is_not_silently_substituted() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::Few,
        vec![text("Gain a few Block.")],
        &[],
    )];
    let catalog = catalog(&manifest, entries);
    assert_eq!(
        catalog
            .render(&plural_request(
                EN,
                "card",
                "defend",
                Some(LocalePluralCategory::One)
            ))
            .expect_err("expected an error"),
        LocaleCatalogError::NotFound
    );
}

#[test]
fn a_non_plural_request_with_only_a_non_other_form_is_not_silently_substituted() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "defend",
        EN,
        LocalePluralCategory::One,
        vec![text("Gain 1 Block.")],
        &[],
    )];
    let catalog = catalog(&manifest, entries);
    assert_eq!(
        catalog
            .render(&plural_request(EN, "card", "defend", None))
            .expect_err("expected an error"),
        LocaleCatalogError::NotFound
    );
}

#[test]
fn the_effective_plural_is_reported_with_the_render() {
    let manifest = base_manifest();
    let entries = vec![
        entry(
            "card",
            "defend",
            EN,
            LocalePluralCategory::One,
            vec![text("Gain 1 Block.")],
            &[],
        ),
        entry(
            "card",
            "defend",
            EN,
            LocalePluralCategory::Other,
            vec![text("Gain some Block.")],
            &[],
        ),
    ];
    let catalog = catalog(&manifest, entries);
    let exact = catalog
        .render(&plural_request(
            EN,
            "card",
            "defend",
            Some(LocalePluralCategory::One),
        ))
        .expect("one");
    assert_eq!(exact.effective_plural, LocalePluralCategory::One);
    assert_eq!(exact.completeness, LocaleCompleteness::Complete);
    let substituted = catalog
        .render(&plural_request(
            EN,
            "card",
            "defend",
            Some(LocalePluralCategory::Few),
        ))
        .expect("few");
    assert_eq!(substituted.effective_plural, LocalePluralCategory::Other);
}
