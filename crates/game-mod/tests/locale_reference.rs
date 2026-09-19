// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/locale_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    LOCALE_REFERENCE_PRODUCER_VERSION, LocaleCompleteness, LocaleDirection, LocalePluralCategory,
    LocaleRenderRequest, LocaleRenderedSegment, LocaleTextSegment,
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
fn catalog_binds_the_manifest_and_producer_identity() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(
        catalog.binding().producer_version,
        LOCALE_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.default_locale(), EN);
    assert_eq!(catalog.entry_count(), 2);
    assert_eq!(catalog.variant_count("card", "strike"), 2);
}

#[test]
fn supported_locales_keep_owner_order_and_direction() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let tags: Vec<&str> = catalog
        .supported_locales()
        .iter()
        .map(|input| input.locale.as_str())
        .collect();
    assert_eq!(tags, vec![EN, DE, AR]);
    assert_eq!(catalog.direction(AR), Some(LocaleDirection::RightToLeft));
    assert_eq!(catalog.direction(EN), Some(LocaleDirection::LeftToRight));
    assert_eq!(
        catalog.fallback_chain(AR),
        Some([AR.to_owned(), EN.to_owned()].as_slice())
    );
    assert_eq!(catalog.direction("fr-FR"), None);
}

#[test]
fn stable_identity_is_independent_of_language() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let english = catalog
        .render(&request(EN, "card", "strike"))
        .expect("english");
    let german = catalog
        .render(&request(DE, "card", "strike"))
        .expect("german");
    assert_eq!(english.entity_kind, german.entity_kind);
    assert_eq!(english.namespaced_id, german.namespaced_id);
    assert_eq!(english.namespaced_id, "strike");
    assert_ne!(english.text_revision, german.text_revision);
    assert_ne!(english.segments, german.segments);
    assert_eq!(english.completeness, LocaleCompleteness::Complete);
    assert_eq!(german.completeness, LocaleCompleteness::Complete);
    assert_eq!(english.effective_locale, EN);
    assert_eq!(german.effective_locale, DE);
}

#[test]
fn effect_amounts_are_carried_verbatim() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let rendered = catalog
        .render(&request(EN, "card", "strike"))
        .expect("english");
    assert_eq!(
        rendered.segments,
        vec![
            LocaleRenderedSegment::Text("Deal ".to_owned()),
            LocaleRenderedSegment::Effect {
                kind: "damage".to_owned(),
                amount: "6".to_owned(),
            },
            LocaleRenderedSegment::Text(" damage.".to_owned()),
        ]
    );
}

#[test]
fn repeated_renders_are_deterministic_and_read_only() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest, baseline_entries());
    let first = catalog
        .render(&request(EN, "card", "strike"))
        .expect("first");
    let second = catalog
        .render(&request(EN, "card", "strike"))
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(catalog.entry_count(), 2);
    assert_eq!(catalog.variant_count("card", "strike"), 2);
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
}

#[test]
fn reference_segments_keep_language_independent_identity() {
    let manifest = base_manifest();
    let mut entries = baseline_entries();
    entries.push(entry(
        "relic",
        "burning_blood",
        EN,
        LocalePluralCategory::Other,
        vec![
            text("Heal equal to "),
            LocaleTextSegment::Reference(reference("card", "strike")),
            text("."),
        ],
        &[],
    ));
    let catalog = catalog(&manifest, entries);
    let rendered = catalog
        .render(&request(EN, "relic", "burning_blood"))
        .expect("relic");
    let referenced = rendered
        .segments
        .iter()
        .find_map(|segment| match segment {
            LocaleRenderedSegment::Reference(reference) => Some(reference.clone()),
            _ => None,
        })
        .expect("reference segment");
    assert_eq!(referenced.entity_kind, "card");
    assert_eq!(referenced.namespaced_id, "strike");
}
