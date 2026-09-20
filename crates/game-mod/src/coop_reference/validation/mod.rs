// SPDX-License-Identifier: MIT

//! Fail-closed validation of one co-op party record before it enters an immutable catalog.

mod context;
mod fields;
mod manifest;
mod peers;
mod shared;

use std::collections::BTreeSet;

use crate::ContentManifest;

use self::context::{context_bytes, validate_context};
use self::fields::require_bounded;
use self::peers::{entry_bytes, pile_bytes, resource_bytes, validate_peer};
use self::shared::{validate_effects, validate_scaling};

use super::identity::validate_opaque_identity;
use super::model::{
    COOP_MAX_EFFECTS, COOP_MAX_PARTY_BYTES, COOP_MAX_PEERS, COOP_MAX_SCALING_RULES,
};
use super::profile::validate_party_scope;
use super::{CoopError, CoopPartyInput, CoopPeerInput, CoopPeerRole, CoopSharedEffect};

fn require_within(actual: usize, limit: usize) -> Result<(), CoopError> {
    if actual > limit {
        return Err(CoopError::PartyTooLarge { limit, actual });
    }
    Ok(())
}

/// Validates one source-owned party record before it enters an immutable catalog.
pub(super) fn validate_party(
    input: &CoopPartyInput,
    manifest: &ContentManifest,
) -> Result<(), CoopError> {
    validate_opaque_identity(&input.party_id, "party_id")?;
    validate_opaque_identity(&input.instance_id, "instance_id")?;
    validate_opaque_identity(&input.run_id, "run_id")?;
    if input.peers.is_empty() {
        return Err(CoopError::EmptyPresentCollection("peers"));
    }
    require_bounded(&input.peers, COOP_MAX_PEERS, "peers")?;
    require_bounded(&input.effects, COOP_MAX_EFFECTS, "effects")?;
    require_bounded(&input.scaling, COOP_MAX_SCALING_RULES, "scaling")?;
    validate_party_scope(input)?;
    let peers = validate_members(&input.peers, manifest)?;
    validate_context(&input.context, &input.peers)?;
    validate_effects(&input.effects, &peers, manifest)?;
    validate_scaling(&input.scaling, &input.effects, manifest)?;
    require_within(party_bytes(input), COOP_MAX_PARTY_BYTES)
}

/// Validates every member and returns the set of member identities the party declares.
///
/// Exactly one member must be local, because the local-only fields of a party belong to exactly one
/// member: a party with none could not own them, and a party with two would let either member read
/// the other's.
///
/// A member's generation must be unique within the party, because a rejoin increments it: two
/// members of one party sharing a generation would make a generation fence unable to tell a
/// pre-rejoin reference from a post-rejoin one.
fn validate_members(
    peers: &[CoopPeerInput],
    manifest: &ContentManifest,
) -> Result<BTreeSet<String>, CoopError> {
    let mut ids = BTreeSet::new();
    let mut generations = BTreeSet::new();
    let mut local: Option<&str> = None;
    for peer in peers {
        validate_peer(peer, manifest)?;
        if !ids.insert(peer.peer_id.clone()) {
            return Err(CoopError::DuplicatePeer(peer.peer_id.clone()));
        }
        if matches!(peer.role, CoopPeerRole::Local) {
            if local.is_some() {
                return Err(CoopError::MultipleLocalPeers(peer.peer_id.clone()));
            }
            local = Some(peer.peer_id.as_str());
        }
        if !generations.insert(peer.generation) {
            return Err(CoopError::DuplicatePeerGeneration(peer.peer_id.clone()));
        }
    }
    if local.is_none() {
        return Err(CoopError::MissingLocalPeer);
    }
    Ok(ids)
}

/// Returns the aggregate owner-defined text bytes one party record would publish.
fn party_bytes(input: &CoopPartyInput) -> usize {
    let peers = input
        .peers
        .iter()
        .map(|peer| {
            peer.peer_id.len()
                + peer.display_name.value().map_or(0, String::len)
                + peer.character_id.value().map_or(0, String::len)
                + resource_bytes(&peer.resources)
                + entry_bytes(&peer.relics)
                + entry_bytes(&peer.potions)
                + entry_bytes(&peer.powers)
                + entry_bytes(&peer.special_mechanics)
                + pile_bytes(peer)
        })
        .sum::<usize>();
    let effects = input
        .effects
        .iter()
        .map(|effect| effect.effect_id.len() + effect_bytes(effect))
        .sum::<usize>();
    let scaling = input
        .scaling
        .iter()
        .map(|rule| rule.rule_id.len() + rule.base.unit.unit.len())
        .sum::<usize>();
    input.party_id.len()
        + input.instance_id.len()
        + input.run_id.len()
        + peers
        + effects
        + scaling
        + context_bytes(&input.context)
}

fn effect_bytes(effect: &CoopSharedEffect) -> usize {
    effect
        .stacks
        .value()
        .map_or(0, |quantity| quantity.unit.unit.len())
}
