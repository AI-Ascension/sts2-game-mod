// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use sts2_game_mod::{
    CoopError, CoopField, CoopFieldValue, CoopPartyContext, CoopPartyInput, CoopPeerInput,
    CoopPeerMembership, CoopPileView, CoopReadScope, CoopTargeting, CoopTurnPhase, CoopVote,
    CoopVoteChoice,
};
use support::*;

/// Produces the shared fixture with one party-wide change and returns the refusal.
fn refuse_party_with(mutate: impl FnOnce(&mut CoopPartyInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party);
    produce_snapshot(snapshot).expect_err("must be refused")
}

/// Produces the shared fixture with one context change and returns the refusal.
fn refuse_context_with(mutate: impl FnOnce(&mut CoopPartyContext)) -> CoopError {
    refuse_party_with(|party| mutate(&mut party.context))
}

/// Produces the shared fixture with one member change and returns the refusal.
fn refuse_peer_with(mutate: impl FnOnce(&mut CoopPeerInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party.peers[1]);
    produce_snapshot(snapshot).expect_err("must be refused")
}

/// One vote that repeats the fixture's own vote identity, so a mutation can target it by name.
fn fixture_vote() -> CoopVote {
    fixture_context()
        .votes
        .value()
        .expect("votes")
        .first()
        .expect("one vote")
        .clone()
}

#[test]
fn the_party_states_the_phase_step_votes_and_targeting_an_action_is_read_against() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    assert_eq!(record.party.context.phase, CoopTurnPhase::Selection);
    assert_eq!(record.party.context.step, 3);
    let votes = record.party.context.votes.value().expect("votes");
    assert_eq!(votes.len(), 1);
    assert_eq!(votes[0].vote_id, "vote.reward");
    assert!(!votes[0].resolved);
    let targeting = record.party.context.targeting.value().expect("targeting");
    assert_eq!(targeting.len(), 1);
    assert_eq!(targeting[0].source_peer_id, "peer.local");
    assert_eq!(targeting[0].target_peer_id, "peer.ally");
}

#[test]
fn a_vote_states_which_members_decided_but_not_what_any_of_them_picked() {
    let vote = fixture_vote();
    assert_eq!(vote.decided_peer_ids, vec!["peer.local".to_owned()]);
    assert!(!vote.resolved);
    let choices = vote.choices.value().expect("choices");
    assert_eq!(choices.len(), 2);
    assert!(
        choices.iter().all(|choice| !choice.choice_id.is_empty()),
        "every option states its own identity"
    );
    assert!(
        choices
            .iter()
            .any(|choice| choice.label.is_present() && choice.label.value().is_some()),
        "an option may state a label"
    );
    assert!(
        choices.iter().any(|choice| !choice.label.is_present()),
        "an option with no label states that rather than an empty string"
    );
}

#[test]
fn the_public_context_is_the_same_at_every_scope_that_observes_the_party() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let peer_view = record.viewed(CoopReadScope::Peer);
    let party_view = record.viewed(CoopReadScope::Party);
    assert_eq!(peer_view.context, party_view.context);
    assert_eq!(party_view.context, fixture_context());
}

#[test]
fn a_vote_in_a_phase_that_admits_none_is_refused() {
    for phase in [CoopTurnPhase::Interlude, CoopTurnPhase::Terminal] {
        assert!(!phase.admits_vote(), "{phase:?} admits no vote");
        assert_eq!(
            refuse_context_with(|context| {
                context.phase = phase;
                context.votes = CoopFieldValue::present(vec![CoopVote {
                    phase,
                    ..fixture_vote()
                }]);
            }),
            CoopError::VoteOutsideVotingPhase(phase.name().to_owned()),
            "{phase:?} admits no vote, so the party's own phase is the refusal"
        );
    }
    for phase in [CoopTurnPhase::Selection, CoopTurnPhase::Combat] {
        assert!(phase.admits_vote(), "{phase:?} admits a vote");
    }
}

#[test]
fn a_vote_whose_own_phase_contradicts_the_party_phase_is_refused() {
    assert_eq!(
        refuse_context_with(|context| {
            context.votes = CoopFieldValue::present(vec![CoopVote {
                phase: CoopTurnPhase::Combat,
                ..fixture_vote()
            }]);
        }),
        CoopError::VoteOutsideVotingPhase("vote.reward".to_owned()),
        "a vote taken in another phase would be read against the wrong step"
    );
}

#[test]
fn a_vote_repeating_an_option_or_naming_a_member_the_party_does_not_carry_is_refused() {
    assert_eq!(
        refuse_context_with(|context| {
            let vote = fixture_vote();
            let choices = vote.choices.value().expect("choices");
            context.votes = CoopFieldValue::present(vec![CoopVote {
                choices: CoopFieldValue::present(vec![choices[0].clone(), choices[0].clone()]),
                ..vote
            }]);
        }),
        CoopError::InvalidVote("vote.reward".to_owned())
    );
    assert_eq!(
        refuse_context_with(|context| {
            context.votes = CoopFieldValue::present(vec![CoopVote {
                decided_peer_ids: vec!["peer.absent".to_owned()],
                ..fixture_vote()
            }]);
        }),
        CoopError::InvalidVote("vote.reward".to_owned()),
        "a member that decided a vote must be a member of the party"
    );
    assert_eq!(
        refuse_context_with(|context| {
            context.votes = CoopFieldValue::present(vec![CoopVote {
                decided_peer_ids: vec!["peer.local".to_owned(), "peer.local".to_owned()],
                ..fixture_vote()
            }]);
        }),
        CoopError::InvalidVote("vote.reward".to_owned()),
        "one member decides once"
    );
}

#[test]
fn a_vote_or_an_option_past_its_local_bound_is_refused() {
    assert_eq!(
        refuse_context_with(|context| {
            let vote = fixture_vote();
            context.votes = CoopFieldValue::present(vec![vote; sts2_game_mod::COOP_MAX_VOTES + 1]);
        }),
        CoopError::InvalidInput("votes")
    );
    assert_eq!(
        refuse_context_with(|context| {
            let choice = CoopVoteChoice {
                choice_id: "choice.one".to_owned(),
                label: CoopFieldValue::absent(),
            };
            context.votes = CoopFieldValue::present(vec![CoopVote {
                choices: CoopFieldValue::present(vec![
                    choice;
                    sts2_game_mod::COOP_MAX_VOTE_CHOICES + 1
                ]),
                ..fixture_vote()
            }]);
        }),
        CoopError::InvalidInput("vote_choices")
    );
}

#[test]
fn a_targeting_relationship_that_names_itself_or_a_non_member_is_refused() {
    assert_eq!(
        refuse_context_with(|context| {
            context.targeting = CoopFieldValue::present(vec![CoopTargeting {
                source_peer_id: "peer.local".to_owned(),
                target_peer_id: "peer.local".to_owned(),
            }]);
        }),
        CoopError::InvalidTargeting("peer.local".to_owned()),
        "a member does not target itself"
    );
    assert_eq!(
        refuse_context_with(|context| {
            context.targeting = CoopFieldValue::present(vec![CoopTargeting {
                source_peer_id: "peer.local".to_owned(),
                target_peer_id: "peer.absent".to_owned(),
            }]);
        }),
        CoopError::InvalidTargeting("peer.local".to_owned()),
        "both ends of a relationship must be members of the party"
    );
}

#[test]
fn a_repeated_or_over_long_targeting_relationship_is_refused() {
    assert_eq!(
        refuse_context_with(|context| {
            let pair = CoopTargeting {
                source_peer_id: "peer.local".to_owned(),
                target_peer_id: "peer.ally".to_owned(),
            };
            context.targeting = CoopFieldValue::present(vec![pair.clone(), pair]);
        }),
        CoopError::InvalidTargeting("peer.local".to_owned())
    );
    assert_eq!(
        refuse_context_with(|context| {
            let pair = CoopTargeting {
                source_peer_id: "peer.local".to_owned(),
                target_peer_id: "peer.ally".to_owned(),
            };
            context.targeting =
                CoopFieldValue::present(vec![pair; sts2_game_mod::COOP_MAX_TARGETS + 1]);
        }),
        CoopError::InvalidInput("targeting")
    );
}

#[test]
fn readiness_is_a_row_of_the_closed_inventory_and_the_party_may_observe_it() {
    assert!(CoopField::ALL.contains(&CoopField::Readiness));
    assert_eq!(CoopField::Readiness.name(), "readiness");
    assert_eq!(
        CoopField::Readiness.default_visibility(),
        sts2_game_mod::CoopFieldVisibility::PublicToParty,
        "readiness is the public signal a party acts on, not private state"
    );
    let (_manifest, catalog) = fixture_catalog();
    for peer_id in ["peer.local", "peer.ally"] {
        let view = catalog
            .peer(&peer_reference(&catalog, peer_id), CoopReadScope::Peer)
            .expect("view");
        assert_eq!(
            view.status(CoopField::Readiness),
            sts2_game_mod::CoopFieldStatus::Present
        );
        assert!(view.readiness.is_present());
    }
    let local = catalog
        .peer(&peer_reference(&catalog, "peer.local"), CoopReadScope::Peer)
        .expect("local view");
    let ally = catalog
        .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
        .expect("ally view");
    assert_eq!(local.readiness.value(), Some(&true));
    assert_eq!(ally.readiness.value(), Some(&false));
}

#[test]
fn readiness_carried_by_a_member_without_a_coherent_snapshot_is_refused() {
    assert!(
        matches!(
            refuse_peer_with(|peer| peer.membership = CoopPeerMembership::Joining),
            CoopError::GameplayForInactivePeer(_)
        ),
        "a member that is still joining states no readiness for a step it never saw"
    );
    assert!(
        matches!(
            refuse_peer_with(|peer| peer.freshness = sts2_game_mod::CoopPeerFreshness::Lagging),
            CoopError::ValueWithoutCurrentFreshness(_)
        ),
        "a readiness that lags is not published as current either"
    );
    let mut snapshot = fixture_snapshot();
    snapshot.party.peers[1].readiness = CoopFieldValue::stale();
    let catalog = produce_snapshot(snapshot).expect("catalog");
    let ally = catalog
        .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
        .expect("ally view");
    assert_eq!(
        ally.status(CoopField::Readiness),
        sts2_game_mod::CoopFieldStatus::Stale,
        "a row that states a reason states it rather than reading as a readiness"
    );
    assert!(ally.readiness.value().is_none());
}

#[test]
fn readiness_is_the_field_that_carries_the_refusal_when_it_is_the_only_one_stated() {
    assert_eq!(
        refuse_peer_with(|peer| {
            peer.membership = CoopPeerMembership::Left;
            peer.character_id = CoopFieldValue::absent();
            peer.health = CoopFieldValue::absent();
            peer.resources = CoopFieldValue::absent();
            peer.relics = CoopFieldValue::absent();
            peer.potions = CoopFieldValue::absent();
            peer.powers = CoopFieldValue::absent();
            peer.special_mechanics = CoopFieldValue::absent();
            peer.piles = peer
                .piles
                .iter()
                .map(|pile| CoopPileView::not_permitted(pile.kind))
                .collect();
            peer.readiness = CoopFieldValue::present(true);
        }),
        CoopError::GameplayForInactivePeer("peer.ally".to_owned()),
        "readiness is a gameplay field like any other, so it is refused on a member that is gone"
    );
    assert_eq!(
        refuse_peer_with(|peer| {
            peer.freshness = sts2_game_mod::CoopPeerFreshness::Unavailable;
            peer.character_id = CoopFieldValue::absent();
            peer.health = CoopFieldValue::absent();
            peer.resources = CoopFieldValue::absent();
            peer.relics = CoopFieldValue::absent();
            peer.potions = CoopFieldValue::absent();
            peer.powers = CoopFieldValue::absent();
            peer.special_mechanics = CoopFieldValue::absent();
            peer.piles = peer
                .piles
                .iter()
                .map(|pile| CoopPileView::not_permitted(pile.kind))
                .collect();
            peer.readiness = CoopFieldValue::present(false);
        }),
        CoopError::ValueWithoutCurrentFreshness("peer.ally".to_owned()),
        "a readiness whose data is not current is not published as current"
    );
}

#[test]
fn a_declared_unavailable_family_carrying_a_context_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.family = sts2_game_mod::CoopFamilyCoverage {
        state: sts2_game_mod::CoopFamilyState::Unavailable,
        peer_count: 0,
        effect_count: 0,
        scaling_count: 0,
    };
    snapshot.party = CoopPartyInput {
        epoch: 0,
        context: CoopPartyContext {
            phase: CoopTurnPhase::Combat,
            step: 2,
            votes: CoopFieldValue::absent(),
            targeting: CoopFieldValue::absent(),
        },
        ..empty_party()
    };
    assert_eq!(
        produce_snapshot(snapshot).expect_err("a context is a party"),
        CoopError::InvalidInput("party"),
        "a party that states a step is a party even with no members"
    );
}
