// SPDX-License-Identifier: MIT

use super::artifact::{
    RUNTIME_V2_ARTIFACT, RUNTIME_V2_GENERATOR, RUNTIME_V2_MAX_GENERATION,
    RUNTIME_V2_MAX_LEASE_EPOCH, RUNTIME_V2_MAX_TURN_INDEX, RUNTIME_V2_SCHEMA_SOURCE,
};

const MAX_IDENTITY_BYTES: usize = 128;

/// Runtime-v2 message kinds. Responses are kept distinct from requests so a response cannot be
/// accidentally admitted as new work.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV2Kind {
    /// Requests the current observation.
    StateRequest,
    /// Returns the current observation.
    StateResponse,
    /// Requests one admitted action.
    ActionRequest,
    /// Returns the action admission or execution result.
    ActionResponse,
    /// Requests the retained result for one operation.
    ReconcileRequest,
    /// Returns the retained operation result.
    ReconcileResponse,
}

/// Runtime-v2 operation statuses.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV2Status {
    /// The operation entered the bounded queue; no action effect is claimed.
    Accepted,
    /// The operation has a fresh observation and effect witness.
    Settled,
    /// The operation was rejected without mutation.
    Rejected,
    /// Delivery or execution outcome is not known to the caller and must be reconciled.
    Unknown,
    /// A cancellation result. Admitted work is never undone by cancellation.
    Cancelled,
}

/// The three observation phases allowed by Runtime-v2.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub enum RuntimeV2CombatPhase {
    /// No combat is active.
    #[serde(rename = "outside_combat")]
    OutsideCombat,
    /// The player may submit `end_turn`.
    #[serde(rename = "combat/player_turn")]
    PlayerTurn,
    /// The player may not submit `end_turn`.
    #[serde(rename = "combat/enemy_turn")]
    EnemyTurn,
}

/// The bounded state projection exchanged by the fake host seam.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV2Observation {
    /// Current combat lifecycle phase.
    pub combat_phase: RuntimeV2CombatPhase,
    /// Bounded turn index.
    pub turn_index: u16,
    /// Whether the host is ready to service work.
    pub host_ready: bool,
    /// Generation used for optimistic concurrency and witnesses.
    pub generation: u64,
}

impl RuntimeV2Observation {
    /// Validates the bounded observation projection.
    pub fn validate(self) -> Result<(), RuntimeV2ValidationError> {
        if self.turn_index > RUNTIME_V2_MAX_TURN_INDEX {
            return Err(RuntimeV2ValidationError::TurnIndexBounds);
        }
        if self.generation > RUNTIME_V2_MAX_GENERATION {
            return Err(RuntimeV2ValidationError::GenerationBounds);
        }
        Ok(())
    }
}

/// The only Runtime-v2 action. It deliberately has no argument field.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV2Action {
    /// The exact action identifier, which must be `end_turn`.
    pub action_id: String,
}

impl RuntimeV2Action {
    /// Creates the argument-free action allowed by Runtime-v2.
    #[must_use]
    pub fn end_turn() -> Self {
        Self {
            action_id: "end_turn".to_owned(),
        }
    }

    pub(super) fn validate(&self) -> Result<(), RuntimeV2ValidationError> {
        (self.action_id == "end_turn")
            .then_some(())
            .ok_or(RuntimeV2ValidationError::ActionBounds)
    }
}

/// The effect witness required for a settled `end_turn`.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV2EffectWitness {
    /// Stable witness kind.
    pub kind: String,
    /// Generation observed after the effect.
    pub generation: u64,
}

impl RuntimeV2EffectWitness {
    /// Creates the only settled effect witness allowed by Runtime-v2.
    #[must_use]
    pub fn turn_end_settled(generation: u64) -> Self {
        Self {
            kind: "turn_end_settled".to_owned(),
            generation,
        }
    }

    pub(super) fn validate(&self) -> Result<(), RuntimeV2ValidationError> {
        if self.kind != "turn_end_settled" || self.generation > RUNTIME_V2_MAX_GENERATION {
            return Err(RuntimeV2ValidationError::EffectWitness);
        }
        Ok(())
    }
}

/// Provenance carried by every Runtime-v2 message.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV2Provenance {
    /// Release-like artifact identity.
    pub artifact: String,
    /// Source schema path recorded by the artifact.
    pub source: String,
    /// Artifact generator identity.
    pub generator: String,
}

impl Default for RuntimeV2Provenance {
    fn default() -> Self {
        Self {
            artifact: RUNTIME_V2_ARTIFACT.to_owned(),
            source: RUNTIME_V2_SCHEMA_SOURCE.to_owned(),
            generator: RUNTIME_V2_GENERATOR.to_owned(),
        }
    }
}

impl RuntimeV2Provenance {
    pub(super) fn validate(&self) -> Result<(), RuntimeV2ValidationError> {
        if self.artifact != RUNTIME_V2_ARTIFACT
            || self.source != RUNTIME_V2_SCHEMA_SOURCE
            || self.generator != RUNTIME_V2_GENERATOR
        {
            return Err(RuntimeV2ValidationError::Provenance);
        }
        Ok(())
    }
}

/// Identity and lease values owned by the fake runtime instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeV2Identity {
    /// Isolated runtime instance identity.
    pub instance_id: String,
    /// Session identity.
    pub session_id: String,
    /// Lease identity.
    pub lease_id: String,
    /// Monotonic lease epoch.
    pub lease_epoch: u64,
}

impl RuntimeV2Identity {
    /// Creates the identity expected by one runtime instance.
    #[must_use]
    pub fn new(
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
    ) -> Self {
        Self {
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
        }
    }

    pub(crate) fn matches(&self, message: &RuntimeV2Message) -> bool {
        self.instance_id == message.instance_id
            && self.session_id == message.session_id
            && self.lease_id == message.lease_id
            && self.lease_epoch == message.lease_epoch
    }

    pub(crate) fn validate(&self) -> Result<(), RuntimeV2ValidationError> {
        validate_identity(&self.instance_id)?;
        validate_identity(&self.session_id)?;
        validate_identity(&self.lease_id)?;
        if self.lease_epoch > RUNTIME_V2_MAX_LEASE_EPOCH {
            return Err(RuntimeV2ValidationError::LeaseEpochBounds);
        }
        Ok(())
    }
}

/// One complete Runtime-v2 request or response envelope.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV2Message {
    /// Exact protocol version.
    pub protocol_version: String,
    /// Exact schema digest from the owner-local artifact seam.
    pub schema_digest: String,
    /// Artifact provenance.
    pub provenance: RuntimeV2Provenance,
    /// Request/response correlation identity.
    pub correlation_id: String,
    /// Isolated runtime instance identity.
    pub instance_id: String,
    /// Runtime session identity.
    pub session_id: String,
    /// Runtime lease identity.
    pub lease_id: String,
    /// Lease epoch used for fencing.
    pub lease_epoch: u64,
    /// Request or result generation.
    pub generation: u64,
    /// Message kind.
    pub kind: RuntimeV2Kind,
    /// Stable operation identity for action and reconcile messages.
    pub operation_id: Option<String>,
    /// Observation, required for state responses and settled results.
    pub observation: Option<RuntimeV2Observation>,
    /// Argument-free action, present only on action messages.
    pub action: Option<RuntimeV2Action>,
    /// Result status, present only on result messages.
    pub status: Option<RuntimeV2Status>,
    /// Stable machine-readable rejection or uncertainty code.
    pub error_code: Option<String>,
    /// Fresh effect witness required by settled results.
    pub effect_witness: Option<RuntimeV2EffectWitness>,
}

/// Deterministic validation failures for the Runtime-v2 wire contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV2ValidationError {
    /// Protocol version or schema digest differs from the owner-local artifact seam.
    Metadata,
    /// Provenance differs from the owner-local artifact seam.
    Provenance,
    /// An identity field is empty, too long, or contains an unsafe byte.
    InvalidIdentity,
    /// Generation is outside the bounded integer range.
    GenerationBounds,
    /// Lease epoch is outside the bounded integer range.
    LeaseEpochBounds,
    /// Turn index is above the contract bound.
    TurnIndexBounds,
    /// Action is not the argument-free `end_turn` action.
    ActionBounds,
    /// Effect witness is not the exact settled witness.
    EffectWitness,
    /// A request contains response-only fields or the wrong request kind.
    RequestShape,
    /// A response contains the wrong fields for its kind.
    ResponseShape,
    /// A settled result lacks a matching fresh-observation witness.
    SettledEvidence,
}

impl std::fmt::Display for RuntimeV2ValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("invalid Runtime-v2 message")
    }
}

impl std::error::Error for RuntimeV2ValidationError {}

pub(super) fn validate_identity(value: &str) -> Result<(), RuntimeV2ValidationError> {
    if value.is_empty()
        || value.len() > MAX_IDENTITY_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-/".contains(&byte))
    {
        return Err(RuntimeV2ValidationError::InvalidIdentity);
    }
    Ok(())
}
