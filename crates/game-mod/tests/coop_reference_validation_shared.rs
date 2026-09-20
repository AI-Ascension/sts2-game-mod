// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use manifest_fixture::quantity;
use sts2_game_mod::{CoopError, CoopFieldValue, CoopPartyInput};
use support::*;

/// Produces the shared fixture with one party-wide change and returns the refusal.
fn refuse_party_with(mutate: impl FnOnce(&mut CoopPartyInput)) -> CoopError {
    let mut snapshot = fixture_snapshot();
    mutate(&mut snapshot.party);
    produce_snapshot(snapshot).expect_err("must be refused")
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
