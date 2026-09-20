// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use manifest_fixture::quantity;
use sts2_game_mod::{
    CoopEffectScope, CoopError, CoopFieldValue, CoopReadScope, CoopScalingKind, CoopScalingRule,
    CoopSharedEffect,
};
use support::*;

fn refuse_effect_with(mutate: impl FnOnce(&mut CoopSharedEffect)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party.effects[0]);
    produce_snapshot(snapshot).expect_err("must be refused")
}

fn refuse_rule_with(mutate: impl FnOnce(&mut CoopScalingRule)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party.scaling[0]);
    produce_snapshot(snapshot).expect_err("must be refused")
}

#[test]
fn a_party_effect_applies_to_every_member_and_names_no_target() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let effect = record
        .party
        .effects
        .iter()
        .find(|effect| effect.scope == CoopEffectScope::Party)
        .expect("party effect");
    assert!(!effect.target_peer_id.is_present());
    assert!(effect.target_peer_id.is_consistent());
    assert!(effect.stacks.is_present());
}

#[test]
fn a_targeted_effect_names_one_member_of_the_party() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let effect = record
        .party
        .effects
        .iter()
        .find(|effect| effect.scope == CoopEffectScope::Targeted)
        .expect("targeted effect");
    let target = effect.target_peer_id.value().expect("target");
    assert!(
        record
            .party
            .peers
            .iter()
            .any(|peer| peer.peer_id == *target)
    );
}

#[test]
fn a_targeted_effect_that_names_no_member_of_the_party_is_refused() {
    assert!(matches!(
        refuse_effect_with(|effect| {
            effect.scope = CoopEffectScope::Targeted;
            effect.target_peer_id = CoopFieldValue::present("peer.absent".to_owned());
        }),
        CoopError::UnknownPeerTarget(_)
    ));
}

#[test]
fn a_targeted_effect_without_its_target_is_refused_rather_than_defaulted() {
    assert!(matches!(
        refuse_effect_with(|effect| {
            effect.scope = CoopEffectScope::Targeted;
            effect.target_peer_id = CoopFieldValue::absent();
        }),
        CoopError::UnresolvedEffectTarget(_)
    ));
}

#[test]
fn a_party_or_peer_effect_naming_a_target_is_refused() {
    for scope in [CoopEffectScope::Party, CoopEffectScope::Peer] {
        assert!(matches!(
            refuse_effect_with(|effect| {
                effect.scope = scope;
                effect.target_peer_id = CoopFieldValue::present("peer.ally".to_owned());
            }),
            CoopError::EffectScopeConflict(_)
        ));
    }
}

#[test]
fn a_shared_pool_rule_states_no_per_member_increment_and_no_amplified_effect() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let rule = record
        .party
        .scaling
        .iter()
        .find(|rule| rule.kind == CoopScalingKind::SharedPool)
        .expect("shared pool rule");
    assert!(!rule.per_peer.is_present());
    assert!(!rule.effect_id.is_present());
    assert!(rule.base.amount > 0);
}

#[test]
fn a_per_member_rule_must_state_its_increment() {
    assert!(matches!(
        refuse_rule_with(|rule| {
            rule.kind = CoopScalingKind::PerPeer;
            rule.per_peer = CoopFieldValue::absent();
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let rule = record
        .party
        .scaling
        .iter()
        .find(|rule| rule.kind == CoopScalingKind::PerPeer)
        .expect("per-member rule");
    assert_eq!(rule.per_peer.value().map(|value| value.amount), Some(2));
    assert!(!rule.effect_id.is_present());
}

#[test]
fn a_shared_pool_rule_that_claims_a_per_member_increment_is_refused() {
    assert!(matches!(
        refuse_rule_with(|rule| {
            rule.per_peer = CoopFieldValue::present(quantity("points", 2));
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
    assert!(matches!(
        refuse_rule_with(|rule| {
            rule.effect_id = CoopFieldValue::present("effect.rage".to_owned());
        }),
        CoopError::ScalingTargetMismatch(_)
    ));
}

#[test]
fn a_target_amplified_rule_must_name_an_effect_the_party_carries() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let rule = record
        .party
        .scaling
        .iter()
        .find(|rule| rule.kind == CoopScalingKind::TargetAmplified)
        .expect("amplified rule");
    let effect_id = rule.effect_id.value().expect("effect");
    assert!(
        record
            .party
            .effects
            .iter()
            .any(|effect| effect.effect_id == *effect_id)
    );
    assert!(matches!(
        refuse_rule_with(|rule| {
            rule.kind = CoopScalingKind::TargetAmplified;
            rule.effect_id = CoopFieldValue::present("effect.not_carried".to_owned());
        }),
        CoopError::UnknownManifestReference { .. } | CoopError::ScalingTargetMismatch(_)
    ));
}

#[test]
fn a_scaling_rule_keeps_its_base_separate_from_its_increment() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let rules = &record.party.scaling;
    assert_eq!(rules.len(), 3);
    for rule in rules {
        assert!(!rule.rule_id.is_empty());
        assert!(!rule.base.unit.unit.is_empty());
        assert!(
            rule.per_peer.is_consistent() && rule.effect_id.is_consistent(),
            "{} states its own availability",
            rule.rule_id
        );
    }
    let per_peer = rules
        .iter()
        .find(|rule| rule.kind == CoopScalingKind::PerPeer)
        .expect("per-member rule");
    assert_eq!(per_peer.base.amount, 5);
    assert_eq!(per_peer.per_peer.value().map(|value| value.amount), Some(2));
}

#[test]
fn the_party_scope_observes_the_same_effects_and_scaling_as_the_peer_scope() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    let peer_view = record.viewed(CoopReadScope::Peer);
    let party_view = record.viewed(CoopReadScope::Party);
    assert_eq!(peer_view.effects, party_view.effects);
    assert_eq!(peer_view.scaling, party_view.scaling);
    assert_eq!(party_view.effects.len(), 2);
    assert_eq!(party_view.scaling.len(), 3);
}

#[test]
fn an_effect_stack_unit_is_preserved_verbatim() {
    let (_manifest, catalog) = fixture_catalog();
    let effect = &catalog.party().expect("party").party.effects[1];
    assert_eq!(effect.stacks.value().map(|value| value.amount), Some(2));
    assert_eq!(
        effect.stacks.value().map(|value| value.unit.unit.as_str()),
        Some("stacks")
    );
}
