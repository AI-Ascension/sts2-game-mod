// SPDX-License-Identifier: MIT

//! Party-wide and targeted effects, and the scaling rules that say how they are shared.

use super::{CoopFieldValue, CoopQuantity};

/// Which members a shared effect applies to.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopEffectScope {
    /// Applies to every member of the party.
    Party,
    /// Applies to the member that owns it.
    Peer,
    /// Applies to one named member of the party.
    Targeted,
}

/// How a co-op effect scales with party size.
///
/// A shared pool, a per-member value and a target-amplified value are three different claims, so a
/// scaling rule states which one it is instead of publishing one number for all three.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopScalingKind {
    /// Every member draws from one shared pool.
    SharedPool,
    /// Each member is affected independently, so the effect does not grow with the party.
    PerPeer,
    /// The effect is amplified by the number of members it targets.
    TargetAmplified,
}

/// One effect the party shares, with the scope it applies at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopSharedEffect {
    /// Namespaced effect identity resolved against the content manifest.
    pub effect_id: String,
    /// Which members the effect applies to.
    pub scope: CoopEffectScope,
    /// Opaque target member, present exactly when the scope is targeted.
    pub target_peer_id: CoopFieldValue<String>,
    /// Remaining stacks, or the stated reason none is reported.
    pub stacks: CoopFieldValue<CoopQuantity>,
}

/// One rule that states how an effect scales with the party.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopScalingRule {
    /// Owner-defined rule identity.
    pub rule_id: String,
    /// Which scaling claim this rule makes.
    pub kind: CoopScalingKind,
    /// Base amount before scaling.
    pub base: CoopQuantity,
    /// Per-member increment, present exactly when the rule scales per member.
    pub per_peer: CoopFieldValue<CoopQuantity>,
    /// Amplified effect, present exactly when the rule is target-amplified.
    pub effect_id: CoopFieldValue<String>,
}
