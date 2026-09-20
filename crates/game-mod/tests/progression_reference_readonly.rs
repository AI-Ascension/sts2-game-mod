// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/progression_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ProgressionDomain, ProgressionListQuery, ProgressionProfileKind, ProgressionProfileQuery,
    ProgressionReadAuthority, ProgressionReadAvailability, ProgressionReadPort,
    ProgressionReadState, ProgressionReferenceError, ProgressionSourceError,
    ProgressionVisibilityScope, UnavailableProgressionHost,
};

fn locked_catalog() -> (
    sts2_game_mod::ContentManifest,
    sts2_game_mod::ProgressionCatalog,
    FixturePort,
) {
    let manifest = manifest(&[]);
    let mut hidden = locked_entry("unlock:hidden", ProgressionDomain::Unlocks);
    hidden.visibility = sts2_game_mod::ProgressionVisibility::Hidden;
    let mut visible = locked_entry("unlock:visible", ProgressionDomain::Unlocks);
    visible.visibility = sts2_game_mod::ProgressionVisibility::OwnerOnly;
    let snapshot = snapshot(
        &manifest,
        unnamed_profile(ProgressionProfileKind::Active, 2),
        &[ProgressionDomain::Unlocks],
        vec![hidden, visible],
    );
    let port = FixturePort::new(snapshot);
    let catalog = produce(&manifest, &port).expect("catalog");
    (manifest, catalog, port)
}

fn query(scope: ProgressionVisibilityScope) -> ProgressionListQuery {
    ProgressionListQuery {
        locale: "en-US".to_owned(),
        profile: ProgressionProfileQuery::Active,
        scope,
        revision: None,
        domain: None,
        state: None,
        limit: 64,
        continuation: None,
    }
}

#[test]
fn every_published_read_states_the_capability_it_withholds() {
    let (_manifest, catalog, _port) = locked_catalog();
    let reader_authority = catalog.reader().authority();
    let page = catalog
        .list(&query(ProgressionVisibilityScope::Owner))
        .expect("page");
    assert_eq!(reader_authority, ProgressionReadAuthority::NotGranted);
    assert_eq!(page.authority, ProgressionReadAuthority::NotGranted);
}

#[test]
fn repeated_reads_retain_no_state_and_never_move_the_profile() {
    let (_manifest, catalog, port) = locked_catalog();
    let before = catalog.binding().clone();
    let request = query(ProgressionVisibilityScope::Owner);
    let first = catalog.list(&request).expect("first");
    let second = catalog.list(&request).expect("second");
    let third = catalog.list(&request).expect("third");
    assert_eq!(first, second);
    assert_eq!(second, third);
    assert_eq!(catalog.binding(), &before);
    assert_eq!(catalog.profile().user_data_id.as_str(), "profile:one");
    assert_eq!(catalog.profile().kind, ProgressionProfileKind::Active);
    assert_eq!(port.reads(), 1);
}

#[test]
fn a_locked_entry_stays_locked_however_often_it_is_read() {
    let (_manifest, catalog, _port) = locked_catalog();
    let request = query(ProgressionVisibilityScope::Owner);
    for _ in 0..3 {
        let page = catalog.list(&request).expect("page");
        for summary in &page.entries {
            assert_eq!(summary.read_state, ProgressionReadState::Locked);
        }
    }
    let reader = catalog.reader();
    for summary in &catalog.list(&request).expect("page").entries {
        let entry = catalog
            .get(&summary.reference, ProgressionVisibilityScope::Owner, None)
            .expect("detail");
        assert_eq!(entry.read_state, ProgressionReadState::Locked);
        let requirements = entry.requirements.value().expect("requirements");
        assert_eq!(requirements[0].state, ProgressionReadState::Locked);
    }
    drop(reader);
}

#[test]
fn a_hidden_entry_is_unreachable_outside_the_owner_scope() {
    let (_manifest, catalog, _port) = locked_catalog();
    let reference_page = catalog
        .list(&query(ProgressionVisibilityScope::Owner))
        .expect("owner page");
    let hidden = reference_page
        .entries
        .iter()
        .find(|entry| entry.reference.entry_id == "unlock:hidden")
        .expect("hidden entry")
        .reference
        .clone();
    for scope in [
        ProgressionVisibilityScope::Public,
        ProgressionVisibilityScope::Reference,
    ] {
        assert_eq!(
            catalog.get(&hidden, scope, None).expect_err("excluded"),
            ProgressionReferenceError::ExcludedByScope
        );
        let ids = catalog
            .list(&query(scope))
            .expect("page")
            .entries
            .iter()
            .map(|entry| entry.reference.entry_id.clone())
            .collect::<Vec<_>>();
        assert!(!ids.contains(&"unlock:hidden".to_owned()));
    }
    let public = catalog
        .list(&query(ProgressionVisibilityScope::Public))
        .expect("public page");
    assert_eq!(public.total, 0);
    let reference = catalog
        .list(&query(ProgressionVisibilityScope::Reference))
        .expect("reference page");
    assert_eq!(reference.total, 1);
}

#[test]
fn the_host_boundary_fails_closed_until_native_reads_are_authorized() {
    let manifest = manifest(&[]);
    let host = UnavailableProgressionHost;
    assert_eq!(
        host.read_availability(),
        ProgressionReadAvailability::UnavailableHost
    );
    assert_eq!(
        produce(&manifest, &host).expect_err("no host"),
        ProgressionReferenceError::NoActiveSource
    );
}

#[test]
fn source_failures_are_sanitized_into_the_owned_error_set() {
    let manifest = manifest(&[]);
    for (source, expected) in [
        (
            ProgressionSourceError::AccessDenied,
            ProgressionReferenceError::SourceAccessDenied,
        ),
        (
            ProgressionSourceError::Malformed,
            ProgressionReferenceError::MalformedSource,
        ),
        (
            ProgressionSourceError::ProfileNotPermitted,
            ProgressionReferenceError::ProfileNotPermitted,
        ),
        (
            ProgressionSourceError::RevisionChanged,
            ProgressionReferenceError::ProfileRevisionMismatch,
        ),
    ] {
        let port = FixturePort::failing(source);
        assert_eq!(produce(&manifest, &port).expect_err("source"), expected);
        assert_eq!(port.reads(), 1);
    }
}
