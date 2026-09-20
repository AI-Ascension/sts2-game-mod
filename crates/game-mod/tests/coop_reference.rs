// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use sts2_game_mod::{
    COOP_REFERENCE_PRODUCER_VERSION, CoopCatalog, CoopCatalogBinding, CoopFamilyState, CoopField,
    CoopFieldStatus, CoopLiveFence, CoopPeerFreshness, CoopPeerMembership, CoopPeerRole,
    CoopPileKind, CoopReadScope, CoopUnit,
};
use support::*;

fn peer(catalog: &CoopCatalog, peer_id: &str, scope: CoopReadScope) -> sts2_game_mod::CoopPeerView {
    catalog
        .peer(&peer_reference(catalog, peer_id), scope)
        .expect("peer")
}

#[test]
fn producing_binds_the_party_to_one_manifest_locale_and_producer() {
    let (manifest, catalog) = fixture_catalog();
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        COOP_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(catalog.family().state, CoopFamilyState::Handled);
    assert_eq!(catalog.family().peer_count, 2);
    assert_eq!(catalog.family().effect_count, 2);
    assert_eq!(catalog.family().scaling_count, 3);
}

#[test]
fn every_gameplay_field_states_its_own_availability() {
    let (_manifest, catalog) = fixture_catalog();
    let local = peer(&catalog, "peer.local", CoopReadScope::Peer);
    for field in CoopField::ALL {
        assert!(
            matches!(local.status(field), CoopFieldStatus::Present),
            "{field:?} should be present for the local member"
        );
    }
    assert_eq!(local.status(CoopField::Character), CoopFieldStatus::Present);
    assert_eq!(
        local.character_id.value().map(String::as_str),
        Some("character.ironclad")
    );
    assert_eq!(local.piles.len(), CoopPileKind::ALL.len());
}

#[test]
fn every_pile_states_its_own_visibility_rather_than_one_party_answer() {
    let (_manifest, catalog) = fixture_catalog();
    let local = peer(&catalog, "peer.local", CoopReadScope::Peer);
    for pile in &local.piles {
        assert_eq!(
            pile.visibility,
            pile.kind.default_visibility(),
            "{:?}",
            pile.kind
        );
        assert!(pile.contents.is_present(), "{:?}", pile.kind);
    }
    assert!(CoopPileKind::Hand.is_local());
    assert!(!CoopPileKind::Discard.is_local());
}

#[test]
fn an_ally_pile_that_is_local_only_is_refused_rather_than_emptied() {
    let (_manifest, catalog) = fixture_catalog();
    let ally = peer(&catalog, "peer.ally", CoopReadScope::Peer);
    let hand = ally
        .piles
        .iter()
        .find(|pile| pile.kind == CoopPileKind::Hand)
        .expect("hand row");
    assert!(hand.contents.is_not_permitted());
    assert_eq!(hand.contents.status(), CoopFieldStatus::NotPermitted);
    assert!(hand.contents.value().is_none());
    let discard = ally
        .piles
        .iter()
        .find(|pile| pile.kind == CoopPileKind::Discard)
        .expect("discard row");
    assert!(discard.contents.is_present());
}

#[test]
fn local_only_fields_never_project_to_an_ally() {
    let (_manifest, catalog) = fixture_catalog();
    let ally = peer(&catalog, "peer.ally", CoopReadScope::Peer);
    assert_eq!(
        ally.status(CoopField::Potions),
        CoopFieldStatus::NotPermitted
    );
    assert!(ally.potions.is_not_permitted());
    assert_eq!(ally.status(CoopField::Health), CoopFieldStatus::Present);
    assert_eq!(ally.status(CoopField::Powers), CoopFieldStatus::Present);
}

#[test]
fn the_local_member_is_absent_from_the_party_scope_it_shares() {
    let (_manifest, catalog) = fixture_catalog();
    let shared = catalog
        .current(
            &CoopLiveFence {
                instance_id: "instance.alpha".to_owned(),
                party_id: "party.alpha".to_owned(),
                epoch: 1,
            },
            CoopReadScope::Party,
        )
        .expect("shared view");
    assert_eq!(shared.scope, CoopReadScope::Party);
    assert_eq!(shared.peers.len(), 1);
    assert_eq!(shared.peers[0].peer_id, "peer.ally");
    assert_eq!(shared.peers[0].role, CoopPeerRole::Ally);
    assert_eq!(shared.effects.len(), 2);
    assert_eq!(shared.scaling.len(), 3);
}

#[test]
fn a_local_only_value_does_not_ride_along_with_a_shared_one() {
    let (_manifest, catalog) = fixture_catalog();
    let shared = catalog.party().expect("party").viewed(CoopReadScope::Party);
    assert_eq!(shared.peers.len(), 1);
    assert!(shared.peers[0].relics.is_present());
    assert!(shared.peers[0].potions.is_not_permitted());
    assert!(
        shared.peers[0]
            .piles
            .iter()
            .any(|pile| pile.kind == CoopPileKind::Hand && pile.contents.is_not_permitted())
    );
}

#[test]
fn membership_and_freshness_stay_distinguishable_from_absence() {
    let (_manifest, catalog) = fixture_catalog();
    let local = peer(&catalog, "peer.local", CoopReadScope::Peer);
    assert!(local.membership.carries_gameplay());
    assert!(local.freshness.is_current());
    assert!(!CoopPeerMembership::Joining.carries_gameplay());
    assert!(!CoopPeerMembership::Disconnected.carries_gameplay());
    assert!(!CoopPeerMembership::Left.carries_gameplay());
    assert!(CoopPeerMembership::Active.carries_gameplay());
    assert!(CoopPeerFreshness::Current.is_current());
    assert!(!CoopPeerFreshness::Lagging.is_current());
    assert!(!CoopPeerFreshness::Unavailable.is_current());
}

#[test]
fn scaling_claims_are_three_separate_rules_rather_than_one_number() {
    let (_manifest, catalog) = fixture_catalog();
    let party = catalog.party().expect("party");
    let kinds = party
        .party
        .scaling
        .iter()
        .map(|rule| rule.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&sts2_game_mod::CoopScalingKind::SharedPool));
    assert!(kinds.contains(&sts2_game_mod::CoopScalingKind::PerPeer));
    assert!(
        kinds.contains(&sts2_game_mod::CoopScalingKind::TargetAmplified),
        "target amplification is its own claim"
    );
}

#[test]
fn an_empty_family_is_explicit_rather_than_a_party_of_zero_members() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = sts2_game_mod::CoopFamilyCoverage {
        state: CoopFamilyState::Unavailable,
        peer_count: 0,
        effect_count: 0,
        scaling_count: 0,
    };
    snapshot.party = empty_party();
    let catalog = produce_snapshot(snapshot).expect("catalog");
    assert_eq!(catalog.family().state, CoopFamilyState::Unavailable);
    assert!(matches!(
        catalog.party(),
        Err(sts2_game_mod::CoopError::UnavailableFamily)
    ));
}

#[test]
fn a_declared_unavailable_family_carrying_a_party_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = sts2_game_mod::CoopFamilyCoverage {
        state: CoopFamilyState::Unavailable,
        peer_count: 2,
        effect_count: 2,
        scaling_count: 3,
    };
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(sts2_game_mod::CoopError::InvalidInput("party"))
    ));
}

#[test]
fn a_reference_carries_the_binding_it_was_produced_for() {
    let (_manifest, catalog) = fixture_catalog();
    let reference = peer_reference(&catalog, "peer.local");
    assert_eq!(reference.catalog, *catalog.binding());
    let other = CoopCatalogBinding {
        manifest: catalog.binding().manifest.clone(),
        locale: "fr-FR".to_owned(),
        producer_version: catalog.binding().producer_version.clone(),
    };
    assert_ne!(other, *catalog.binding());
    assert_eq!(
        CoopUnit {
            unit: "energy".to_owned()
        }
        .unit,
        "energy"
    );
}
