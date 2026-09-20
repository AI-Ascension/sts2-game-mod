// SPDX-License-Identifier: MIT

//! One two-member co-op party and the source that answers with it.

use std::cell::Cell;

use sts2_game_mod::{
    COOP_REFERENCE_PRODUCER_VERSION, ContentManifest, CoopCatalog, CoopCatalogProducer,
    CoopCatalogSnapshot, CoopCatalogSource, CoopEffectScope, CoopEntryKind, CoopError,
    CoopFamilyCoverage, CoopFamilyState, CoopFieldValue, CoopPartyContext, CoopPartyInput,
    CoopPartyReference, CoopPeerFreshness, CoopPeerInput, CoopPeerMembership, CoopPeerReference,
    CoopPeerRole, CoopPileKind, CoopPileView, CoopQuantity, CoopResource, CoopScalingKind,
    CoopScalingRule, CoopSharedEffect, CoopSourceError, CoopTargeting, CoopTurnPhase, CoopVote,
    CoopVoteChoice,
};

use crate::manifest_fixture::{entry, fixture_manifest, health, quantity};

/// The five pile rows of a member, with the visibility each pile kind carries.
fn reported_piles() -> Vec<CoopPileView> {
    vec![
        CoopPileView::reported(
            CoopPileKind::Draw,
            3,
            vec!["card.strike".to_owned(), "card.defend".to_owned()],
        ),
        CoopPileView::reported(CoopPileKind::Hand, 1, vec!["card.bash".to_owned()]),
        CoopPileView::reported(CoopPileKind::Discard, 1, vec!["card.strike".to_owned()]),
        CoopPileView::reported(CoopPileKind::Exhaust, 0, Vec::new()),
        CoopPileView::reported(CoopPileKind::Play, 0, Vec::new()),
    ]
}

/// The pile rows of an ally: the two local-only piles are refused, not emptied.
fn ally_piles() -> Vec<CoopPileView> {
    vec![
        CoopPileView::not_permitted(CoopPileKind::Draw),
        CoopPileView::not_permitted(CoopPileKind::Hand),
        CoopPileView::reported(CoopPileKind::Discard, 1, vec!["card.defend".to_owned()]),
        CoopPileView::reported(CoopPileKind::Exhaust, 0, Vec::new()),
        CoopPileView::reported(CoopPileKind::Play, 0, Vec::new()),
    ]
}

fn resources(unit_name: &str, amount: i64) -> CoopFieldValue<Vec<CoopResource>> {
    CoopFieldValue::present(vec![CoopResource {
        resource_id: "resource.energy".to_owned(),
        amount: quantity(unit_name, amount),
    }])
}

/// The member this party is observed from.
pub fn local_peer() -> CoopPeerInput {
    CoopPeerInput {
        peer_id: "peer.local".to_owned(),
        generation: 1,
        role: CoopPeerRole::Local,
        membership: CoopPeerMembership::Active,
        freshness: CoopPeerFreshness::Current,
        display_name: CoopFieldValue::present("Alpha".to_owned()),
        character_id: CoopFieldValue::present("character.ironclad".to_owned()),
        health: health(70, 80),
        resources: resources("energy", 3),
        relics: CoopFieldValue::present(vec![entry(
            CoopEntryKind::Relic,
            "relic.burning_blood",
            1,
            "Burning Blood",
        )]),
        potions: CoopFieldValue::present(vec![entry(
            CoopEntryKind::Potion,
            "potion.fire",
            1,
            "Fire Potion",
        )]),
        powers: CoopFieldValue::present(vec![entry(
            CoopEntryKind::Power,
            "power.strength",
            2,
            "Strength",
        )]),
        special_mechanics: CoopFieldValue::present(vec![entry(
            CoopEntryKind::SpecialMechanic,
            "mechanic.replay",
            1,
            "Replay",
        )]),
        readiness: CoopFieldValue::present(true),
        piles: reported_piles(),
    }
}

/// Another member of the same party; its local-only fields carry no value at all.
pub fn ally_peer() -> CoopPeerInput {
    CoopPeerInput {
        peer_id: "peer.ally".to_owned(),
        generation: 2,
        role: CoopPeerRole::Ally,
        membership: CoopPeerMembership::Active,
        freshness: CoopPeerFreshness::Current,
        display_name: CoopFieldValue::present("Beta".to_owned()),
        character_id: CoopFieldValue::present("character.silent".to_owned()),
        health: health(55, 70),
        resources: resources("energy", 2),
        relics: CoopFieldValue::present(vec![entry(
            CoopEntryKind::Relic,
            "relic.burning_blood",
            1,
            "Burning Blood",
        )]),
        potions: CoopFieldValue::withheld(),
        powers: CoopFieldValue::present(vec![entry(
            CoopEntryKind::Power,
            "power.strength",
            1,
            "Strength",
        )]),
        special_mechanics: CoopFieldValue::absent(),
        readiness: CoopFieldValue::present(true),
        piles: ally_piles(),
    }
}

/// The public context of the fixture party: one live vote and one targeting relationship.
///
/// The phase admits a vote and the vote names that same phase, so the fixture is coherent rather
/// than merely non-empty. A test that wants an incoherent context mutates one of the two.
pub fn fixture_context() -> CoopPartyContext {
    CoopPartyContext {
        phase: CoopTurnPhase::Combat,
        step: 12,
        votes: CoopFieldValue::present(vec![CoopVote {
            vote_id: "vote.target".to_owned(),
            phase: CoopTurnPhase::Combat,
            choices: CoopFieldValue::present(vec![CoopVoteChoice {
                choice_id: "choice.ally".to_owned(),
                label: CoopFieldValue::present("Beta".to_owned()),
            }]),
            decided_peer_ids: vec!["peer.ally".to_owned()],
            resolved: false,
        }]),
        targeting: CoopFieldValue::present(vec![CoopTargeting {
            source_peer_id: "peer.local".to_owned(),
            target_peer_id: "peer.ally".to_owned(),
        }]),
    }
}

/// One party-wide effect, one targeted effect and the three scaling claims as separate rules.
pub fn fixture_party() -> CoopPartyInput {
    CoopPartyInput {
        party_id: "party.alpha".to_owned(),
        instance_id: "instance.alpha".to_owned(),
        run_id: "run.alpha.1".to_owned(),
        epoch: 1,
        peers: vec![local_peer(), ally_peer()],
        effects: vec![
            CoopSharedEffect {
                effect_id: "effect.rage".to_owned(),
                scope: CoopEffectScope::Party,
                target_peer_id: CoopFieldValue::absent(),
                stacks: CoopFieldValue::present(quantity("stacks", 1)),
            },
            CoopSharedEffect {
                effect_id: "effect.barricade".to_owned(),
                scope: CoopEffectScope::Targeted,
                target_peer_id: CoopFieldValue::present("peer.ally".to_owned()),
                stacks: CoopFieldValue::present(quantity("stacks", 2)),
            },
        ],
        scaling: vec![
            CoopScalingRule {
                rule_id: "rule.shared".to_owned(),
                kind: CoopScalingKind::SharedPool,
                base: quantity("points", 10),
                per_peer: CoopFieldValue::absent(),
                effect_id: CoopFieldValue::absent(),
            },
            CoopScalingRule {
                rule_id: "rule.per_peer".to_owned(),
                kind: CoopScalingKind::PerPeer,
                base: quantity("points", 5),
                per_peer: CoopFieldValue::present(quantity("points", 2)),
                effect_id: CoopFieldValue::absent(),
            },
            CoopScalingRule {
                rule_id: "rule.amplified".to_owned(),
                kind: CoopScalingKind::TargetAmplified,
                base: quantity("points", 1),
                per_peer: CoopFieldValue::absent(),
                effect_id: CoopFieldValue::present("effect.rage".to_owned()),
            },
        ],
        context: fixture_context(),
    }
}

/// The source snapshot the fixture producer consumes.
pub fn fixture_snapshot() -> CoopCatalogSnapshot {
    CoopCatalogSnapshot {
        manifest: fixture_manifest().cursor_binding(),
        locale: "en-US".to_owned(),
        producer_version: COOP_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: CoopFamilyCoverage {
            state: CoopFamilyState::Handled,
            peer_count: 2,
            effect_count: 2,
            scaling_count: 3,
        },
        party: fixture_party(),
    }
}

/// Source that records how many times a read was taken, so a read can be shown not to re-read.
#[derive(Debug)]
pub struct CoopFixtureSource {
    pub snapshot: CoopCatalogSnapshot,
    pub reads: Cell<usize>,
}

impl CoopCatalogSource for CoopFixtureSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<CoopCatalogSnapshot, CoopSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

pub fn fixture_source() -> CoopFixtureSource {
    CoopFixtureSource {
        snapshot: fixture_snapshot(),
        reads: Cell::new(0),
    }
}

/// Produces a catalog from a snapshot of the shared fixture manifest.
pub fn produce_snapshot(snapshot: CoopCatalogSnapshot) -> Result<CoopCatalog, CoopError> {
    produce_with(&fixture_manifest(), snapshot)
}

/// Produces a catalog from a snapshot against an explicit manifest.
///
/// A fixture that reduces or replaces the manifest resolves its party references against that
/// manifest rather than the shared one.
pub fn produce_with(
    manifest: &ContentManifest,
    snapshot: CoopCatalogSnapshot,
) -> Result<CoopCatalog, CoopError> {
    CoopCatalogProducer::new().produce(
        manifest,
        &CoopFixtureSource {
            snapshot,
            reads: Cell::new(0),
        },
    )
}

pub fn fixture_catalog() -> (ContentManifest, CoopCatalog) {
    let manifest = fixture_manifest();
    let source = fixture_source();
    let catalog = CoopCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");
    (manifest, catalog)
}

pub fn party_reference(catalog: &CoopCatalog) -> CoopPartyReference {
    CoopPartyReference {
        catalog: catalog.binding().clone(),
        party_id: "party.alpha".to_owned(),
    }
}

pub fn peer_reference(catalog: &CoopCatalog, peer_id: &str) -> CoopPeerReference {
    CoopPeerReference {
        catalog: catalog.binding().clone(),
        party_id: "party.alpha".to_owned(),
        peer_id: peer_id.to_owned(),
        generation: generation_of(catalog, peer_id),
    }
}

/// Returns the membership generation the fixture catalog holds for one member.
///
/// A reference names a generation, so a fixture reference must name the generation the catalog
/// actually retained rather than a constant a rejoin would invalidate.
fn generation_of(catalog: &CoopCatalog, peer_id: &str) -> u64 {
    catalog
        .party()
        .ok()
        .and_then(|record| {
            record
                .party
                .peers
                .iter()
                .find(|peer| peer.peer_id == peer_id)
                .map(|peer| peer.generation)
        })
        .unwrap_or(0)
}

/// One empty party as a declared-unavailable family would carry it.
pub fn empty_party() -> CoopPartyInput {
    CoopPartyInput {
        party_id: String::new(),
        instance_id: String::new(),
        run_id: String::new(),
        epoch: 0,
        peers: Vec::new(),
        effects: Vec::new(),
        scaling: Vec::new(),
        context: CoopPartyContext {
            phase: CoopTurnPhase::Terminal,
            step: 0,
            votes: CoopFieldValue::absent(),
            targeting: CoopFieldValue::absent(),
        },
    }
}

/// A quantity builder re-exported so tests need one import for party values.
pub fn stacks(amount: i64) -> CoopFieldValue<CoopQuantity> {
    CoopFieldValue::present(quantity("stacks", amount))
}
