// SPDX-License-Identifier: MIT

//! Shared-effect and scaling-rule validation across the whole party.

use std::collections::BTreeSet;

use crate::ContentManifest;

use super::super::identity::validate_opaque_identity;
use super::super::{
    COOP_EFFECT_KIND, CoopEffectScope, CoopError, CoopScalingKind, CoopScalingRule,
    CoopSharedEffect,
};
use super::fields::{require_consistent, validate_text};
use super::manifest::validate_manifest_reference;

/// Validates every shared effect against the party's own member set.
///
/// A targeted effect that names no member of the party is refused: the target would otherwise be a
/// dangling identity that a reader could mistake for a member it simply cannot see.
pub(super) fn validate_effects(
    effects: &[CoopSharedEffect],
    peers: &BTreeSet<String>,
    manifest: &ContentManifest,
) -> Result<(), CoopError> {
    let mut seen = BTreeSet::new();
    for effect in effects {
        validate_text(&effect.effect_id, "effect_id")?;
        validate_manifest_reference(manifest, COOP_EFFECT_KIND, &effect.effect_id)?;
        if !seen.insert(effect.effect_id.clone()) {
            return Err(CoopError::DuplicateEffect(effect.effect_id.clone()));
        }
        require_consistent(&effect.target_peer_id, "target_peer_id")?;
        require_consistent(&effect.stacks, "stacks")?;
        if let Some(quantity) = effect.stacks.value() {
            validate_text(&quantity.unit.unit, "effect_stacks")?;
        }
        match effect.scope {
            CoopEffectScope::Targeted => {
                let Some(target) = effect.target_peer_id.value() else {
                    return Err(CoopError::UnresolvedEffectTarget(effect.effect_id.clone()));
                };
                validate_opaque_identity(target, "target_peer_id")?;
                if !peers.contains(target) {
                    return Err(CoopError::UnknownPeerTarget(target.clone()));
                }
            }
            CoopEffectScope::Party | CoopEffectScope::Peer => {
                if effect.target_peer_id.is_present() {
                    return Err(CoopError::EffectScopeConflict(effect.effect_id.clone()));
                }
            }
        }
    }
    Ok(())
}

/// Validates every scaling rule against the effects the party actually carries.
///
/// A rule that claims a per-member increment without stating it, or that amplifies an effect the
/// party does not carry, is refused rather than defaulted, because either default would publish a
/// number the source never reported.
pub(super) fn validate_scaling(
    rules: &[CoopScalingRule],
    effects: &[CoopSharedEffect],
    manifest: &ContentManifest,
) -> Result<(), CoopError> {
    let mut seen = BTreeSet::new();
    for rule in rules {
        validate_text(&rule.rule_id, "rule_id")?;
        if !seen.insert(rule.rule_id.clone()) {
            return Err(CoopError::DuplicateScalingRule(rule.rule_id.clone()));
        }
        require_consistent(&rule.per_peer, "per_peer")?;
        require_consistent(&rule.effect_id, "effect_id")?;
        validate_text(&rule.base.unit.unit, "scaling_base")?;
        if let Some(quantity) = rule.per_peer.value() {
            validate_text(&quantity.unit.unit, "scaling_per_peer")?;
        }
        match rule.kind {
            CoopScalingKind::PerPeer => {
                if !rule.per_peer.is_present() || rule.effect_id.is_present() {
                    return Err(CoopError::ScalingTargetMismatch(rule.rule_id.clone()));
                }
            }
            CoopScalingKind::SharedPool => {
                if rule.per_peer.is_present() || rule.effect_id.is_present() {
                    return Err(CoopError::ScalingTargetMismatch(rule.rule_id.clone()));
                }
            }
            CoopScalingKind::TargetAmplified => {
                let Some(effect_id) = rule.effect_id.value() else {
                    return Err(CoopError::ScalingTargetMismatch(rule.rule_id.clone()));
                };
                if rule.per_peer.is_present() {
                    return Err(CoopError::ScalingTargetMismatch(rule.rule_id.clone()));
                }
                validate_manifest_reference(manifest, COOP_EFFECT_KIND, effect_id)?;
                if !effects.iter().any(|effect| effect.effect_id == *effect_id) {
                    return Err(CoopError::ScalingTargetMismatch(rule.rule_id.clone()));
                }
            }
        }
    }
    Ok(())
}
