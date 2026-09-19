// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reference_text.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    PublicControlKind, PublicScreenKind, ReferenceCompleteness, ReferenceDiscovery,
    ReferenceFamily, ReferenceTextError, ReferenceTextSegment, ReferenceUnavailableReason,
    ScreenReadEffect,
};

fn failure(family: ReferenceFamily, id: &str) -> ReferenceTextError {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    catalog
        .read_document(family, id)
        .expect_err("must not be found")
}

#[test]
fn a_blocking_message_is_readable_without_dismissing_it() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let read = catalog.read_screen("blocking_message").expect("screen");
    assert_eq!(read.screen_kind, PublicScreenKind::BlockingMessage);
    assert!(read.blocking);
    assert!(!read.dismissible);
    assert_eq!(read.completeness, ReferenceCompleteness::Complete);
    assert_eq!(read.effect, ScreenReadEffect::None);
    assert_eq!(read.locale, EN);
    assert_eq!(read.binding, *catalog.binding());
    assert_eq!(
        read.visible_text,
        vec![text("The Spire calls."), link("card", "strike")]
    );
    assert_eq!(read.controls.len(), 1);
    assert_eq!(read.controls[0].control_id, "acknowledge");
    assert!(read.controls[0].available);
    assert!(!read.controls[0].value_withheld);
    assert_eq!(read.withheld_private_inputs, 0);
}

#[test]
fn reading_a_public_screen_has_exactly_one_possible_effect() {
    // `ScreenReadEffect` is a single-variant type, so an exhaustive match is total: reading can
    // never click, confirm or dismiss, and there is no second variant to reach.
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    for screen_id in [
        "tutorial_overlay",
        "blocking_message",
        "settings_screen",
        "unknown_screen",
    ] {
        let read = catalog.read_screen(screen_id).expect("screen");
        let effect = match read.effect {
            ScreenReadEffect::None => "none",
        };
        assert_eq!(effect, "none");
    }
}

#[test]
fn a_dismissible_overlay_is_read_with_its_formatting_preserved() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let overlay = catalog.read_screen("tutorial_overlay").expect("overlay");
    assert!(overlay.dismissible);
    assert!(!overlay.blocking);
    assert_eq!(
        overlay.visible_text,
        vec![text("Move with the arrow keys.")]
    );

    let document = catalog
        .read_document(ReferenceFamily::Tutorial, "tutorial_combat")
        .expect("tutorial");
    assert_eq!(document.title, vec![text("Combat basics")]);
    assert_eq!(document.completeness, ReferenceCompleteness::Complete);
    assert_eq!(document.discovery, ReferenceDiscovery::Open);
    assert!(document.retained_bytes > 0);
    assert_eq!(document.sections.len(), 1);
    assert_eq!(
        document.sections[0].segments,
        vec![
            text("Play a card."),
            ReferenceTextSegment::LineBreak,
            emphasis("End turn."),
        ]
    );
}

#[test]
fn a_locked_document_withholds_its_text_with_an_explicit_reason() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let read = catalog
        .read_document(ReferenceFamily::Lore, "lore_spire")
        .expect("lore");
    assert_eq!(read.completeness, ReferenceCompleteness::Partial);
    assert_eq!(read.discovery, ReferenceDiscovery::Locked);
    assert_eq!(
        read.title,
        vec![ReferenceTextSegment::Unavailable(
            ReferenceUnavailableReason::Locked
        )]
    );
    assert!(read.sections.is_empty());
    assert_eq!(read.retained_bytes, 0);
}

#[test]
fn an_unprojected_family_reports_unsupported_scope() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let read = catalog
        .read_document(ReferenceFamily::Unknown, "legacy_secret")
        .expect("legacy");
    assert_eq!(read.completeness, ReferenceCompleteness::Unsupported);
    assert_eq!(
        read.title,
        vec![ReferenceTextSegment::Unavailable(
            ReferenceUnavailableReason::UnsupportedScope
        )]
    );
}

#[test]
fn an_unclassified_screen_reports_unsupported_scope() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let read = catalog.read_screen("unknown_screen").expect("screen");
    assert_eq!(read.screen_kind, PublicScreenKind::Unknown);
    assert_eq!(read.completeness, ReferenceCompleteness::Unsupported);
    assert_eq!(read.effect, ScreenReadEffect::None);
}

#[test]
fn private_input_values_are_withheld_rather_than_read() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let read = catalog.read_screen("settings_screen").expect("screen");
    assert_eq!(read.withheld_private_inputs, 1);
    assert_eq!(read.controls[0].kind, PublicControlKind::TextInput);
    assert!(read.controls[0].value_withheld);
    assert_eq!(read.controls[1].kind, PublicControlKind::Button);
    assert!(!read.controls[1].value_withheld);
    assert!(!read.controls[1].available);
    assert_eq!(
        read.controls[1].unavailable_reason,
        Some(ReferenceUnavailableReason::PrivateInput)
    );
}

#[test]
fn a_discovered_document_is_readable_in_full() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let read = catalog
        .read_document(ReferenceFamily::Credits, "credits_team")
        .expect("credits");
    assert!(read.discovery.is_readable());
    assert_eq!(read.completeness, ReferenceCompleteness::Complete);
    assert_eq!(read.title, vec![text("Credits")]);
}

#[test]
fn absent_references_are_reported_as_not_found() {
    assert_eq!(
        failure(ReferenceFamily::Tutorial, "tutorial_combat "),
        ReferenceTextError::InvalidInput("namespaced_id")
    );
    assert_eq!(
        failure(ReferenceFamily::Tutorial, "tutorial_missing"),
        ReferenceTextError::NotFound
    );
    assert_eq!(
        failure(ReferenceFamily::Help, "tutorial_combat"),
        ReferenceTextError::NotFound
    );
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    assert_eq!(
        catalog.read_screen("missing_screen").expect_err("missing"),
        ReferenceTextError::NotFound
    );
    assert_eq!(
        catalog.read_screen("").expect_err("empty"),
        ReferenceTextError::InvalidInput("screen_id")
    );
}
