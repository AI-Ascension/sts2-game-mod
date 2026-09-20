// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use manifest_fixture::quantity;
use sts2_game_mod::{
    CoopEffectScope, CoopError, CoopFamilyCoverage, CoopFamilyState, CoopFieldValue,
    CoopPartyInput, CoopPeerRole, CoopReadScope, CoopScalingKind, CoopTargeting, CoopTurnPhase,
    CoopVote,
};
use support::*;

/// Produces the shared fixture with one party-wide change and returns the refusal.
fn refuse_party_with(mutate: impl FnOnce(&mut CoopPartyInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party);
    produce_snapshot(snapshot).expect_err("must be refused")
}

/// One otherwise-coherent vote naming the given members, so one field can be made incoherent.
fn vote(vote_id: &str, decided_peer_ids: Vec<String>) -> CoopVote {
    CoopVote {
        vote_id: vote_id.to_owned(),
        phase: CoopTurnPhase::Combat,
        choices: CoopFieldValue::absent(),
        decided_peer_ids,
        resolved: false,
    }
}

/// One targeting relationship, so a self-target or a dangling one can be measured against it.
fn targeting(source_peer_id: &str, target_peer_id: &str) -> CoopTargeting {
    CoopTargeting {
        source_peer_id: source_peer_id.to_owned(),
        target_peer_id: target_peer_id.to_owned(),
    }
}

#[test]
fn a_targeted_effect_must_name_a_member_and_a_party_effect_must_not() {
    assert!(matches!(
        refuse_party_with(|party| {
            party.effects[1].target_peer_id = CoopFieldValue::present("peer.absent".to_owned());
        }),
        CoopError::UnknownPeerTarget(_)
    ));
    assert!(matches!(
        refuse_party_with(|party| {
            party.effects[1].target_peer_id = CoopFieldValue::absent();
        }),
        CoopError::UnresolvedEffectTarget(_)
    ));
    assert!(matches!(
        refuse_party_with(|party| {
            party.effects[0].target_peer_id = CoopFieldValue::present("peer.ally".to_owned());
        }),
        CoopError::EffectScopeConflict(_)
    ));
}

#[test]
fn a_scaling_rule_must_match_the_shape_of_its_own_kind() {
    assert!(matches!(
        refuse_party_with(|party| {
            party.scaling[0].per_peer = CoopFieldValue::present(quantity("points", 2));
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
    assert!(matches!(
        refuse_party_with(|party| {
            party.scaling[1].per_peer = CoopFieldValue::absent();
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
    assert!(matches!(
        refuse_party_with(|party| {
            party.scaling[2].per_peer = CoopFieldValue::present(quantity("points", 1));
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
    assert!(matches!(
        refuse_party_with(|party| {
            party.scaling[2].effect_id = CoopFieldValue::absent();
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
    assert!(
        matches!(
            refuse_party_with(|party| {
                party.scaling[2].effect_id =
                    CoopFieldValue::present("effect.not_carried".to_owned());
            }),
            CoopError::UnknownManifestReference { .. }
        ),
        "a target the manifest does not carry is refused before the party is asked about it"
    );
}

#[test]
fn an_amplified_rule_naming_an_effect_the_party_does_not_carry_is_refused() {
    let mut entries = manifest_fixture::FIXTURE_ENTRIES.to_vec();
    entries.push(("effect", "effect.elsewhere"));
    let manifest = manifest_fixture::manifest(&entries);
    let mut snapshot = fixture_snapshot();
    snapshot.manifest = manifest.cursor_binding();
    snapshot.party.scaling[2].effect_id = CoopFieldValue::present("effect.elsewhere".to_owned());
    assert!(matches!(
        produce_with(&manifest, snapshot).expect_err("uncarried effect"),
        CoopError::ScalingTargetMismatch(_)
    ));
}

#[test]
fn a_repeated_effect_or_scaling_identity_is_refused() {
    assert!(matches!(
        refuse_party_with(|party| {
            let duplicate = party.effects[0].clone();
            party.effects.push(duplicate);
        }),
        CoopError::DuplicateEffect(_)
    ));
    assert!(matches!(
        refuse_party_with(|party| {
            let duplicate = party.scaling[0].clone();
            party.scaling.push(duplicate);
        }),
        CoopError::DuplicateScalingRule(_)
    ));
}

#[test]
fn a_collection_past_its_local_bound_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.family.peer_count = sts2_game_mod::COOP_MAX_PEERS + 1;
    snapshot.party.peers = (0..=sts2_game_mod::COOP_MAX_PEERS)
        .map(|index| {
            let mut peer = if index == 0 {
                local_peer()
            } else {
                ally_peer()
            };
            peer.peer_id = format!("peer.{index:02}");
            if index > 0 {
                peer.role = CoopPeerRole::Ally;
            }
            peer
        })
        .collect();
    assert_eq!(
        produce_snapshot(snapshot).expect_err("too many members"),
        CoopError::InvalidInput("peers")
    );
}

#[test]
fn a_party_past_its_aggregate_byte_bound_is_refused_with_the_measured_size() {
    let mut snapshot = fixture_snapshot();
    let filler = "x".repeat(sts2_game_mod::COOP_MAX_TEXT_BYTES - 3);
    for peer in &mut snapshot.party.peers {
        peer.display_name = CoopFieldValue::present(filler.clone());
        peer.resources = CoopFieldValue::present(
            (0..sts2_game_mod::COOP_MAX_RESOURCES)
                .map(|index| sts2_game_mod::CoopResource {
                    resource_id: format!("{filler}{index:02}"),
                    amount: quantity("energy", 1),
                })
                .collect(),
        );
    }
    let refused = produce_snapshot(snapshot).expect_err("too large");
    assert!(matches!(refused, CoopError::PartyTooLarge { .. }));
    let CoopError::PartyTooLarge { limit, actual } = refused else {
        return;
    };
    assert_eq!(limit, sts2_game_mod::COOP_MAX_PARTY_BYTES);
    assert!(actual > limit, "measured {actual} must exceed {limit}");
}

#[test]
fn a_declared_unavailable_family_must_carry_no_party_at_all() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = CoopFamilyCoverage {
        state: CoopFamilyState::Unavailable,
        peer_count: 0,
        effect_count: 0,
        scaling_count: 0,
    };
    assert_eq!(
        produce_snapshot(snapshot).expect_err("party under an unavailable family"),
        CoopError::InvalidInput("party")
    );
}

#[test]
fn the_fixture_party_is_the_shape_these_refusals_are_measured_against() {
    let (_manifest, catalog) = fixture_catalog();
    let view = catalog
        .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
        .expect("ally view");
    assert_eq!(view.role, CoopPeerRole::Ally);
    assert!(view.potions.is_not_permitted());
    assert_eq!(
        CoopEffectScope::Targeted,
        catalog.party().expect("party").party.effects[1].scope
    );
    assert_eq!(
        CoopScalingKind::SharedPool,
        catalog.party().expect("party").party.scaling[0].kind
    );
}

#[test]
fn two_members_sharing_one_membership_generation_are_refused() {
    assert_eq!(
        refuse_party_with(|party| party.peers[1].generation = party.peers[0].generation),
        CoopError::DuplicatePeerGeneration("peer.ally".to_owned())
    );
    let mut snapshot = fixture_snapshot();
    snapshot.party.peers[1].generation = snapshot.party.peers[0].generation + 1;
    assert!(
        produce_snapshot(snapshot).is_ok(),
        "a rejoin's incremented generation is what the rule admits"
    );
}

#[test]
fn a_vote_in_a_phase_that_admits_none_is_refused() {
    assert_eq!(
        refuse_party_with(|party| {
            party.context.votes = CoopFieldValue::present(vec![CoopVote {
                phase: CoopTurnPhase::Terminal,
                ..vote("vote.target", Vec::new())
            }]);
        }),
        CoopError::VoteOutsideVotingPhase("terminal".to_owned()),
        "a terminal party is choosing nothing, so its vote cannot resolve"
    );
    assert_eq!(
        refuse_party_with(|party| party.context.phase = CoopTurnPhase::Terminal),
        CoopError::VoteOutsideVotingPhase("vote.target".to_owned()),
        "a vote that admits itself but contradicts the party's own phase is refused by name"
    );
}

#[test]
fn a_vote_naming_another_phase_or_a_member_the_party_does_not_carry_is_refused() {
    assert_eq!(
        refuse_party_with(|party| {
            party.context.votes = CoopFieldValue::present(vec![CoopVote {
                phase: CoopTurnPhase::Interlude,
                ..vote("vote.target", Vec::new())
            }]);
        }),
        CoopError::VoteOutsideVotingPhase("interlude".to_owned()),
        "a vote stating a phase that admits none is refused by that phase's name"
    );
    assert_eq!(
        refuse_party_with(|party| {
            party.context.votes =
                CoopFieldValue::present(vec![vote("vote.target", vec!["peer.absent".to_owned()])]);
        }),
        CoopError::InvalidVote("vote.target".to_owned()),
        "a vote may not name a member the party does not declare"
    );
}

#[test]
fn a_repeated_vote_identity_or_a_repeated_decider_is_refused() {
    assert_eq!(
        refuse_party_with(|party| {
            party.context.votes = CoopFieldValue::present(vec![
                vote("vote.one", Vec::new()),
                vote("vote.one", Vec::new()),
            ]);
        }),
        CoopError::InvalidVote("vote.one".to_owned())
    );
    assert_eq!(
        refuse_party_with(|party| {
            party.context.votes = CoopFieldValue::present(vec![vote(
                "vote.one",
                vec!["peer.ally".to_owned(), "peer.ally".to_owned()],
            )]);
        }),
        CoopError::InvalidVote("vote.one".to_owned()),
        "one member signalling twice is not two members deciding"
    );
}

#[test]
fn a_targeting_relationship_that_names_itself_or_a_stranger_is_refused() {
    assert_eq!(
        refuse_party_with(|party| {
            party.context.targeting =
                CoopFieldValue::present(vec![targeting("peer.local", "peer.local")]);
        }),
        CoopError::InvalidTargeting("peer.local".to_owned()),
        "a member cannot aim at itself"
    );
    assert_eq!(
        refuse_party_with(|party| {
            party.context.targeting =
                CoopFieldValue::present(vec![targeting("peer.local", "peer.absent")]);
        }),
        CoopError::InvalidTargeting("peer.local".to_owned()),
        "a relationship to a member the party does not carry is refused"
    );
    assert_eq!(
        refuse_party_with(|party| {
            party.context.targeting = CoopFieldValue::present(vec![
                targeting("peer.local", "peer.ally"),
                targeting("peer.local", "peer.ally"),
            ]);
        }),
        CoopError::InvalidTargeting("peer.local".to_owned()),
        "the same relationship stated twice is a repeat, not a second edge"
    );
}
