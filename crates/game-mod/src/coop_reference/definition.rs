// SPDX-License-Identifier: MIT

//! Owner-reported party and member records, and the records a catalog retains.

use super::{
    CoopCatalogBinding, CoopFieldValue, CoopHealth, CoopPartyContext, CoopPeerFreshness,
    CoopPeerMembership, CoopPeerRole, CoopPileView, CoopResource, CoopScalingRule,
    CoopSharedEffect,
};

/// Kind of a retained member entry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopEntryKind {
    /// A relic.
    Relic,
    /// A potion.
    Potion,
    /// A power or status.
    Power,
    /// A special mechanic the supported build reports.
    SpecialMechanic,
}

/// One retained relic, potion, power or special mechanic of a member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopEntry {
    /// Namespaced identity resolved against the content manifest.
    pub entry_id: String,
    /// Which family the identity resolves against.
    pub kind: CoopEntryKind,
    /// Copies or stacks held.
    pub count: u32,
    /// Localized or owner-defined label.
    pub label: CoopFieldValue<String>,
}

/// Owner-reported state of one member of a party.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPeerInput {
    /// Opaque member identity.
    pub peer_id: String,
    /// Membership generation of this member, which a rejoin increments.
    pub generation: u64,
    /// Whether this member is the observing one or another member of the party.
    pub role: CoopPeerRole,
    /// Membership state of this member.
    pub membership: CoopPeerMembership,
    /// Freshness of the data reported for this member.
    pub freshness: CoopPeerFreshness,
    /// Display name, stated absent when the host reports none.
    pub display_name: CoopFieldValue<String>,
    /// The character this member is playing.
    pub character_id: CoopFieldValue<String>,
    /// Current and maximum health.
    pub health: CoopFieldValue<CoopHealth>,
    /// Character-specific resources.
    pub resources: CoopFieldValue<Vec<CoopResource>>,
    /// Relics this member holds.
    pub relics: CoopFieldValue<Vec<CoopEntry>>,
    /// Potions this member holds.
    pub potions: CoopFieldValue<Vec<CoopEntry>>,
    /// Powers and statuses affecting this member.
    pub powers: CoopFieldValue<Vec<CoopEntry>>,
    /// Special mechanics the supported build reports for this member.
    pub special_mechanics: CoopFieldValue<Vec<CoopEntry>>,
    /// Whether this member has signalled readiness for the current public step.
    pub readiness: CoopFieldValue<bool>,
    /// One row per pile, each with its own visibility and stated availability.
    pub piles: Vec<CoopPileView>,
}

/// Owner-reported party: one instance's membership, effects and scaling rules.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPartyInput {
    /// Opaque party identity.
    pub party_id: String,
    /// Opaque instance identity this party belongs to.
    pub instance_id: String,
    /// Opaque run identity this party belongs to.
    pub run_id: String,
    /// Monotonic epoch of the instance's observation stream this party was read at.
    pub epoch: u64,
    /// The members, exactly one of which is local.
    pub peers: Vec<CoopPeerInput>,
    /// Effects the party shares.
    pub effects: Vec<CoopSharedEffect>,
    /// Rules that state how an effect scales with the party.
    pub scaling: Vec<CoopScalingRule>,
    /// Public turn, vote and targeting context needed to interpret an action.
    pub context: CoopPartyContext,
}

impl CoopPartyInput {
    /// Returns the member this party is observed from, if it has exactly one.
    #[must_use]
    pub fn local_peer(&self) -> Option<&CoopPeerInput> {
        self.peers
            .iter()
            .find(|peer| matches!(peer.role, CoopPeerRole::Local))
    }
}

/// A validated member record bound to one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPeerRecord {
    /// Catalog binding this record was validated against.
    pub binding: CoopCatalogBinding,
    /// Party this member belongs to.
    pub party_id: String,
    /// Validated member value.
    pub peer: CoopPeerInput,
}

impl CoopPeerRecord {
    /// Returns the opaque member identity.
    #[must_use]
    pub fn peer_id(&self) -> &str {
        &self.peer.peer_id
    }

    /// Returns whether this member may carry a coherent gameplay value.
    #[must_use]
    pub const fn carries_gameplay(&self) -> bool {
        self.peer.membership.carries_gameplay() && self.peer.freshness.is_current()
    }
}

/// A validated party record bound to one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPartyRecord {
    /// Catalog binding this record was validated against.
    pub binding: CoopCatalogBinding,
    /// Validated party value.
    pub party: CoopPartyInput,
}

impl CoopPartyRecord {
    /// Returns the opaque party identity.
    #[must_use]
    pub fn party_id(&self) -> &str {
        &self.party.party_id
    }

    /// Returns the exact reference to one of this party's members.
    #[must_use]
    pub fn peer_reference(&self, peer_id: &str) -> super::CoopPeerReference {
        let generation = self
            .party
            .peers
            .iter()
            .find(|peer| peer.peer_id == peer_id)
            .map_or(0, |peer| peer.generation);
        super::CoopPeerReference {
            catalog: self.binding.clone(),
            party_id: self.party.party_id.clone(),
            peer_id: peer_id.to_owned(),
            generation,
        }
    }
}
