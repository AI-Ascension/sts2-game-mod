// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/progression_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ProgressionCatalog, ProgressionDomain, ProgressionDomainState, ProgressionEntryReference,
    ProgressionListQuery, ProgressionProfileKind, ProgressionProfileQuery,
    ProgressionReadAuthority, ProgressionReadState, ProgressionReferenceError,
    ProgressionVisibilityScope,
};

fn fixture() -> (sts2_game_mod::ContentManifest, FixturePort) {
    fixture_at(7)
}

/// Same synthetic source read, taken at one exact profile revision.
fn fixture_at(revision: u64) -> (sts2_game_mod::ContentManifest, FixturePort) {
    let manifest = manifest(&["card"]);
    (
        manifest.clone(),
        FixturePort::new(canonical_snapshot(&manifest, revision)),
    )
}

fn catalog() -> (ProgressionCatalog, FixturePort) {
    let (manifest, port) = fixture();
    let catalog = produce(&manifest, &port).expect("catalog");
    (catalog, port)
}

fn query() -> ProgressionListQuery {
    ProgressionListQuery {
        locale: "en-US".to_owned(),
        profile: ProgressionProfileQuery::Active,
        scope: ProgressionVisibilityScope::Reference,
        revision: None,
        domain: None,
        state: None,
        limit: 64,
        continuation: None,
    }
}

#[test]
fn produces_a_fenced_catalog_from_one_source_read() {
    let (manifest, port) = fixture();
    let catalog = produce(&manifest, &port).expect("catalog");
    assert_eq!(port.reads(), 1);
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(catalog.profile().user_data_id.as_str(), "profile:one");
    assert_eq!(catalog.profile().kind, ProgressionProfileKind::Active);
    assert_eq!(catalog.revision().revision, 7);
    assert_eq!(
        catalog.revision().user_data_id,
        catalog.profile().user_data_id
    );
}

#[test]
fn states_one_coverage_row_per_inventoried_domain() {
    let (catalog, _port) = catalog();
    assert_eq!(catalog.domains().len(), ProgressionDomain::all().len());
    for domain in ProgressionDomain::all() {
        let row = catalog.domain_state(domain).expect("coverage row");
        assert_eq!(row.domain, domain);
    }
    assert_eq!(
        catalog
            .domain_state(ProgressionDomain::AccountScoped)
            .expect("row")
            .state,
        ProgressionDomainState::Unsupported
    );
    assert_eq!(
        catalog
            .domain_state(ProgressionDomain::CharacterProgression)
            .expect("row")
            .state,
        ProgressionDomainState::Unavailable
    );
}

#[test]
fn lists_every_visible_entry_once_in_stable_identity_order() {
    let (catalog, _port) = catalog();
    let page = catalog.list(&query()).expect("page");
    let ids = page
        .entries
        .iter()
        .map(|entry| entry.reference.entry_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "achievement:ascendancy",
            "achievement:first_blood",
            "best_record:fastest_floor",
            "compendium_entry:ember",
            "statistic:runs_started",
            "unlock:act_four",
        ]
    );
    assert_eq!(page.total, 6);
    assert!(page.complete);
    assert!(!page.is_partial());
    assert_eq!(page.authority, ProgressionReadAuthority::NotGranted);
    assert_eq!(page.revision, catalog.revision());
    assert_eq!(page.binding, *catalog.binding());
}

#[test]
fn pages_partially_and_never_labels_a_partial_page_complete() {
    let (catalog, _port) = catalog();
    let mut reader = catalog.reader();
    let mut request = query();
    request.limit = 4;
    let first = reader.list(&request).expect("first page");
    assert_eq!(first.entries.len(), 4);
    assert_eq!(first.total, 6);
    assert!(!first.complete);
    assert!(first.is_partial());
    let continuation = first.continuation.expect("continuation");
    request.continuation = Some(continuation.clone());
    let second = reader.list(&request).expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert_eq!(second.total, 6);
    assert!(second.complete);
    assert!(second.continuation.is_none());
    request.continuation = Some(continuation);
    assert_eq!(
        reader.list(&request).expect_err("reused continuation"),
        ProgressionReferenceError::InvalidContinuation
    );
}

#[test]
fn filters_by_domain_and_by_read_state() {
    let (catalog, _port) = catalog();
    let mut request = query();
    request.domain = Some(ProgressionDomain::Achievements);
    let page = catalog.list(&request).expect("domain page");
    assert_eq!(page.total, 2);
    request.state = Some(ProgressionReadState::Locked);
    let page = catalog.list(&request).expect("state page");
    assert_eq!(page.total, 1);
    assert_eq!(page.entries[0].reference.entry_id, "achievement:ascendancy");
    assert_eq!(page.entries[0].stated_requirements, 1);
}

#[test]
fn reads_one_exact_entry_with_its_requirements_and_content_references() {
    let (catalog, _port) = catalog();
    let page = catalog.list(&query()).expect("page");
    let locked = page
        .entries
        .iter()
        .find(|entry| entry.reference.entry_id == "achievement:ascendancy")
        .expect("locked entry");
    let entry = catalog
        .get(
            &locked.reference,
            ProgressionVisibilityScope::Reference,
            Some(&catalog.revision()),
        )
        .expect("detail");
    assert_eq!(entry.read_state, ProgressionReadState::Locked);
    let requirements = entry.requirements.value().expect("stated requirements");
    assert_eq!(requirements.len(), 1);
    assert_eq!(
        requirements[0]
            .content
            .as_ref()
            .expect("content")
            .entity_kind,
        "unlock"
    );
    assert_eq!(
        entry.field_status(sts2_game_mod::ProgressionField::Progress),
        Some(sts2_game_mod::ProgressionFieldStatus::Available)
    );
}

#[test]
fn keeps_an_untracked_value_out_of_the_result_rather_than_reporting_zero() {
    let (catalog, _port) = catalog();
    let page = catalog.list(&query()).expect("page");
    let stat = page
        .entries
        .iter()
        .find(|entry| entry.reference.entry_id == "statistic:runs_started")
        .expect("statistic");
    assert_eq!(stat.read_state, ProgressionReadState::NotTracked);
    assert!(stat.progress.value().is_none());
    assert_eq!(
        stat.progress.status(),
        sts2_game_mod::ProgressionFieldStatus::NotObserved
    );
    assert!(stat.description.value().is_some());
}

#[test]
fn refuses_an_unknown_identity_and_a_reference_from_another_catalog() {
    let (catalog, _port) = catalog();
    let missing = ProgressionEntryReference {
        catalog: catalog.binding().clone(),
        entry_id: "unlock:absent".to_owned(),
    };
    assert_eq!(
        catalog
            .get(&missing, ProgressionVisibilityScope::Reference, None)
            .expect_err("missing"),
        ProgressionReferenceError::NotFound
    );
    let (other_manifest, other_port) = fixture_at(9);
    let other = produce(&other_manifest, &other_port).expect("other catalog");
    assert_ne!(other.binding(), catalog.binding());
    let mut foreign = missing;
    foreign.catalog = other.binding().clone();
    assert_eq!(
        catalog
            .get(&foreign, ProgressionVisibilityScope::Reference, None)
            .expect_err("stale"),
        ProgressionReferenceError::StaleReference
    );
}

#[test]
fn refuses_a_locale_the_catalog_was_not_read_in() {
    let (catalog, _port) = catalog();
    let mut request = query();
    request.locale = "fr-FR".to_owned();
    assert_eq!(
        catalog.list(&request).expect_err("locale"),
        ProgressionReferenceError::LocaleMismatch
    );
}
