// SPDX-License-Identifier: MIT

use crate::ContentUnlockState;

use super::model::{RelicField, RelicPool, RelicRarity, RelicTier, RelicUnit, RelicVisibility};

/// Acquisition and unlock metadata copied from an owner source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicAcquisition {
    pub rules: Vec<RelicAcquisitionRule>,
    pub unlock: RelicUnlock,
}

/// One acquisition path with optional source and restriction references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicAcquisitionRule {
    pub kind: String,
    pub reference: Option<String>,
    pub requirement: Option<String>,
}

/// Unlock observation and explicit requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicUnlock {
    pub state: ContentUnlockState,
    pub requirements: Vec<String>,
}

/// Static condition reference used by activation and trigger metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicCondition {
    pub id: String,
    pub label: String,
}

/// Explicit activation model; callers must not infer it from a display name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicActivationDefinition {
    pub kind: RelicActivationKind,
    pub condition: Option<RelicCondition>,
    pub counter_ids: Vec<String>,
}

/// Typed activation families.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicActivationKind {
    Passive,
    Charged,
    TurnCounter,
    RoomCounter,
    Conditional,
    Multiple,
    Unknown,
}

/// Static counter schema for one relic definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicCounterDefinition {
    pub id: String,
    pub label: String,
    pub unit: RelicUnit,
    pub visibility: RelicVisibility,
    pub reset: super::model::RelicCounterReset,
}

/// Live value for one declared counter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicCounterState {
    pub id: String,
    pub value: RelicField<i64>,
}

/// Typed visible parameter value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RelicParameterValue {
    Integer(i64),
    Boolean(bool),
    Text(String),
}

/// Static parameter declaration used to link descriptions to visible values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicParameterDefinition {
    pub id: String,
    pub label: String,
    pub unit: RelicUnit,
    pub visibility: RelicVisibility,
}

/// Resolved value linked to a static parameter ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicResolvedParameter {
    pub id: String,
    pub unit: RelicUnit,
    pub value: RelicParameterValue,
}

/// Semantic reference to an effect, keyword, or rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicSemanticReference {
    pub kind: RelicSemanticReferenceKind,
    pub id: String,
    pub label: String,
}

/// Supported semantic reference families.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicSemanticReferenceKind {
    Effect,
    Keyword,
    Rule,
}

/// Explicit supported variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicVariant {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}

/// Static pending-trigger declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicTriggerDefinition {
    pub id: String,
    pub label: String,
    pub condition: Option<RelicCondition>,
    pub visibility: RelicVisibility,
}

/// Complete source-owned static relic definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicDefinitionInput {
    pub relic_id: String,
    pub title: String,
    pub description: String,
    pub rarity: RelicRarity,
    pub tier: RelicTier,
    pub pool: Option<RelicPool>,
    pub origin: super::model::RelicOrigin,
    pub acquisition: RelicAcquisition,
    pub references: Vec<RelicSemanticReference>,
    pub variants: Vec<RelicVariant>,
    pub parameters: Vec<RelicParameterDefinition>,
    pub counters: Vec<RelicCounterDefinition>,
    pub activation: RelicActivationDefinition,
    pub triggers: Vec<RelicTriggerDefinition>,
}

/// Immutable definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicDefinition {
    pub reference: super::model::RelicDefinitionReference,
    pub title: String,
    pub description: String,
    pub rarity: RelicRarity,
    pub tier: RelicTier,
    pub pool: Option<RelicPool>,
    pub origin: super::model::RelicOrigin,
    pub acquisition: RelicAcquisition,
    pub references: Vec<RelicSemanticReference>,
    pub variants: Vec<RelicVariant>,
    pub parameters: Vec<RelicParameterDefinition>,
    pub counters: Vec<RelicCounterDefinition>,
    pub activation: RelicActivationDefinition,
    pub triggers: Vec<RelicTriggerDefinition>,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicFamilyCoverage {
    pub entity_kind: String,
    pub state: RelicFamilyState,
    pub definition_count: usize,
}

/// Source support state for the relic family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicFamilyState {
    Handled,
    Unsupported,
    Unavailable,
}

/// Static visibility scope.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RelicVisibilityScope {
    Public,
    Reference,
    /// Explicit owner-authorized scope for owner-only fields.
    Owner,
}
