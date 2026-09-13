// SPDX-License-Identifier: MIT

use crate::ContentUnlockState;

use super::model::{
    PowerStatusCategory, PowerStatusKind, PowerStatusOrigin, PowerStatusReset, PowerStatusUnit,
    PowerStatusVisibility,
};

/// Typed amount shape declared by a static definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusAmountDefinition {
    /// The status is a marker and has no scalar amount.
    Amountless,
    /// One signed integer with an explicit unit.
    Integer { unit: PowerStatusUnit },
    /// One boolean state with an explicit unit or label.
    Boolean { unit: PowerStatusUnit },
    /// A fixed-point value; `scale` is decimal places.
    Decimal { unit: PowerStatusUnit, scale: u8 },
    /// A bounded textual state or label.
    Text { unit: PowerStatusUnit },
    /// Multiple named counters with independent reset/visibility semantics.
    Counters(Vec<PowerStatusCounterDefinition>),
    /// A source-defined typed value retained without executable semantics.
    Custom {
        kind: String,
        unit: Option<PowerStatusUnit>,
    },
}

/// A cap applied to a stack or amount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusCap {
    /// Optional inclusive lower bound.
    pub minimum: Option<i64>,
    /// Optional inclusive upper bound.
    pub maximum: Option<i64>,
    /// Unit for both bounds.
    pub unit: PowerStatusUnit,
}

/// Stacking and cap semantics copied from the owner definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusStackingDefinition {
    /// How a repeated application combines with the existing instance.
    pub policy: PowerStatusStackingPolicy,
    /// Optional amount or stack cap.
    pub cap: Option<PowerStatusCap>,
}

/// Supported repeated-application policies.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusStackingPolicy {
    /// Reapplications do not create additional amount.
    NonStacking,
    /// Reapplications add their amount.
    Additive,
    /// Reapplications retain the stack and refresh duration.
    RefreshDuration,
    /// Reapplications replace the prior value.
    Replace,
    /// The larger value wins.
    KeepHighest,
    /// The smaller value wins.
    KeepLowest,
    /// Separate live instances may share this definition.
    IndependentInstances,
    /// Owner-defined policy retained as a bounded token.
    Custom(String),
    /// Source could not establish policy.
    Unknown,
}

/// Static duration rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusDurationDefinition {
    /// Default duration when the owner supplies one.
    pub default: Option<i64>,
    /// Counter/boundary/condition semantics.
    pub rule: PowerStatusDurationRule,
    /// Reset timing associated with the duration.
    pub reset: PowerStatusReset,
}

/// Static duration families.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusDurationRule {
    /// No expiry is declared.
    Permanent,
    /// A visible remaining counter with an explicit unit.
    Counter { unit: PowerStatusUnit },
    /// Expires at the named boundary.
    Boundary(PowerStatusReset),
    /// Expires when the source-defined condition becomes true.
    Condition(String),
    /// Expires after a source-defined event.
    Event(String),
    /// Source could not establish duration semantics.
    Unknown,
}

/// Static decay rule and its reset timing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusDecayDefinition {
    /// Amount/condition decay semantics.
    pub rule: PowerStatusDecayRule,
    /// Boundary that resets the decay accumulator.
    pub reset: PowerStatusReset,
}

/// Supported decay forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusDecayRule {
    /// Amount does not decay.
    None,
    /// Subtract a fixed amount per reset boundary.
    By { amount: i64, unit: PowerStatusUnit },
    /// Clamp toward a floor at each reset boundary.
    To { amount: i64, unit: PowerStatusUnit },
    /// Owner-defined non-executable decay rule.
    Formula(String),
    /// Source could not establish decay semantics.
    Unknown,
}

/// One named counter for a multi-counter amount.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusCounterDefinition {
    /// Stable counter identity.
    pub id: String,
    /// Localized counter label.
    pub label: String,
    /// Counter unit.
    pub unit: PowerStatusUnit,
    /// Reset boundary for this counter.
    pub reset: PowerStatusReset,
    /// Visibility of the counter value.
    pub visibility: PowerStatusVisibility,
    /// Optional inclusive cap.
    pub cap: Option<i64>,
}

/// Semantic link to an effect, keyword, or rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusSemanticReference {
    /// Reference family.
    pub kind: PowerStatusReferenceKind,
    /// Stable owner-defined reference identity.
    pub id: String,
    /// Localized reference label.
    pub label: String,
}

/// Supported semantic reference families.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusReferenceKind {
    /// Effect or modifier rule.
    Effect,
    /// Keyword used by the description.
    Keyword,
    /// Rule or timing reference.
    Rule,
}

/// Complete source-owned static power/status definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusDefinitionInput {
    /// Namespaced content definition identity.
    pub definition_id: String,
    /// Power/status family.
    pub kind: PowerStatusKind,
    /// Owner-defined category.
    pub category: PowerStatusCategory,
    /// Localized title.
    pub title: String,
    /// Localized description.
    pub description: String,
    /// Origin/package provenance.
    pub origin: PowerStatusOrigin,
    /// Visibility of the definition itself.
    pub visibility: PowerStatusVisibility,
    /// Unlock observation copied from the content index.
    pub unlock_state: ContentUnlockState,
    /// Typed amount shape.
    pub amount: PowerStatusAmountDefinition,
    /// Reapplication semantics and caps.
    pub stacking: PowerStatusStackingDefinition,
    /// Duration and reset timing.
    pub duration: PowerStatusDurationDefinition,
    /// Decay and reset timing.
    pub decay: PowerStatusDecayDefinition,
    /// Effect/keyword/rule links.
    pub references: Vec<PowerStatusSemanticReference>,
}

/// Immutable definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusDefinition {
    /// Exact static reference.
    pub reference: super::model::PowerStatusDefinitionReference,
    /// Power/status family.
    pub kind: PowerStatusKind,
    /// Owner-defined category.
    pub category: PowerStatusCategory,
    /// Localized title.
    pub title: String,
    /// Localized description.
    pub description: String,
    /// Origin/package provenance.
    pub origin: PowerStatusOrigin,
    /// Visibility of the definition itself.
    pub visibility: PowerStatusVisibility,
    /// Unlock observation.
    pub unlock_state: ContentUnlockState,
    /// Typed amount shape.
    pub amount: PowerStatusAmountDefinition,
    /// Reapplication semantics and caps.
    pub stacking: PowerStatusStackingDefinition,
    /// Duration and reset timing.
    pub duration: PowerStatusDurationDefinition,
    /// Decay and reset timing.
    pub decay: PowerStatusDecayDefinition,
    /// Effect/keyword/rule links.
    pub references: Vec<PowerStatusSemanticReference>,
}

/// Static family coverage, including explicit unsupported/unavailable state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusFamilyCoverage {
    /// Manifest family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: PowerStatusFamilyState,
    /// Number of definitions present in the manifest family.
    pub definition_count: usize,
}

/// Source support state for the power/status family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusFamilyState {
    /// Typed source extraction is available.
    Handled,
    /// This build has no supported extractor.
    Unsupported,
    /// The source is currently unavailable.
    Unavailable,
}
