// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/locale_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    LOCALE_REFERENCE_MAX_SEGMENTS, LOCALE_REFERENCE_MAX_TEXT_BYTES,
    LOCALE_REFERENCE_PRODUCER_VERSION, LocaleCatalogError, LocaleCatalogProducer, LocaleDirection,
    LocaleInput, LocalePluralCategory, LocaleTextSegment,
};

fn produce(
    manifest: &sts2_game_mod::ContentManifest,
    locales: Vec<LocaleInput>,
    entries: Vec<sts2_game_mod::LocaleEntryInput>,
) -> Result<sts2_game_mod::LocaleCatalog, LocaleCatalogError> {
    LocaleCatalogProducer::new().produce(
        manifest,
        &Source {
            snapshot: snapshot(manifest, locales, entries),
        },
    )
}

#[test]
fn a_duplicate_locale_is_rejected() {
    let manifest = base_manifest();
    let mut declared = locales();
    declared.push(LocaleInput {
        locale: EN.to_owned(),
        direction: LocaleDirection::LeftToRight,
        fallback: vec![EN.to_owned()],
    });
    assert_eq!(
        produce(&manifest, declared, baseline_entries()).expect_err("expected an error"),
        LocaleCatalogError::DuplicateLocale(EN.to_owned())
    );
}

#[test]
fn a_fallback_chain_that_does_not_end_at_the_default_locale_is_rejected() {
    let manifest = base_manifest();
    let mut declared = locales();
    declared[1].fallback = vec![DE.to_owned(), AR.to_owned()];
    assert_eq!(
        produce(&manifest, declared, baseline_entries()).expect_err("expected an error"),
        LocaleCatalogError::InvalidFallbackChain("unordered")
    );
}

#[test]
fn a_cyclic_fallback_chain_is_rejected() {
    let manifest = base_manifest();
    let mut declared = locales();
    declared[1].fallback = vec![DE.to_owned(), EN.to_owned(), DE.to_owned()];
    assert_eq!(
        produce(&manifest, declared, baseline_entries()).expect_err("expected an error"),
        LocaleCatalogError::InvalidFallbackChain("cyclic")
    );
}

#[test]
fn a_missing_default_locale_is_rejected() {
    let manifest = base_manifest();
    let mut declared = locales();
    declared[0].fallback = vec![EN.to_owned()];
    let error = LocaleCatalogProducer::new()
        .produce(
            &manifest,
            &Source {
                snapshot: sts2_game_mod::LocaleCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    producer_version: LOCALE_REFERENCE_PRODUCER_VERSION.to_owned(),
                    default_locale: "fr-FR".to_owned(),
                    locales: declared,
                    entries: baseline_entries(),
                },
            },
        )
        .expect_err("expected an error");
    assert_eq!(error, LocaleCatalogError::MissingDefaultLocale);
}

#[test]
fn a_producer_version_mismatch_is_rejected() {
    let manifest = base_manifest();
    let error = LocaleCatalogProducer::new()
        .produce(
            &manifest,
            &Source {
                snapshot: sts2_game_mod::LocaleCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    producer_version: "game-locale-reference-producer-v0".to_owned(),
                    default_locale: EN.to_owned(),
                    locales: locales(),
                    entries: baseline_entries(),
                },
            },
        )
        .expect_err("expected an error");
    assert_eq!(error, LocaleCatalogError::ProducerVersionMismatch);
}

#[test]
fn a_manifest_mismatch_is_rejected() {
    let mismatched_manifest = base_manifest();
    let other = manifest(&[("card", "strike"), ("text", "shop_title")]);
    let error = LocaleCatalogProducer::new()
        .produce(
            &mismatched_manifest,
            &Source {
                snapshot: snapshot(&other, locales(), baseline_entries()),
            },
        )
        .expect_err("expected an error");
    assert_eq!(error, LocaleCatalogError::ManifestMismatch);
}

#[test]
fn a_duplicate_entry_is_rejected() {
    let manifest = base_manifest();
    let mut entries = baseline_entries();
    entries.push(entry(
        "card",
        "strike",
        EN,
        LocalePluralCategory::Other,
        vec![text("Deal 6 damage.")],
        &[],
    ));
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::DuplicateEntry {
            entity_kind: "card".to_owned(),
            namespaced_id: "strike".to_owned(),
            locale: EN.to_owned(),
            plural: LocalePluralCategory::Other,
        }
    );
}

#[test]
fn an_entry_for_a_definition_absent_from_the_manifest_is_rejected() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "absent",
        EN,
        LocalePluralCategory::Other,
        vec![text("Absent.")],
        &[],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "absent".to_owned(),
        }
    );
}

#[test]
fn a_reference_segment_to_an_absent_definition_is_rejected() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "strike",
        EN,
        LocalePluralCategory::Other,
        vec![
            text("Refers to "),
            LocaleTextSegment::Reference(reference("card", "absent")),
        ],
        &[],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "absent".to_owned(),
        }
    );
}

#[test]
fn an_unused_declared_placeholder_is_rejected() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "strike",
        EN,
        LocalePluralCategory::Other,
        vec![text("Deal 6 damage.")],
        &["amount"],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::InvalidInput("unused_placeholder")
    );
}

#[test]
fn an_undeclared_placeholder_segment_is_rejected() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "strike",
        EN,
        LocalePluralCategory::Other,
        vec![text("Deal "), placeholder("amount")],
        &[],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::InvalidInput("undeclared_placeholder")
    );
}

#[test]
fn an_effect_amount_without_a_rendered_number_is_rejected() {
    let manifest = base_manifest();
    let entries = vec![entry(
        "card",
        "strike",
        EN,
        LocalePluralCategory::Other,
        vec![effect("damage", "many")],
        &[],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::InvalidInput("effect_amount")
    );
}

#[test]
fn an_oversized_text_is_rejected() {
    let manifest = base_manifest();
    let oversized = "x".repeat(LOCALE_REFERENCE_MAX_TEXT_BYTES + 1);
    let entries = vec![entry(
        "text",
        "shop_title",
        EN,
        LocalePluralCategory::Other,
        vec![text(&oversized)],
        &[],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::TextTooLarge {
            limit: LOCALE_REFERENCE_MAX_TEXT_BYTES,
            actual: LOCALE_REFERENCE_MAX_TEXT_BYTES + 1,
        }
    );
}

#[test]
fn too_many_segments_are_rejected() {
    let manifest = base_manifest();
    let segments: Vec<LocaleTextSegment> = (0..=LOCALE_REFERENCE_MAX_SEGMENTS)
        .map(|_| text("x"))
        .collect();
    let entries = vec![entry(
        "text",
        "shop_title",
        EN,
        LocalePluralCategory::Other,
        segments,
        &[],
    )];
    assert_eq!(
        produce(&manifest, locales(), entries).expect_err("expected an error"),
        LocaleCatalogError::InvalidInput("segments")
    );
}
