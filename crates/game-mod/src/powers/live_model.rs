// SPDX-License-Identifier: MIT

use super::{
    definition::PowerStatusDurationRule,
    model::{
        PowerStatusField, PowerStatusLiveBinding, PowerStatusOwnerId, PowerStatusReset,
        PowerStatusUnit,
    },
};

/// Owner family of a live power/status instance.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusOwnerKind {
    /// The player entity.
    Player,
    /// A friendly ally or companion.
    Ally,
    /// An enemy or hostile creature.
    Enemy,
    /// A secondary entity such as a summon, orb, or encounter object.
    Secondary,
}

/// Owner identity and optional visible label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusOwner {
    /// Owner family.
    pub kind: PowerStatusOwnerKind,
    /// Stable entity identity.
    pub id: PowerStatusOwnerId,
    /// Localized owner label when visible.
    pub label: PowerStatusField<String>,
}

/// Source families that can create or apply a status.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PowerStatusSourceKind {
    /// A card or card effect.
    Card,
    /// A relic or relic effect.
    Relic,
    /// A character or passive source.
    Character,
    /// An enemy or enemy intent.
    Enemy,
    /// An event or room effect.
    Event,
    /// An environment or encounter rule.
    Environment,
    /// A source-defined family.
    Custom(String),
    /// Source could not classify the creator.
    Unknown,
}

/// Visible source/creator reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusSourceReference {
    /// Source family.
    pub kind: PowerStatusSourceKind,
    /// Stable source identity.
    pub id: PowerStatusOwnerId,
    /// Optional localized source label.
    pub label: PowerStatusField<String>,
}

/// Typed amount for a live power/status instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusAmount {
    /// Amountless marker; normally represented by `NotApplicable` at the field layer.
    Amountless,
    /// One signed integer with a unit.
    Integer {
        /// Numeric value.
        value: i64,
        /// Amount unit.
        unit: PowerStatusUnit,
    },
    /// Boolean state with a unit or label.
    Boolean {
        /// Boolean value.
        value: bool,
        /// Amount unit.
        unit: PowerStatusUnit,
    },
    /// Fixed-point value with decimal places.
    Decimal {
        /// Scaled integer value.
        value: i64,
        /// Number of decimal places.
        scale: u8,
        /// Amount unit.
        unit: PowerStatusUnit,
    },
    /// Bounded textual amount/state.
    Text {
        /// Text value.
        value: String,
        /// Amount unit.
        unit: PowerStatusUnit,
    },
    /// Multiple named counters with independent availability.
    Counters(Vec<PowerStatusCounterValue>),
    /// Owner-defined typed amount retained without executable semantics.
    Custom {
        /// Owner-defined amount kind.
        kind: String,
        /// Bounded source value.
        value: String,
        /// Optional unit.
        unit: Option<PowerStatusUnit>,
    },
}

/// One live named counter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusCounterValue {
    /// Static counter identity.
    pub id: String,
    /// Explicitly available or unavailable value.
    pub value: PowerStatusField<i64>,
    /// Unit copied from the static declaration.
    pub unit: PowerStatusUnit,
}

/// Duration state observed for one live instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusDurationState {
    /// No duration is active.
    Permanent,
    /// Visible remaining counter.
    Remaining {
        /// Remaining amount.
        value: i64,
        /// Counter unit.
        unit: PowerStatusUnit,
    },
    /// Expires at a known boundary.
    Boundary {
        /// Boundary copied from the source.
        reset: PowerStatusReset,
    },
    /// Expires when a visible condition is met.
    Condition {
        /// Stable condition ID.
        id: String,
        /// Localized condition label.
        label: String,
    },
    /// Duration is source-defined but not classifiable.
    Unknown,
}

/// Pending visible expiry marker, separate from the static duration rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PowerStatusPendingExpiry {
    /// Expires at a boundary.
    Boundary { reset: PowerStatusReset },
    /// Expires after a remaining counter.
    Remaining {
        /// Remaining amount.
        value: i64,
        /// Counter unit.
        unit: PowerStatusUnit,
    },
    /// Expires when a visible condition is met.
    Condition { id: String, label: String },
    /// Source reported a pending expiry without a supported shape.
    Unknown,
}

/// Source-owned live instance before a snapshot reference is attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusInstanceInput {
    /// Stable live instance identity, distinct from the definition ID.
    pub instance_id: String,
    /// Static definition identity.
    pub definition_id: String,
    /// Owner identity and family.
    pub owner: PowerStatusOwner,
    /// Visible source/creator when supplied.
    pub source: PowerStatusField<PowerStatusSourceReference>,
    /// Typed amount and explicit availability.
    pub amount: PowerStatusField<PowerStatusAmount>,
    /// Current duration and explicit availability.
    pub duration: PowerStatusField<PowerStatusDurationState>,
    /// Application ordering when the host exposes it.
    pub application_order: PowerStatusField<u32>,
    /// Whether the instance is currently active.
    pub active: PowerStatusField<bool>,
    /// Whether the instance is currently suppressed.
    pub suppressed: PowerStatusField<bool>,
    /// Pending visible expiry information.
    pub pending_expiry: PowerStatusField<PowerStatusPendingExpiry>,
}

/// Coherent live read input with one explicit identity fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PowerStatusLiveSnapshotInput {
    /// Manifest/instance/run/snapshot/epoch witness.
    pub binding: PowerStatusLiveBinding,
    /// Owned instances in source order.
    pub instances: Vec<PowerStatusInstanceInput>,
}

/// Returns whether a live duration state matches a static duration rule.
pub(super) fn duration_rule_matches(
    rule: &PowerStatusDurationRule,
    value: &PowerStatusDurationState,
) -> bool {
    match (rule, value) {
        (PowerStatusDurationRule::Permanent, PowerStatusDurationState::Permanent)
        | (PowerStatusDurationRule::Unknown, PowerStatusDurationState::Unknown) => true,
        (
            PowerStatusDurationRule::Counter { unit },
            PowerStatusDurationState::Remaining { unit: observed, .. },
        ) => unit == observed,
        (
            PowerStatusDurationRule::Boundary(expected),
            PowerStatusDurationState::Boundary { reset },
        ) => expected == reset,
        (PowerStatusDurationRule::Condition(_), PowerStatusDurationState::Condition { .. })
        | (PowerStatusDurationRule::Event(_), PowerStatusDurationState::Unknown) => true,
        _ => false,
    }
}
