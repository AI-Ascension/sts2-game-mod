// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use sts2_game_mod::{
    CoopError, CoopFamilyCoverage, CoopFamilyState, CoopLiveFence, CoopPartyInput,
    CoopPeerListQuery, CoopPeerRole, CoopReadAuthority, CoopReadScope, CoopScalingKind,
};
use support::*;

const PAGE: usize = sts2_game_mod::COOP_MAX_PAGE_ITEMS;

fn query(
    catalog: &sts2_game_mod::CoopCatalog,
    scope: CoopReadScope,
    limit: usize,
) -> CoopPeerListQuery {
    CoopPeerListQuery {
        locale: catalog.locale().to_owned(),
        party: party_reference(catalog),
        scope,
        limit,
        continuation: None,
        live_fence: None,
    }
}

/// A party of the given size, with exactly one local member and the rest allies.
fn party_of(size: usize) -> CoopPartyInput {
    let mut party = fixture_party();
    party.peers = (0..size)
        .map(|index| match index {
            0 => local_peer(),
            1 => ally_peer(),
            other => {
                let mut peer = ally_peer();
                peer.peer_id = format!("peer.{other:03}");
                peer.role = CoopPeerRole::Ally;
                peer.generation = other as u64 + 1;
                peer
            }
        })
        .collect();
    party
}

#[test]
fn one_page_lists_every_member_the_scope_observes_and_reports_completion() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list_peers(&query(&catalog, CoopReadScope::Peer, PAGE))
        .expect("page");
    assert_eq!(page.total, 2);
    assert_eq!(page.entries.len(), 2);
    assert!(page.complete);
    assert!(page.continuation.is_none());
    assert_eq!(page.authority, CoopReadAuthority::NotGranted);
    assert_eq!(page.binding, *catalog.binding());
    assert_eq!(page.party.peer_count, 2);
    assert_eq!(page.party.observed_peer_count, 2);
}

#[test]
fn the_party_scope_lists_only_the_members_it_may_observe() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list_peers(&query(&catalog, CoopReadScope::Party, PAGE))
        .expect("page");
    assert_eq!(page.total, 1);
    assert_eq!(page.entries.len(), 1);
    assert_eq!(page.entries[0].role, CoopPeerRole::Ally);
    assert_eq!(page.party.observed_peer_count, 1);
    assert_eq!(
        page.party.peer_count, 2,
        "the declared size is stated even where the scope observes fewer"
    );
}

#[test]
fn a_walk_of_bounded_pages_reaches_every_member_exactly_once() {
    let mut snapshot = fixture_snapshot();
    snapshot.party = party_of(9);
    snapshot.family.peer_count = 9;
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let mut reader = catalog.reader();
    let mut seen: Vec<String> = Vec::new();
    let mut request = query(&catalog, CoopReadScope::Peer, 2);
    loop {
        let page = reader.list_peers(&request).expect("page");
        assert!(page.entries.len() <= 2);
        seen.extend(
            page.entries
                .iter()
                .map(|entry| entry.reference.peer_id.clone()),
        );
        match page.continuation {
            Some(continuation) => {
                request.continuation = Some(continuation);
            }
            None => break,
        }
    }
    assert_eq!(seen.len(), 9);
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 9, "a page walk must not repeat a member");
}

#[test]
fn a_continuation_is_single_use_and_bound_to_the_query_that_minted_it() {
    let mut snapshot = fixture_snapshot();
    snapshot.party = party_of(9);
    snapshot.family.peer_count = 9;
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let mut reader = catalog.reader();
    let mut request = query(&catalog, CoopReadScope::Peer, 2);
    let page = reader.list_peers(&request).expect("page");
    let continuation = page.continuation.expect("partial page");
    request.continuation = Some(continuation.clone());
    assert!(
        reader.list_peers(&request).is_ok(),
        "the first use is served"
    );
    request.continuation = Some(continuation);
    assert_eq!(
        reader.list_peers(&request).expect_err("reused"),
        CoopError::InvalidContinuation
    );
}

#[test]
fn a_continuation_cannot_carry_a_walk_into_another_scope_or_page_size() {
    let mut snapshot = fixture_snapshot();
    snapshot.party = party_of(9);
    snapshot.family.peer_count = 9;
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let mut reader = catalog.reader();
    let request = query(&catalog, CoopReadScope::Peer, 2);
    let page = reader.list_peers(&request).expect("page");
    let continuation = page.continuation.expect("partial page");
    let mut switched = query(&catalog, CoopReadScope::Party, 2);
    switched.continuation = Some(continuation.clone());
    assert_eq!(
        reader.list_peers(&switched).expect_err("scope switch"),
        CoopError::InvalidContinuation
    );
    let mut resized = query(&catalog, CoopReadScope::Peer, 3);
    resized.continuation = Some(continuation);
    assert_eq!(
        reader.list_peers(&resized).expect_err("page size switch"),
        CoopError::InvalidContinuation
    );
}

#[test]
fn a_continuation_from_one_reader_is_refused_by_another() {
    let mut snapshot = fixture_snapshot();
    snapshot.party = party_of(9);
    snapshot.family.peer_count = 9;
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let mut first = catalog.reader();
    let mut request = query(&catalog, CoopReadScope::Peer, 2);
    let continuation = first
        .list_peers(&request)
        .expect("page")
        .continuation
        .expect("partial page");
    let mut second = catalog.reader();
    request.continuation = Some(continuation);
    assert_eq!(
        second.list_peers(&request).expect_err("foreign reader"),
        CoopError::InvalidContinuation
    );
}

#[test]
fn a_page_size_of_zero_or_past_its_bound_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    for limit in [0, PAGE + 1] {
        assert_eq!(
            reader
                .list_peers(&query(&catalog, CoopReadScope::Peer, limit))
                .expect_err("page size"),
            CoopError::InvalidPageSize
        );
    }
    assert!(
        reader
            .list_peers(&query(&catalog, CoopReadScope::Peer, PAGE))
            .is_ok(),
        "the bound itself is servable"
    );
}

#[test]
fn a_list_read_carries_no_live_fence_and_refuses_one_that_is_supplied() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let mut request = query(&catalog, CoopReadScope::Peer, PAGE);
    request.live_fence = Some(CoopLiveFence {
        instance_id: "instance.alpha".to_owned(),
        party_id: "party.alpha".to_owned(),
        epoch: 41,
    });
    assert_eq!(
        reader
            .list_peers(&request)
            .expect_err("fence on a retained read"),
        CoopError::UnexpectedLiveFence
    );
}

#[test]
fn a_list_locale_or_party_from_another_binding_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let mut request = query(&catalog, CoopReadScope::Peer, PAGE);
    request.locale = "de-DE".to_owned();
    assert_eq!(
        reader.list_peers(&request).expect_err("locale"),
        CoopError::LocaleMismatch
    );
    let mut stale = query(&catalog, CoopReadScope::Peer, PAGE);
    stale.party.party_id = "party.beta".to_owned();
    assert_eq!(
        reader.list_peers(&stale).expect_err("party"),
        CoopError::NotFound
    );
    let mut foreign = query(&catalog, CoopReadScope::Peer, PAGE);
    foreign.party.catalog.locale = "de-DE".to_owned();
    assert_eq!(
        reader.list_peers(&foreign).expect_err("binding"),
        CoopError::StaleReference
    );
}

#[test]
fn a_listed_entry_states_its_membership_and_freshness_not_only_its_identity() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list_peers(&query(&catalog, CoopReadScope::Peer, PAGE))
        .expect("page");
    let local = page
        .entries
        .iter()
        .find(|entry| entry.role == CoopPeerRole::Local)
        .expect("local entry");
    assert!(local.membership.carries_gameplay());
    assert!(local.freshness.is_current());
    assert!(local.character_id.is_present());
    assert_eq!(local.reference.party_id, "party.alpha");
    assert!(!local.reference.peer_id.is_empty());
}

#[test]
fn an_unavailable_family_lists_nothing_rather_than_an_empty_party() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = CoopFamilyCoverage {
        state: CoopFamilyState::Unavailable,
        peer_count: 0,
        effect_count: 0,
        scaling_count: 0,
    };
    snapshot.party = empty_party();
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let mut reader = catalog.reader();
    assert!(matches!(
        reader.list_peers(&query(&catalog, CoopReadScope::Peer, PAGE)),
        Err(CoopError::UnavailableFamily)
    ));
    assert!(matches!(reader.party(), Err(CoopError::UnavailableFamily)));
}

#[test]
fn a_listed_page_states_the_party_scaling_claims_it_does_not_flatten() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list_peers(&query(&catalog, CoopReadScope::Peer, PAGE))
        .expect("page");
    assert_eq!(page.party.effect_count, 2);
    assert_eq!(page.party.scaling_count, 3);
    let record = catalog.party().expect("party");
    assert!(
        record
            .party
            .scaling
            .iter()
            .any(|rule| rule.kind == CoopScalingKind::TargetAmplified)
    );
}
