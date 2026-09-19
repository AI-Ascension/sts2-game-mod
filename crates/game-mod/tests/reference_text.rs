// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reference_text.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    PublicControlKind, PublicScreenKind, REFERENCE_TEXT_PRODUCER_VERSION, ReferenceDiscovery,
    ReferenceFamily, ReferenceListQuery,
};

#[test]
fn catalog_binds_the_manifest_and_producer_identity() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(
        catalog.binding().producer_version,
        REFERENCE_TEXT_PRODUCER_VERSION
    );
    assert_eq!(catalog.locale(), EN);
    assert_eq!(catalog.document_count(), 6);
    assert_eq!(catalog.screen_count(), 4);
    assert_eq!(
        catalog.screen_ids(),
        [
            "blocking_message".to_owned(),
            "settings_screen".to_owned(),
            "tutorial_overlay".to_owned(),
            "unknown_screen".to_owned(),
        ]
    );
}

#[test]
fn every_family_is_inventoried_or_reported_as_unsupported_scope() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let coverage = catalog.coverage();
    let rows: Vec<(ReferenceFamily, usize, usize)> = coverage
        .families
        .iter()
        .map(|row| (row.family, row.documents, row.sections))
        .collect();
    assert_eq!(
        rows,
        vec![
            (ReferenceFamily::Tutorial, 1, 1),
            (ReferenceFamily::Help, 1, 1),
            (ReferenceFamily::Lore, 1, 0),
            (ReferenceFamily::Credits, 1, 0),
            (ReferenceFamily::UiText, 1, 0),
            (ReferenceFamily::Unknown, 1, 0),
        ]
    );
    assert_eq!(
        coverage.unsupported_documents,
        vec!["legacy_secret".to_owned()]
    );
    assert_eq!(ReferenceFamily::ALL.len(), ReferenceFamily::COUNT);
    assert_eq!(ReferenceFamily::COUNT, 6);
}

#[test]
fn family_tokens_round_trip_and_state_which_families_are_projected() {
    for family in ReferenceFamily::ALL {
        assert_eq!(ReferenceFamily::parse(family.as_str()), Some(family));
    }
    assert_eq!(ReferenceFamily::parse("ship"), None);
    assert!(ReferenceFamily::Tutorial.is_inventoried());
    assert!(ReferenceFamily::UiText.is_inventoried());
    assert!(!ReferenceFamily::Unknown.is_inventoried());
}

#[test]
fn discovery_tokens_state_which_documents_are_readable() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let page = catalog
        .page(
            &ReferenceListQuery {
                page_size: 64,
                ..ReferenceListQuery::default()
            },
            None,
        )
        .expect("page");
    let lore = page
        .items
        .iter()
        .find(|item| item.namespaced_id == "lore_spire")
        .expect("lore row");
    assert_eq!(lore.discovery, ReferenceDiscovery::Locked);
    assert!(!lore.discovery.is_readable());
    let credits = page
        .items
        .iter()
        .find(|item| item.namespaced_id == "credits_team")
        .expect("credits row");
    assert_eq!(credits.discovery, ReferenceDiscovery::Discovered);
    assert!(credits.discovery.is_readable());
    for token in ["open", "discovered", "locked", "unknown"] {
        let parsed = ReferenceDiscovery::parse(token).expect("discovery token");
        assert_eq!(parsed.as_str(), token);
    }
    assert_eq!(ReferenceDiscovery::parse("sealed"), None);
    assert!(ReferenceDiscovery::Open.is_readable());
    assert!(!ReferenceDiscovery::Unknown.is_readable());
}

#[test]
fn screen_and_control_kinds_classify_and_flag_private_input() {
    assert_eq!(PublicScreenKind::COUNT, 5);
    for token in [
        "modal",
        "tutorial_overlay",
        "blocking_message",
        "confirmation",
        "unknown",
    ] {
        let parsed = PublicScreenKind::parse(token).expect("screen kind");
        assert_eq!(parsed.as_str(), token);
    }
    assert_eq!(PublicScreenKind::parse("tooltip"), None);
    assert!(PublicScreenKind::Modal.is_classified());
    assert!(!PublicScreenKind::Unknown.is_classified());

    for token in ["button", "choice", "text_input", "unknown"] {
        let parsed = PublicControlKind::parse(token).expect("control kind");
        assert_eq!(parsed.as_str(), token);
    }
    assert_eq!(PublicControlKind::parse("slider"), None);
    assert!(PublicControlKind::TextInput.carries_private_input());
    assert!(!PublicControlKind::Button.carries_private_input());
    assert!(!PublicControlKind::Choice.carries_private_input());
    assert!(!PublicControlKind::Unknown.carries_private_input());
}

#[test]
fn summary_rows_carry_a_stable_identity_and_their_keyword_index() {
    let manifest = base_manifest();
    let catalog = catalog(&manifest);
    let page = catalog
        .page(
            &ReferenceListQuery {
                page_size: 64,
                ..ReferenceListQuery::default()
            },
            None,
        )
        .expect("page");
    let tutorial = page
        .items
        .iter()
        .find(|item| item.namespaced_id == "tutorial_combat")
        .expect("tutorial row");
    assert_eq!(tutorial.family, ReferenceFamily::Tutorial);
    assert_eq!(tutorial.revision, "text-tutorial_combat");
    assert_eq!(tutorial.section_count, 1);
    assert_eq!(
        tutorial.keywords,
        vec![
            "combat".to_owned(),
            "turn".to_owned(),
            "tutorial".to_owned()
        ]
    );
}
