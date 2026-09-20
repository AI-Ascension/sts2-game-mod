// SPDX-License-Identifier: MIT

//! Validation of the public party context: phase, votes, targeting and readiness.
//!
//! The context is public by construction, so what can go wrong is not disclosure but incoherence:
//! a vote taken in a phase that admits none, a vote naming a member the party does not carry, or a
//! member declaring readiness against a step the party's own context contradicts.

use std::collections::BTreeSet;

use super::super::identity::validate_opaque_identity;
use super::super::model::{COOP_MAX_TARGETS, COOP_MAX_VOTE_CHOICES, COOP_MAX_VOTES};
use super::super::{CoopError, CoopPartyContext, CoopPeerInput};
use super::fields::require_bounded;

/// Validates the public context against the members the party actually declares.
pub(super) fn validate_context(
    context: &CoopPartyContext,
    peers: &[CoopPeerInput],
) -> Result<(), CoopError> {
    let ids: BTreeSet<&str> = peers.iter().map(|peer| peer.peer_id.as_str()).collect();
    validate_votes(context, &ids)?;
    validate_targeting(context, &ids)
}

fn validate_votes(context: &CoopPartyContext, ids: &BTreeSet<&str>) -> Result<(), CoopError> {
    let Some(votes) = context.votes.value() else {
        return Ok(());
    };
    require_bounded(votes, COOP_MAX_VOTES, "votes")?;
    let mut seen = BTreeSet::new();
    for vote in votes {
        validate_opaque_identity(&vote.vote_id, "vote_id")?;
        if !seen.insert(vote.vote_id.as_str()) {
            return Err(CoopError::InvalidVote(vote.vote_id.clone()));
        }
        if !vote.phase.admits_vote() {
            return Err(CoopError::VoteOutsideVotingPhase(
                vote.phase.name().to_owned(),
            ));
        }
        if vote.phase != context.phase {
            return Err(CoopError::VoteOutsideVotingPhase(vote.vote_id.clone()));
        }
        if let Some(choices) = vote.choices.value() {
            require_bounded(choices, COOP_MAX_VOTE_CHOICES, "vote_choices")?;
            let mut seen_choices = BTreeSet::new();
            for choice in choices {
                validate_opaque_identity(&choice.choice_id, "choice_id")?;
                if !seen_choices.insert(choice.choice_id.as_str()) {
                    return Err(CoopError::InvalidVote(vote.vote_id.clone()));
                }
            }
        }
        let mut decided = BTreeSet::new();
        for peer_id in &vote.decided_peer_ids {
            if !ids.contains(peer_id.as_str()) {
                return Err(CoopError::InvalidVote(vote.vote_id.clone()));
            }
            if !decided.insert(peer_id.as_str()) {
                return Err(CoopError::InvalidVote(vote.vote_id.clone()));
            }
        }
    }
    Ok(())
}

fn validate_targeting(context: &CoopPartyContext, ids: &BTreeSet<&str>) -> Result<(), CoopError> {
    let Some(targeting) = context.targeting.value() else {
        return Ok(());
    };
    require_bounded(targeting, COOP_MAX_TARGETS, "targeting")?;
    let mut seen = BTreeSet::new();
    for target in targeting {
        if target.source_peer_id == target.target_peer_id {
            return Err(CoopError::InvalidTargeting(target.source_peer_id.clone()));
        }
        if !ids.contains(target.source_peer_id.as_str())
            || !ids.contains(target.target_peer_id.as_str())
        {
            return Err(CoopError::InvalidTargeting(target.source_peer_id.clone()));
        }
        if !seen.insert((
            target.source_peer_id.as_str(),
            target.target_peer_id.as_str(),
        )) {
            return Err(CoopError::InvalidTargeting(target.source_peer_id.clone()));
        }
    }
    Ok(())
}

/// Reports the context bytes one party publishes, for the aggregate byte bound.
pub(super) fn context_bytes(context: &CoopPartyContext) -> usize {
    let votes = context.votes.value().map_or(0, |votes| {
        votes
            .iter()
            .map(|vote| {
                vote.vote_id.len()
                    + vote.choices.value().map_or(0, |choices| {
                        choices
                            .iter()
                            .map(|choice| {
                                choice.choice_id.len() + choice.label.value().map_or(0, String::len)
                            })
                            .sum::<usize>()
                    })
                    + vote.decided_peer_ids.iter().map(String::len).sum::<usize>()
            })
            .sum::<usize>()
    });
    let targeting = context.targeting.value().map_or(0, |targets| {
        targets
            .iter()
            .map(|target| target.source_peer_id.len() + target.target_peer_id.len())
            .sum::<usize>()
    });
    votes + targeting
}
