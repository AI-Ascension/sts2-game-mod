// SPDX-License-Identifier: MIT

use crate::ContentUnlockState;

use super::model::{
    PotionField, PotionPool, PotionRarity, PotionTargetMode, PotionUnit, PotionUseRule,
    PotionVisibility,
};

/// Acquisition and unlock metadata copied from an owner source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionAcquisition {
    /// Bounded acquisition paths such as rewards, shops, or event sources.
    pub rules: Vec<PotionAcquisitionRule>,
    /// Explicit unlock observation and requirements.
    pub unlock: PotionUnlock,
}

/// One acquisition path with optional source and restriction references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionAcquisitionRule {
    /// Owner-defined acquisition kind.
    pub kind: String,
    /// Optional source identity such as a reward table or shop.
    pub reference: Option<String>,
    /// Optional owner-defined restriction reference.
    pub requirement: Option<String>,
}

/// Unlock observation and explicit requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionUnlock {
    /// Explicit owner unlock state.
    pub state: ContentUnlockState,
    /// Bounded requirement identities, never inferred by this module.
    pub requirements: Vec<String>,
}

/// Static condition reference used by conditional effects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionCondition {
    /// Stable owner-defined condition identity.
    pub id: String,
    /// Localized or owner-defined condition label.
    pub label: String,
}

/// Static magnitude shape. Formulas and unknowns remain explicit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionEffectMagnitude {
    /// One fixed signed amount.
    Fixed(i64),
    /// A bounded inclusive range whose outcome is resolved by the game.
    Range { min: i64, max: i64 },
    /// An owner-defined formula retained as text; it is never evaluated here.
    Formula(String),
    /// The source exposed a magnitude but could not classify it.
    Unknown,
}

/// Structural effect families; this module does not resolve random choices.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionEffectKind {
    /// One direct effect with a typed magnitude or explicit unknown.
    Direct,
    /// A deterministic group of referenced effects.
    Multiple,
    /// One of the bounded alternatives is selected by a game rule or RNG.
    RandomChoice,
    /// An effect gated by a condition.
    Conditional,
    /// A source effect whose family is not classified.
    Unknown,
}

/// A bounded alternative for a multi-effect or choice effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionEffectAlternative {
    /// Stable alternative identity.
    pub id: String,
    /// Localized or owner-defined alternative label.
    pub label: String,
    /// Referenced effect IDs. An empty list is allowed only for explicit unknown outcomes.
    pub effect_ids: Vec<String>,
    /// Optional rule reference controlling selection.
    pub rule_reference: Option<String>,
}

/// Semantic reference to an effect, keyword, or rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionSemanticReference {
    /// Reference family.
    pub kind: PotionSemanticReferenceKind,
    /// Stable owner-defined identity.
    pub id: String,
    /// Human-readable label.
    pub label: String,
}

/// Supported semantic reference families.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionSemanticReferenceKind {
    Effect,
    Keyword,
    Rule,
}

/// One typed static effect record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionEffect {
    /// Stable effect identity.
    pub id: String,
    /// Human-readable effect label.
    pub label: String,
    /// Structural effect family.
    pub kind: PotionEffectKind,
    /// Target mode declared for this effect.
    pub target: PotionTargetMode,
    /// Base amount or explicit unavailable/unknown state.
    pub magnitude: PotionField<PotionEffectMagnitude>,
    /// Condition required by a conditional effect.
    pub condition: Option<PotionCondition>,
    /// Bounded alternatives/referenced effect branches.
    pub alternatives: Vec<PotionEffectAlternative>,
    /// Effect-local semantic references.
    pub references: Vec<PotionSemanticReference>,
    /// Field visibility.
    pub visibility: PotionVisibility,
}

/// Static parameter declaration used to link descriptions to visible values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionParameterDefinition {
    /// Stable parameter ID.
    pub id: String,
    /// Human-readable parameter label.
    pub label: String,
    /// Parameter unit.
    pub unit: PotionUnit,
    /// Field visibility.
    pub visibility: PotionVisibility,
}

/// Typed live value for one declared parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PotionParameterValue {
    Integer(i64),
    Boolean(bool),
    Text(String),
    IntegerList(Vec<i64>),
    /// A source value exists but cannot be classified.
    Unknown,
}

/// Resolved visible parameter linked to a static parameter ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionResolvedParameter {
    /// Static parameter ID.
    pub id: String,
    /// Unit copied from the static declaration.
    pub unit: PotionUnit,
    /// Current visible value.
    pub value: PotionParameterValue,
}

/// Complete source-owned static potion definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionDefinitionInput {
    /// Namespaced potion ID.
    pub potion_id: String,
    /// Localized title.
    pub title: String,
    /// Localized static description.
    pub description: String,
    /// Owner-defined rarity.
    pub rarity: PotionRarity,
    /// Optional acquisition pool.
    pub pool: Option<PotionPool>,
    /// Source origin/provenance.
    pub origin: super::model::PotionOrigin,
    /// Acquisition and unlock metadata.
    pub acquisition: PotionAcquisition,
    /// Static targeting mode.
    pub target_mode: PotionTargetMode,
    /// Static usability rule.
    pub use_rule: PotionUseRule,
    /// Bounded structural effects.
    pub effects: Vec<PotionEffect>,
    /// Static parameter declarations.
    pub parameters: Vec<PotionParameterDefinition>,
    /// Effect/keyword/rule references.
    pub references: Vec<PotionSemanticReference>,
}

/// Immutable definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionDefinition {
    /// Exact definition reference.
    pub reference: super::model::PotionDefinitionReference,
    /// Localized title.
    pub title: String,
    /// Localized static description.
    pub description: String,
    /// Owner-defined rarity.
    pub rarity: PotionRarity,
    /// Optional acquisition pool.
    pub pool: Option<PotionPool>,
    /// Source origin/provenance.
    pub origin: super::model::PotionOrigin,
    /// Acquisition and unlock metadata.
    pub acquisition: PotionAcquisition,
    /// Static targeting mode.
    pub target_mode: PotionTargetMode,
    /// Static usability rule.
    pub use_rule: PotionUseRule,
    /// Bounded structural effects.
    pub effects: Vec<PotionEffect>,
    /// Static parameter declarations.
    pub parameters: Vec<PotionParameterDefinition>,
    /// Effect/keyword/rule references.
    pub references: Vec<PotionSemanticReference>,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PotionFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: PotionFamilyState,
    /// Number of potion definitions in the manifest.
    pub definition_count: usize,
}

/// Source support state for the potion family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PotionFamilyState {
    Handled,
    Unsupported,
    Unavailable,
}
