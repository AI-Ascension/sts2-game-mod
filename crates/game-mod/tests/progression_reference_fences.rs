// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/progression_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentCursorBinding, ContentManifest, PROGRESSION_REFERENCE_PRODUCER_VERSION,
    ProgressionCatalog, ProgressionCatalogSnapshot, ProgressionDomain, ProgressionDomainState,
    ProgressionEntryInput, ProgressionField, ProgressionListQuery, ProgressionProfileKind,
    ProgressionProfileQuery, ProgressionReadState, ProgressionReferenceError,
    ProgressionVisibilityScope,
};

fn refuse(
    manifest: &ContentManifest,
    snapshot: ProgressionCatalogSnapshot,
) -> ProgressionReferenceError {
    produce(manifest, &FixturePort::new(snapshot)).expect_err("refused")
}

/// A canonical snapshot around one varied entry, so a coverage rule is the only thing broken.
fn refuse_entries(
    manifest: &ContentManifest,
    domains: &[ProgressionDomain],
    entries: Vec<ProgressionEntryInput>,
) -> ProgressionReferenceError {
    let snapshot = snapshot(
        manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        domains,
        entries,
    );
    refuse(manifest, snapshot)
}

fn catalog() -> (ContentManifest, ProgressionCatalog) {
    let manifest = manifest(&[]);
    let snapshot = canonical_snapshot(&manifest, 7);
    let catalog = produce(&manifest, &FixturePort::new(snapshot)).expect("catalog");
    (manifest, catalog)
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
fn a_snapshot_from_another_manifest_locale_or_producer_is_refused() {
    let manifest = manifest(&[]);

    let mut foreign_manifest = canonical_snapshot(&manifest, 7);
    foreign_manifest.manifest = ContentCursorBinding {
        catalog_generation: 99,
        ..foreign_manifest.manifest
    };
    assert_eq!(
        refuse(&manifest, foreign_manifest),
        ProgressionReferenceError::ManifestMismatch
    );

    let mut foreign_locale = canonical_snapshot(&manifest, 7);
    foreign_locale.locale = "fr-FR".to_owned();
    assert_eq!(
        refuse(&manifest, foreign_locale),
        ProgressionReferenceError::LocaleMismatch
    );

    let mut foreign_producer = canonical_snapshot(&manifest, 7);
    foreign_producer.producer_version = format!("{PROGRESSION_REFERENCE_PRODUCER_VERSION}-other");
    assert_eq!(
        refuse(&manifest, foreign_producer),
        ProgressionReferenceError::ProducerVersionMismatch
    );
}

#[test]
fn a_snapshot_past_the_entry_bound_is_refused() {
    let manifest = manifest(&[]);
    let entry = unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    let snapshot = snapshot(
        &manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        &PROJECTED_DOMAINS,
        vec![entry; 4_097],
    );
    assert_eq!(
        refuse(&manifest, snapshot),
        ProgressionReferenceError::InvalidInput("entries")
    );
}

#[test]
fn every_domain_states_exactly_one_coverage_row() {
    let manifest = manifest(&[]);

    let mut dropped = canonical_snapshot(&manifest, 7);
    dropped
        .domains
        .retain(|row| row.domain != ProgressionDomain::AccountScoped);
    assert_eq!(
        refuse(&manifest, dropped),
        ProgressionReferenceError::DomainCoverageIncomplete
    );

    let mut repeated = canonical_snapshot(&manifest, 7);
    let extra = repeated
        .domains
        .iter()
        .find(|row| row.domain == ProgressionDomain::Unlocks)
        .cloned()
        .expect("row");
    repeated.domains.push(extra);
    assert_eq!(
        refuse(&manifest, repeated),
        ProgressionReferenceError::DomainCoverageIncomplete
    );
}

#[test]
fn a_declared_entry_count_must_match_what_the_catalog_observes() {
    let manifest = manifest(&[]);
    let mut snapshot = canonical_snapshot(&manifest, 7);
    for row in &mut snapshot.domains {
        if row.domain == ProgressionDomain::Unlocks {
            row.entry_count += 1;
        }
    }
    assert_eq!(
        refuse(&manifest, snapshot),
        ProgressionReferenceError::DomainCountMismatch
    );
}

#[test]
fn an_account_scoped_domain_is_never_projected() {
    let manifest = manifest(&[]);
    let snapshot = snapshot(
        &manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        &[ProgressionDomain::Unlocks, ProgressionDomain::AccountScoped],
        vec![unlocked_entry(
            "unlock:act_four",
            ProgressionDomain::Unlocks,
        )],
    );
    assert_eq!(
        refuse(&manifest, snapshot),
        ProgressionReferenceError::AccountScopedDomainClaimed
    );
}

#[test]
fn an_entry_inside_a_domain_the_source_does_not_project_is_refused() {
    let manifest = manifest(&[]);
    let character = unlocked_entry("character:silent", ProgressionDomain::CharacterProgression);
    assert_eq!(
        refuse_entries(&manifest, &PROJECTED_DOMAINS, vec![character]),
        ProgressionReferenceError::UnavailableDomain
    );
}

#[test]
fn a_field_the_domain_declares_unsupported_is_never_reported_as_available() {
    let manifest = manifest(&[]);
    let mut snapshot = snapshot(
        &manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        &PROJECTED_DOMAINS,
        canonical_entries(),
    );
    for row in &mut snapshot.domains {
        if row.domain == ProgressionDomain::Unlocks {
            row.unsupported_fields = vec![ProgressionField::RelatedEntries];
        }
    }
    assert_eq!(
        refuse(&manifest, snapshot),
        ProgressionReferenceError::InconsistentField("related_entries")
    );
}

#[test]
fn a_domains_unsupported_field_list_stays_inside_its_bound() {
    let manifest = manifest(&[]);
    let mut snapshot = snapshot(
        &manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        &PROJECTED_DOMAINS,
        vec![unlocked_entry(
            "unlock:act_four",
            ProgressionDomain::Unlocks,
        )],
    );
    for row in &mut snapshot.domains {
        if row.domain == ProgressionDomain::Unlocks {
            row.unsupported_fields = vec![ProgressionField::RelatedEntries; 17];
        }
    }
    assert_eq!(
        refuse(&manifest, snapshot),
        ProgressionReferenceError::InvalidInput("unsupported_fields")
    );
}

#[test]
fn an_unavailable_domain_states_a_support_reason_rather_than_a_zero() {
    let (_manifest, catalog) = catalog();
    let character = catalog
        .domain_state(ProgressionDomain::CharacterProgression)
        .expect("row");
    assert_eq!(character.entry_count, 0);
    assert_eq!(character.state, ProgressionDomainState::Unavailable);
    assert_eq!(
        catalog
            .domain_state(ProgressionDomain::Unlocks)
            .expect("row")
            .entry_count,
        1
    );
    assert!(
        catalog
            .domains()
            .iter()
            .all(|row| row.unsupported_fields.len() <= 16)
    );
}

#[test]
fn a_page_size_outside_its_bound_is_refused() {
    let (_manifest, catalog) = catalog();

    let mut empty = query();
    empty.limit = 0;
    assert_eq!(
        catalog.list(&empty).expect_err("zero"),
        ProgressionReferenceError::InvalidPageSize
    );

    let mut oversized = query();
    oversized.limit = 65;
    assert_eq!(
        catalog.list(&oversized).expect_err("oversized"),
        ProgressionReferenceError::InvalidPageSize
    );
}

#[test]
fn a_continuation_is_single_use_and_bound_to_its_own_reader() {
    let (_manifest, catalog) = catalog();
    let mut paged = query();
    paged.limit = 4;

    let mut reader = catalog.reader();
    let first = reader.list(&paged).expect("first page");
    let token = first.continuation.clone().expect("continuation");
    assert!(first.is_partial());
    paged.continuation = Some(token.clone());
    let second = reader.list(&paged).expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert!(second.complete);

    paged.continuation = Some(token.clone());
    assert_eq!(
        reader.list(&paged).expect_err("reused"),
        ProgressionReferenceError::InvalidContinuation
    );

    let mut foreign = query();
    foreign.limit = 2;
    foreign.continuation = Some(token);
    assert_eq!(
        catalog.reader().list(&foreign).expect_err("foreign"),
        ProgressionReferenceError::InvalidContinuation
    );
}

#[test]
fn a_page_the_result_does_not_fit_into_requires_a_retained_reader() {
    let (_manifest, catalog) = catalog();
    let mut bounded = query();
    bounded.limit = 4;
    assert_eq!(
        catalog.list(&bounded).expect_err("partial"),
        ProgressionReferenceError::PartialPageRequiresReader
    );
    assert!(
        catalog.list(&query()).is_ok(),
        "a complete page is still served by the convenience"
    );
}

#[test]
fn a_filter_for_a_domain_the_source_does_not_project_is_refused() {
    let (_manifest, catalog) = catalog();
    let mut request = query();
    request.domain = Some(ProgressionDomain::CharacterProgression);
    assert_eq!(
        catalog.reader().list(&request).expect_err("unavailable"),
        ProgressionReferenceError::UnavailableDomain
    );
}

#[test]
fn a_read_keeps_the_four_non_held_states_apart() {
    let (_manifest, catalog) = catalog();
    let page = catalog.list(&query()).expect("page");
    let states = page
        .entries
        .iter()
        .map(|entry| entry.read_state)
        .collect::<Vec<ProgressionReadState>>();
    for state in [
        ProgressionReadState::Unlocked,
        ProgressionReadState::Locked,
        ProgressionReadState::Undiscovered,
        ProgressionReadState::NotTracked,
    ] {
        assert!(states.contains(&state), "state {state:?} is observed once");
    }
    assert_eq!(page.total, states.len());
    assert!(page.complete);

    let mut filtered = query();
    filtered.state = Some(ProgressionReadState::Locked);
    let locked = catalog.list(&filtered).expect("filtered");
    assert_eq!(locked.entries.len(), 1);
    assert_eq!(
        locked.entries[0].reference.entry_id,
        "achievement:ascendancy"
    );
}
