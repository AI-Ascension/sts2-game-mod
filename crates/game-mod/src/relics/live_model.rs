// SPDX-License-Identifier: MIT

use super::definition::{RelicCounterState, RelicResolvedParameter};
use super::model::{RelicCounterReset, RelicField, RelicLiveBinding, RelicOwnerId, RelicUnit};

/// Live activation state kept separate from the static activation model.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicActivationState {
    /// Whether the relic is currently active.
    pub active: RelicField<bool>,
    /// Whether its current activation was consumed.
    pub used: RelicField<bool>,
}

/// Live accumulated value distinct from a resettable counter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicAccumulatedValue {
    /// Stable accumulated-value ID.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Typed unit.
    pub unit: RelicUnit,
    /// Lifetime semantics.
    pub reset: RelicCounterReset,
    /// Explicitly observed value.
    pub value: RelicField<i64>,
}

/// Live pending visible trigger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicPendingTrigger {
    /// Static trigger ID.
    pub id: String,
    /// Human-readable trigger label.
    pub label: String,
    /// Optional current condition.
    pub condition: Option<super::definition::RelicCondition>,
    /// Resolved visible parameters, linked by static IDs.
    pub parameters: Vec<RelicResolvedParameter>,
}

/// Source-owned live instance before a snapshot reference is attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicInstanceInput {
    /// Stable live instance identity, distinct from the definition ID.
    pub instance_id: String,
    /// Static definition identity.
    pub definition_id: String,
    /// Owning player/entity identity.
    pub owner_id: RelicOwnerId,
    /// Typed counter state.
    pub counters: RelicField<Vec<RelicCounterState>>,
    /// Current activation/used state.
    pub activation: RelicField<RelicActivationState>,
    /// Values accumulated under explicit lifetime semantics.
    pub accumulated: RelicField<Vec<RelicAccumulatedValue>>,
    /// Pending visible triggers.
    pub pending_triggers: RelicField<Vec<RelicPendingTrigger>>,
    /// Resolved visible description parameters.
    pub resolved_parameters: RelicField<Vec<RelicResolvedParameter>>,
}

/// Coherent live read input with an explicit identity fence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelicLiveSnapshotInput {
    /// Manifest/run/epoch witness for this collection.
    pub binding: RelicLiveBinding,
    /// Owned live instances in source order.
    pub instances: Vec<RelicInstanceInput>,
}
