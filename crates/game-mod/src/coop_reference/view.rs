// SPDX-License-Identifier: MIT

//! Scoped projections: every field states whether it is observed, withheld or refused.

use super::{
    CoopCatalogBinding, CoopField, CoopFieldStatus, CoopFieldValue, CoopFieldVisibility,
    CoopPartyContext, CoopPartyRecord, CoopPartyReference, CoopPeerFreshness, CoopPeerMembership,
    CoopPeerRecord, CoopPeerReference, CoopPeerRole, CoopPileView, CoopReadAuthority,
    CoopReadScope, CoopScalingRule, CoopSharedEffect,
};

/// One member as a caller at a stated scope may observe it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPeerView {
    /// Exact static reference to this member.
    pub reference: CoopPeerReference,
    /// Opaque member identity.
    pub peer_id: String,
    /// Whether this member is local or an ally.
    pub role: CoopPeerRole,
    /// Membership state.
    pub membership: CoopPeerMembership,
    /// Freshness of the reported data.
    pub freshness: CoopPeerFreshness,
    /// Display name, or the reason no value is observed.
    pub display_name: CoopFieldValue<String>,
    /// Character, or the reason no value is observed.
    pub character_id: CoopFieldValue<String>,
    /// Health, or the reason no value is observed.
    pub health: CoopFieldValue<super::CoopHealth>,
    /// Resources, or the reason no value is observed.
    pub resources: CoopFieldValue<Vec<super::CoopResource>>,
    /// Relics, or the reason no value is observed.
    pub relics: CoopFieldValue<Vec<super::CoopEntry>>,
    /// Potions, or the reason no value is observed.
    pub potions: CoopFieldValue<Vec<super::CoopEntry>>,
    /// Powers, or the reason no value is observed.
    pub powers: CoopFieldValue<Vec<super::CoopEntry>>,
    /// Special mechanics, or the reason no value is observed.
    pub special_mechanics: CoopFieldValue<Vec<super::CoopEntry>>,
    /// Readiness for the current public step, or the reason no value is observed.
    pub readiness: CoopFieldValue<bool>,
    /// One row per pile, refused rather than emptied where this scope may not observe it.
    pub piles: Vec<CoopPileView>,
}

impl CoopPeerView {
    /// Returns the availability this view states for one gameplay field.
    ///
    /// The field inventory is closed, so a caller can ask about every field by name rather than
    /// reading only the ones it remembered. `Piles` is the one field carried as a row per pile: it
    /// is present only when every row states contents, and otherwise states the refusal of the
    /// first row that carries none.
    #[must_use]
    pub fn status(&self, field: CoopField) -> CoopFieldStatus {
        match field {
            CoopField::Character => self.character_id.status(),
            CoopField::Health => self.health.status(),
            CoopField::Resources => self.resources.status(),
            CoopField::Piles => self.piles_status(),
            CoopField::Relics => self.relics.status(),
            CoopField::Potions => self.potions.status(),
            CoopField::Powers => self.powers.status(),
            CoopField::SpecialMechanics => self.special_mechanics.status(),
            CoopField::Readiness => self.readiness.status(),
        }
    }

    /// Returns the first pile row that carries no contents, in the row order this view states.
    fn piles_status(&self) -> CoopFieldStatus {
        self.piles
            .iter()
            .map(|pile| pile.contents.status())
            .find(|status| status.is_stated())
            .unwrap_or(CoopFieldStatus::Present)
    }
}

/// One party as a caller at a stated scope may observe it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPartyView {
    /// Exact static reference to this party.
    pub reference: CoopPartyReference,
    /// Capability this read does not grant.
    pub authority: CoopReadAuthority,
    /// Scope this party was projected for.
    pub scope: CoopReadScope,
    /// Opaque party identity.
    pub party_id: String,
    /// The members this scope observes.
    pub peers: Vec<CoopPeerView>,
    /// Effects the party shares, with their scopes.
    pub effects: Vec<CoopSharedEffect>,
    /// Rules that state how an effect scales with the party.
    pub scaling: Vec<CoopScalingRule>,
    /// Public turn, vote and targeting context needed to interpret an action.
    pub context: CoopPartyContext,
}

/// Bounded party entry returned by one party page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPartySummary {
    /// Exact static party reference.
    pub reference: CoopPartyReference,
    /// Scope this entry was projected for.
    pub scope: CoopReadScope,
    /// Opaque party identity.
    pub party_id: String,
    /// Opaque instance identity this party belongs to.
    pub instance_id: String,
    /// Opaque run identity this party belongs to.
    pub run_id: String,
    /// The member this party is observed from, absent when no local member is declared.
    pub local_peer_id: CoopFieldValue<String>,
    /// Number of members this scope observes.
    pub observed_peer_count: usize,
    /// Number of members the source declares.
    pub peer_count: usize,
    /// Number of shared effects.
    pub effect_count: usize,
    /// Number of scaling rules.
    pub scaling_count: usize,
}

impl CoopPeerRecord {
    /// Projects one validated member for a scope, stating every refused field.
    #[must_use]
    pub fn viewed(&self, scope: CoopReadScope) -> CoopPeerView {
        let role = self.peer.role;
        CoopPeerView {
            reference: CoopPeerReference {
                catalog: self.binding.clone(),
                party_id: self.party_id.clone(),
                peer_id: self.peer.peer_id.clone(),
                generation: self.peer.generation,
            },
            peer_id: self.peer.peer_id.clone(),
            role,
            membership: self.peer.membership,
            freshness: self.peer.freshness,
            display_name: project(
                &self.peer.display_name,
                scope,
                role,
                CoopFieldVisibility::PublicToParty,
            ),
            character_id: project_field(
                &self.peer.character_id,
                super::CoopField::Character,
                scope,
                role,
            ),
            health: project_field(&self.peer.health, super::CoopField::Health, scope, role),
            resources: project_field(
                &self.peer.resources,
                super::CoopField::Resources,
                scope,
                role,
            ),
            relics: project_field(&self.peer.relics, super::CoopField::Relics, scope, role),
            potions: project_field(&self.peer.potions, super::CoopField::Potions, scope, role),
            powers: project_field(&self.peer.powers, super::CoopField::Powers, scope, role),
            special_mechanics: project_field(
                &self.peer.special_mechanics,
                super::CoopField::SpecialMechanics,
                scope,
                role,
            ),
            readiness: project_field(
                &self.peer.readiness,
                super::CoopField::Readiness,
                scope,
                role,
            ),
            piles: self
                .peer
                .piles
                .iter()
                .map(|pile| project_pile(pile, scope, role))
                .collect(),
        }
    }

    /// Returns whether a caller at this scope may observe this member's own row at all.
    #[must_use]
    pub const fn is_observable(&self, scope: CoopReadScope) -> bool {
        scope.observes_peer(self.peer.role)
    }
}

impl CoopPartyRecord {
    /// Projects a party for a scope, refusing rather than emptying what the scope may not observe.
    #[must_use]
    pub fn viewed(&self, scope: CoopReadScope) -> CoopPartyView {
        CoopPartyView {
            reference: CoopPartyReference {
                catalog: self.binding.clone(),
                party_id: self.party.party_id.clone(),
            },
            authority: CoopReadAuthority::NotGranted,
            scope,
            party_id: self.party.party_id.clone(),
            peers: self
                .party
                .peers
                .iter()
                .filter(|peer| scope.observes_peer(peer.role))
                .map(|peer| {
                    CoopPeerRecord {
                        binding: self.binding.clone(),
                        party_id: self.party.party_id.clone(),
                        peer: peer.clone(),
                    }
                    .viewed(scope)
                })
                .collect(),
            effects: self.party.effects.clone(),
            scaling: self.party.scaling.clone(),
            context: self.party.context.clone(),
        }
    }

    /// Builds one bounded party entry without projecting the member rows.
    #[must_use]
    pub fn summary(&self, scope: CoopReadScope) -> CoopPartySummary {
        CoopPartySummary {
            reference: CoopPartyReference {
                catalog: self.binding.clone(),
                party_id: self.party.party_id.clone(),
            },
            scope,
            party_id: self.party.party_id.clone(),
            instance_id: self.party.instance_id.clone(),
            run_id: self.party.run_id.clone(),
            local_peer_id: self
                .party
                .local_peer()
                .map_or_else(CoopFieldValue::absent, |peer| {
                    CoopFieldValue::present(peer.peer_id.clone())
                }),
            observed_peer_count: self
                .party
                .peers
                .iter()
                .filter(|peer| scope.observes_peer(peer.role))
                .count(),
            peer_count: self.party.peers.len(),
            effect_count: self.party.effects.len(),
            scaling_count: self.party.scaling.len(),
        }
    }

    /// Returns the exact static reference to this party.
    #[must_use]
    pub fn reference(&self) -> CoopPartyReference {
        CoopPartyReference {
            catalog: self.binding.clone(),
            party_id: self.party.party_id.clone(),
        }
    }

    /// Returns the binding this record was validated against.
    #[must_use]
    pub fn binding(&self) -> &CoopCatalogBinding {
        &self.binding
    }
}

fn project<T: Clone>(
    value: &CoopFieldValue<T>,
    scope: CoopReadScope,
    role: CoopPeerRole,
    visibility: CoopFieldVisibility,
) -> CoopFieldValue<T> {
    if scope.observes_peer_field(role, visibility) {
        value.clone()
    } else {
        CoopFieldValue::not_permitted()
    }
}

fn project_field<T: Clone>(
    value: &CoopFieldValue<T>,
    field: super::CoopField,
    scope: CoopReadScope,
    role: CoopPeerRole,
) -> CoopFieldValue<T> {
    project(value, scope, role, field.default_visibility())
}

fn project_pile(pile: &CoopPileView, scope: CoopReadScope, role: CoopPeerRole) -> CoopPileView {
    if scope.observes_peer_field(role, pile.visibility) {
        pile.clone()
    } else {
        CoopPileView::not_permitted(pile.kind)
    }
}
