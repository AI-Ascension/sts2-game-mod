// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/progression_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ProgressionCatalog, ProgressionDomain, ProgressionListQuery, ProgressionProfileKind,
    ProgressionProfilePermit, ProgressionProfileQuery, ProgressionReadState,
    ProgressionReferenceError, ProgressionRevision, ProgressionVisibilityScope,
    SaveProfileBaseline, UserDataIdentity,
};

fn projected() -> [ProgressionDomain; 2] {
    [ProgressionDomain::Unlocks, ProgressionDomain::Compendium]
}

fn entries() -> Vec<sts2_game_mod::ProgressionEntryInput> {
    vec![
        unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks),
        entry(
            "compendium_entry:ember",
            ProgressionDomain::Compendium,
            ProgressionReadState::Undiscovered,
        ),
    ]
}

fn snapshot_for(
    manifest: &sts2_game_mod::ContentManifest,
    identity: &str,
    revision: u64,
) -> sts2_game_mod::ProgressionCatalogSnapshot {
    snapshot(
        manifest,
        profile(identity, ProgressionProfileKind::Active, revision),
        &projected(),
        entries(),
    )
}

fn catalog_for(
    manifest: &sts2_game_mod::ContentManifest,
    identity: &str,
    revision: u64,
) -> ProgressionCatalog {
    let port = FixturePort::new(snapshot_for(manifest, identity, revision));
    produce(manifest, &port).expect("catalog")
}

fn query(revision: Option<ProgressionRevision>) -> ProgressionListQuery {
    ProgressionListQuery {
        locale: "en-US".to_owned(),
        profile: ProgressionProfileQuery::Active,
        scope: ProgressionVisibilityScope::Reference,
        revision,
        domain: None,
        state: None,
        limit: 64,
        continuation: None,
    }
}

#[test]
fn two_profiles_stay_isolated_and_their_references_do_not_cross() {
    let manifest = manifest(&[]);
    let first = catalog_for(&manifest, "profile:one", 3);
    let second = catalog_for(&manifest, "profile:two", 3);
    assert_ne!(first.profile().user_data_id, second.profile().user_data_id);
    let page = second.list(&query(None)).expect("second page");
    let reference = page.entries[0].reference.clone();
    assert_eq!(
        first
            .get(&reference, ProgressionVisibilityScope::Reference, None)
            .expect_err("cross-profile reference"),
        ProgressionReferenceError::StaleReference
    );
    assert_eq!(first.revision().user_data_id.as_str(), "profile:one");
    assert_eq!(second.revision().user_data_id.as_str(), "profile:two");
}

#[test]
fn a_profile_revision_change_invalidates_a_stale_query() {
    let manifest = manifest(&[]);
    let catalog = catalog_for(&manifest, "profile:one", 4);
    let current = catalog.revision();
    assert_eq!(current.revision, 4);
    assert!(catalog.list(&query(Some(current.clone()))).is_ok());
    let stale = ProgressionRevision {
        user_data_id: current.user_data_id.clone(),
        revision: 3,
    };
    assert_eq!(
        catalog
            .list(&query(Some(stale)))
            .expect_err("stale revision"),
        ProgressionReferenceError::ProfileRevisionMismatch
    );
    let page = catalog.list(&query(None)).expect("page");
    let reference = page.entries[0].reference.clone();
    let older = ProgressionRevision {
        user_data_id: current.user_data_id.clone(),
        revision: 99,
    };
    let detail = sts2_game_mod::ProgressionDetailQuery {
        entry: reference,
        profile: ProgressionProfileQuery::Active,
        scope: ProgressionVisibilityScope::Reference,
        revision: Some(older),
    };
    assert_eq!(
        catalog.reader().get(&detail).expect_err("stale detail"),
        ProgressionReferenceError::ProfileRevisionMismatch
    );
}

#[test]
fn naming_another_identity_is_refused_rather_than_answered_by_switching() {
    let manifest = manifest(&[]);
    let catalog = catalog_for(&manifest, "profile:one", 1);
    let mut request = query(None);
    request.profile = ProgressionProfileQuery::Named {
        user_data_id: UserDataIdentity::new("profile:two").expect("identity"),
        kind: ProgressionProfileKind::Active,
    };
    assert_eq!(
        catalog.list(&request).expect_err("implicit switch"),
        ProgressionReferenceError::ImplicitProfileSwitch
    );
    let mut matching = query(None);
    matching.profile = ProgressionProfileQuery::Named {
        user_data_id: UserDataIdentity::new("profile:one").expect("identity"),
        kind: ProgressionProfileKind::Active,
    };
    assert!(catalog.list(&matching).is_ok());
}

#[test]
fn an_offline_or_foreign_profile_requires_an_explicitly_supported_port() {
    let manifest = manifest(&[]);
    let catalog = catalog_for(&manifest, "profile:one", 1);
    for kind in [
        ProgressionProfileKind::Offline,
        ProgressionProfileKind::Foreign,
    ] {
        let mut request = query(None);
        request.profile = ProgressionProfileQuery::Named {
            user_data_id: UserDataIdentity::new("profile:one").expect("identity"),
            kind,
        };
        assert_eq!(
            catalog.list(&request).expect_err("kind"),
            ProgressionReferenceError::ForeignProfileRequiresReadPort
        );
    }
    let port = FixturePort::new(snapshot(
        &manifest,
        profile("profile:one", ProgressionProfileKind::Offline, 1),
        &projected(),
        entries(),
    ));
    assert_eq!(
        produce(&manifest, &port).expect_err("offline snapshot"),
        ProgressionReferenceError::ForeignProfileRequiresReadPort
    );
}

#[test]
fn a_profile_without_explicit_permission_is_never_published() {
    let manifest = manifest(&[]);
    let mut input = profile("profile:one", ProgressionProfileKind::Active, 1);
    input.permit = ProgressionProfilePermit::NotPermitted;
    let port = FixturePort::new(snapshot(&manifest, input, &projected(), entries()));
    assert_eq!(
        produce(&manifest, &port).expect_err("unpermitted"),
        ProgressionReferenceError::ProfileNotPermitted
    );
}

#[test]
fn a_freshness_witness_for_another_identity_is_refused() {
    let manifest = manifest(&[]);
    let mut input = profile("profile:one", ProgressionProfileKind::Active, 1);
    input.baseline = SaveProfileBaseline::new(
        UserDataIdentity::new("profile:two").expect("identity"),
        "digest:two",
        1,
    )
    .expect("baseline");
    let port = FixturePort::new(snapshot(&manifest, input, &projected(), entries()));
    assert_eq!(
        produce(&manifest, &port).expect_err("baseline"),
        ProgressionReferenceError::ProfileBaselineMismatch
    );
}
