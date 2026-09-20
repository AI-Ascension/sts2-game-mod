// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use sts2_game_mod::{CoopError, CoopPartyInput, CoopPeerReference, CoopReadScope};
use support::*;

/// Produces the shared fixture with one party-wide change and returns the refusal.
fn refuse_party_with(mutate: impl FnOnce(&mut CoopPartyInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party);
    produce_snapshot(snapshot).expect_err("must be refused")
}

#[test]
fn a_member_reference_minted_before_a_rejoin_is_refused_rather_than_resolved() {
    let (_manifest, catalog) = fixture_catalog();
    let mut stale = peer_reference(&catalog, "peer.ally");
    assert_eq!(stale.generation, 2);
    stale.generation = 1;
    assert_eq!(
        catalog
            .peer(&stale, CoopReadScope::Peer)
            .expect_err("pre-rejoin reference"),
        CoopError::StalePeerGeneration("peer.ally".to_owned()),
        "a rejoin starts a new generation, so the old reference names a member that is gone"
    );
    assert!(
        catalog
            .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
            .is_ok(),
        "the current generation still resolves"
    );
}

#[test]
fn a_rejoin_increments_the_generation_and_the_previous_reference_stops_resolving() {
    let mut snapshot = fixture_snapshot();
    let before = snapshot.party.peers[1].generation;
    snapshot.party.peers[1].generation = before + 1;
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let reference = peer_reference(&catalog, "peer.ally");
    assert_eq!(
        reference.generation,
        before + 1,
        "a reference names the generation it was minted for"
    );
    assert!(catalog.peer(&reference, CoopReadScope::Peer).is_ok());
    let rejoined = CoopPeerReference {
        generation: before,
        ..reference
    };
    assert_eq!(
        catalog
            .peer(&rejoined, CoopReadScope::Peer)
            .expect_err("the rejoined member's old reference"),
        CoopError::StalePeerGeneration("peer.ally".to_owned())
    );
}

#[test]
fn two_members_of_one_party_may_not_share_a_membership_generation() {
    assert!(
        matches!(
            refuse_party_with(|party| {
                let generation = party.peers[0].generation;
                party.peers[1].generation = generation;
            }),
            CoopError::DuplicatePeerGeneration(_)
        ),
        "a shared generation would make a generation fence unable to tell the members apart"
    );
    assert!(
        matches!(
            refuse_party_with(|party| {
                let mut duplicate = party.peers[1].clone();
                duplicate.peer_id = "peer.ally.two".to_owned();
                party.peers.push(duplicate);
            }),
            CoopError::DuplicatePeerGeneration(_)
        ),
        "and two distinct members may not repeat one generation either"
    );
}
