// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use manifest_fixture::{fixture_manifest, manifest};
use sts2_game_mod::{
    CoopEntryKind, CoopError, CoopFamilyCoverage, CoopFieldValue, CoopPartyInput, CoopPeerInput,
    CoopPeerRole, CoopPileKind, CoopPileView,
};
use support::*;

/// Produces the shared fixture with one member replaced and returns the refusal.
fn refuse_peer_with(mutate: impl FnOnce(&mut CoopPeerInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party.peers[1]);
    produce_snapshot(snapshot).expect_err("must be refused")
}

/// Produces the shared fixture with one party-wide change and returns the refusal.
fn refuse_party_with(mutate: impl FnOnce(&mut CoopPartyInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party);
    produce_snapshot(snapshot).expect_err("must be refused")
}

#[test]
fn a_snapshot_for_another_manifest_locale_or_producer_is_refused() {
    let manifest = fixture_manifest();
    let cases: [fn(&mut sts2_game_mod::CoopCatalogSnapshot); 3] = [
        |declared| declared.manifest = other_of(),
        |declared| declared.locale = "de-DE".to_owned(),
        |declared| declared.producer_version = "game-coop-reference-producer-v0".to_owned(),
    ];
    let expected = [
        CoopError::ManifestMismatch,
        CoopError::LocaleMismatch,
        CoopError::ProducerVersionMismatch,
    ];
    for (case, expected) in cases.into_iter().zip(expected) {
        let mut declared = fixture_snapshot();
        case(&mut declared);
        let error = produce_with(&manifest, declared).expect_err("fence mismatch");
        assert_eq!(error, expected);
    }
}

/// Returns another manifest's binding, so a mismatched fence can be injected.
fn other_of() -> sts2_game_mod::ContentCursorBinding {
    manifest(&[("card", "card.strike")]).cursor_binding()
}

#[test]
fn declared_coverage_must_match_the_party_the_source_reports() {
    for mutate in [
        (|family: &mut CoopFamilyCoverage| family.peer_count = 1) as fn(&mut CoopFamilyCoverage),
        |family: &mut CoopFamilyCoverage| family.effect_count = 1,
        |family: &mut CoopFamilyCoverage| family.scaling_count = 4,
    ] {
        let mut snapshot = fixture_snapshot();
        mutate(&mut snapshot.family);
        assert_eq!(
            produce_snapshot(snapshot).expect_err("count mismatch"),
            CoopError::FamilyCountMismatch
        );
    }
}

#[test]
fn a_party_identity_that_aliases_its_instance_or_run_is_refused() {
    assert_eq!(
        refuse_party_with(|party| party.party_id = party.instance_id.clone()),
        CoopError::IdentityNamespaceCollision("party_id")
    );
    assert_eq!(
        refuse_party_with(|party| party.party_id = party.run_id.clone()),
        CoopError::IdentityNamespaceCollision("party_id")
    );
}

#[test]
fn a_path_shaped_identity_is_refused_rather_than_sanitized() {
    for mutate in [
        (|party: &mut CoopPartyInput| party.party_id = "/save/profile".to_owned())
            as fn(&mut CoopPartyInput),
        |party: &mut CoopPartyInput| party.instance_id = r"C:\saves".to_owned(),
        |party: &mut CoopPartyInput| party.run_id = "..".to_owned(),
    ] {
        assert!(matches!(
            refuse_party_with(mutate),
            CoopError::NonOpaqueIdentity(_)
        ));
    }
}

#[test]
fn exactly_one_local_member_is_required() {
    assert_eq!(
        refuse_party_with(|party| party.peers = vec![ally_peer_only()]),
        CoopError::MissingLocalPeer
    );
    assert!(matches!(
        refuse_party_with(|party| {
            let mut second = party.peers[0].clone();
            second.peer_id = "peer.local.two".to_owned();
            second.generation = 3;
            party.peers.push(second);
        }),
        CoopError::MultipleLocalPeers(_)
    ));
}

/// One party whose only member is an ally, so no member could own the local-only fields.
fn ally_peer_only() -> CoopPeerInput {
    let mut peer = ally_peer();
    peer.role = CoopPeerRole::Ally;
    peer
}

#[test]
fn a_repeated_member_identity_is_refused() {
    assert!(matches!(
        refuse_party_with(|party| {
            let mut duplicate = party.peers[1].clone();
            duplicate.role = CoopPeerRole::Ally;
            party.peers.push(duplicate);
        }),
        CoopError::DuplicatePeer(_)
    ));
}

#[test]
fn a_value_carried_by_a_member_without_a_coherent_snapshot_is_refused() {
    for membership in [
        sts2_game_mod::CoopPeerMembership::Joining,
        sts2_game_mod::CoopPeerMembership::Disconnected,
        sts2_game_mod::CoopPeerMembership::Left,
    ] {
        assert!(matches!(
            refuse_peer_with(|peer| peer.membership = membership),
            CoopError::GameplayForInactivePeer(_)
        ));
    }
    for freshness in [
        sts2_game_mod::CoopPeerFreshness::Lagging,
        sts2_game_mod::CoopPeerFreshness::Unavailable,
    ] {
        assert!(matches!(
            refuse_peer_with(|peer| peer.freshness = freshness),
            CoopError::ValueWithoutCurrentFreshness(_)
        ));
    }
    let error = refuse_peer_with(|peer| {
        peer.freshness = sts2_game_mod::CoopPeerFreshness::Lagging;
        peer.potions = CoopFieldValue::stale();
        peer.health = CoopFieldValue::absent();
    });
    assert!(
        matches!(error, CoopError::ValueWithoutCurrentFreshness(_)),
        "a field without a value is not a value, so the present one is still the refusal"
    );
}

#[test]
fn a_local_only_value_on_an_ally_is_refused_even_when_it_is_labeled() {
    assert_eq!(
        refuse_peer_with(|peer| {
            peer.role = CoopPeerRole::Ally;
            peer.potions = CoopFieldValue::present(vec![manifest_fixture::entry(
                CoopEntryKind::Potion,
                "potion.fire",
                1,
                "Fire Potion",
            )]);
        }),
        CoopError::LocalOnlyValueOnAlly("potions".to_owned())
    );
}

#[test]
fn a_local_only_pile_published_to_an_ally_is_refused_rather_than_emptied() {
    assert_eq!(
        refuse_peer_with(|peer| {
            peer.role = CoopPeerRole::Ally;
            peer.piles = ally_piles_reported();
        }),
        CoopError::LocalPilePublishedToAlly("hand".to_owned())
    );
}

/// The ally's pile rows with the local-only piles reported instead of refused.
fn ally_piles_reported() -> Vec<CoopPileView> {
    vec![
        CoopPileView::not_permitted(CoopPileKind::Draw),
        CoopPileView::reported(CoopPileKind::Hand, 1, vec!["card.bash".to_owned()]),
        CoopPileView::reported(CoopPileKind::Discard, 1, vec!["card.strike".to_owned()]),
        CoopPileView::reported(CoopPileKind::Exhaust, 0, Vec::new()),
        CoopPileView::reported(CoopPileKind::Play, 0, Vec::new()),
    ]
}

#[test]
fn a_pile_whose_declared_visibility_contradicts_its_kind_is_refused() {
    assert_eq!(
        refuse_peer_with(
            |peer| peer.piles[1].visibility = sts2_game_mod::CoopFieldVisibility::PublicToParty
        ),
        CoopError::PileVisibilityMismatch("hand".to_owned())
    );
}

#[test]
fn a_missing_or_repeated_pile_kind_is_refused() {
    assert_eq!(
        refuse_peer_with(|peer| {
            peer.piles.pop();
        }),
        CoopError::InvalidInput("piles")
    );
    assert!(matches!(
        refuse_peer_with(|peer| {
            peer.piles[3] =
                CoopPileView::reported(CoopPileKind::Hand, 1, vec!["card.bash".to_owned()]);
        }),
        CoopError::DuplicatePile(_)
    ));
    assert!(
        matches!(
            refuse_peer_with(|peer| {
                peer.piles.push(CoopPileView::reported(
                    CoopPileKind::Hand,
                    1,
                    vec!["card.bash".to_owned()],
                ));
            }),
            CoopError::InvalidInput("piles")
        ),
        "a sixth row is past the pile bound before it can repeat a kind"
    );
}

#[test]
fn a_present_collection_that_is_empty_states_nothing_and_is_refused() {
    assert_eq!(
        refuse_peer_with(|peer| peer.relics = CoopFieldValue::present(Vec::new())),
        CoopError::EmptyPresentCollection("relics")
    );
    assert_eq!(
        refuse_peer_with(|peer| peer.resources = CoopFieldValue::present(Vec::new())),
        CoopError::EmptyPresentCollection("resources")
    );
    assert_eq!(
        refuse_party_with(|party| party.peers = Vec::new()),
        CoopError::EmptyPresentCollection("peers")
    );
}

#[test]
fn impossible_health_and_zero_counts_are_refused_by_name() {
    assert!(matches!(
        refuse_peer_with(|peer| peer.health = manifest_fixture::health(90, 80)),
        CoopError::ImpossibleHealth(_)
    ));
    assert!(matches!(
        refuse_peer_with(|peer| peer.health = manifest_fixture::health(-1, 80)),
        CoopError::ImpossibleHealth(_)
    ));
    assert!(matches!(
        refuse_peer_with(|peer| {
            peer.relics = CoopFieldValue::present(vec![manifest_fixture::entry(
                CoopEntryKind::Relic,
                "relic.burning_blood",
                0,
                "Burning Blood",
            )]);
        }),
        CoopError::ZeroCountEntry(_)
    ));
}

#[test]
fn an_entry_of_another_kind_or_a_dangling_reference_is_refused() {
    assert_eq!(
        refuse_peer_with(|peer| {
            peer.relics = CoopFieldValue::present(vec![manifest_fixture::entry(
                CoopEntryKind::Potion,
                "relic.burning_blood",
                1,
                "Burning Blood",
            )]);
        }),
        CoopError::InvalidInput("relics")
    );
    assert!(matches!(
        refuse_peer_with(|peer| {
            peer.relics = CoopFieldValue::present(vec![manifest_fixture::entry(
                CoopEntryKind::Relic,
                "relic.not_in_manifest",
                1,
                "Unknown",
            )]);
        }),
        CoopError::UnknownManifestReference { .. }
    ));
}
