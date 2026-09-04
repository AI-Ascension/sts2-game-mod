// SPDX-License-Identifier: MIT

use super::*;

/// Requests and responses for the semantic gameplay adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayMessageKind {
    StateRequest,
    StateResponse,
    LegalActionsRequest,
    LegalActionsResponse,
    DispatchActionRequest,
    DispatchActionResponse,
    WaitRequest,
    WaitResponse,
    ReobserveRequest,
    ReobserveResponse,
    RecoverRequest,
    RecoverResponse,
}

/// Provenance for the owner-local release-like artifact.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayProvenance {
    pub artifact: String,
    pub source: String,
    pub generator: String,
}

impl Default for RuntimeV3GameplayProvenance {
    fn default() -> Self {
        Self {
            artifact: RUNTIME_V3_GAMEPLAY_ARTIFACT.to_owned(),
            source: RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE.to_owned(),
            generator: RUNTIME_V3_GAMEPLAY_GENERATOR.to_owned(),
        }
    }
}

/// Identity preserved across gateway, MCP, host, and harness boundaries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeV3GameplayContext {
    pub correlation_id: String,
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: u64,
}

impl RuntimeV3GameplayContext {
    /// Creates context metadata without granting authorization.
    #[must_use]
    pub fn new(
        correlation_id: impl Into<String>,
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
    ) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
        }
    }
}

/// Complete wire envelope. Optional fields are constrained by `validate` according to `kind`.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayMessage {
    pub protocol_version: String,
    pub schema_digest: String,
    pub provenance: RuntimeV3GameplayProvenance,
    pub correlation_id: String,
    pub instance_id: String,
    pub session_id: String,
    pub lease_id: String,
    pub lease_epoch: u64,
    pub generation: u64,
    pub kind: RuntimeV3GameplayMessageKind,
    #[serde(deserialize_with = "required_nullable")]
    pub state_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub operation_id: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub observation: Option<RuntimeV3GameplayObservation>,
    #[serde(deserialize_with = "required_nullable")]
    pub legal_actions: Option<Vec<RuntimeV3GameplayLegalAction>>,
    #[serde(deserialize_with = "required_nullable")]
    pub action: Option<RuntimeV3GameplayLegalAction>,
    #[serde(deserialize_with = "required_nullable")]
    pub status: Option<RuntimeV3GameplayStatus>,
    #[serde(deserialize_with = "required_nullable")]
    pub transition: Option<RuntimeV3GameplayTransitionWitness>,
    #[serde(deserialize_with = "required_nullable")]
    pub error_code: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub wait_for_millis: Option<u32>,
    #[serde(deserialize_with = "required_nullable")]
    pub wait_outcome: Option<RuntimeV3GameplayWaitOutcome>,
    #[serde(deserialize_with = "required_nullable")]
    pub recovery: Option<RuntimeV3GameplayRecovery>,
}

impl RuntimeV3GameplayMessage {
    /// Creates a state request.
    #[must_use]
    pub fn state_request(context: RuntimeV3GameplayContext, generation: u64) -> Self {
        Self::base(
            context,
            generation,
            RuntimeV3GameplayMessageKind::StateRequest,
        )
    }

    /// Creates a legal-action catalog request bound to one state generation.
    #[must_use]
    pub fn legal_actions_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        state_id: impl Into<String>,
    ) -> Self {
        Self {
            state_id: Some(state_id.into()),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::LegalActionsRequest,
            )
        }
    }

    /// Creates one typed dispatch request. The host still rechecks catalog membership.
    #[must_use]
    pub fn dispatch_action_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        state_id: impl Into<String>,
        operation_id: impl Into<String>,
        action: RuntimeV3GameplayLegalAction,
    ) -> Self {
        Self {
            state_id: Some(state_id.into()),
            operation_id: Some(operation_id.into()),
            action: Some(action),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::DispatchActionRequest,
            )
        }
    }

    /// Creates a fresh observation request after a contradiction or stale response.
    #[must_use]
    pub fn reobserve_request(context: RuntimeV3GameplayContext, generation: u64) -> Self {
        Self::base(
            context,
            generation,
            RuntimeV3GameplayMessageKind::ReobserveRequest,
        )
    }

    /// Creates an explicitly safe recovery request.
    #[must_use]
    pub fn recover_request(
        context: RuntimeV3GameplayContext,
        generation: u64,
        recovery: RuntimeV3GameplayRecovery,
    ) -> Self {
        Self {
            recovery: Some(recovery),
            ..Self::base(
                context,
                generation,
                RuntimeV3GameplayMessageKind::RecoverRequest,
            )
        }
    }

    /// Validates metadata, bounds, and kind-specific shape.
    pub fn validate(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        if self.protocol_version != RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION
            || self.schema_digest != RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST
        {
            return Err(RuntimeV3GameplayValidationError::Metadata);
        }
        if self.provenance != RuntimeV3GameplayProvenance::default() {
            return Err(RuntimeV3GameplayValidationError::Provenance);
        }
        for identity in [
            &self.correlation_id,
            &self.instance_id,
            &self.session_id,
            &self.lease_id,
        ] {
            if !valid_identity(identity) {
                return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
            }
        }
        if self.lease_epoch > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || self.generation > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
        {
            return Err(RuntimeV3GameplayValidationError::GenerationBounds);
        }
        for identity in [&self.state_id, &self.operation_id, &self.error_code]
            .into_iter()
            .flatten()
        {
            if !valid_identity(identity) {
                return Err(RuntimeV3GameplayValidationError::InvalidIdentity);
            }
        }
        if let Some(observation) = &self.observation {
            observation.validate()?;
        }
        if let Some(actions) = &self.legal_actions {
            validate_actions(actions)?;
        }
        if let Some(action) = &self.action {
            action.validate()?;
        }
        if let Some(transition) = &self.transition {
            transition.validate()?;
        }
        if let Some(recovery) = &self.recovery {
            recovery.validate()?;
        }
        super::shape::validate_shape(self)
    }

    /// Rejects response messages when a caller expects a request.
    pub fn validate_request(&self) -> Result<(), RuntimeV3GameplayValidationError> {
        self.validate()?;
        if matches!(
            self.kind,
            RuntimeV3GameplayMessageKind::StateRequest
                | RuntimeV3GameplayMessageKind::LegalActionsRequest
                | RuntimeV3GameplayMessageKind::DispatchActionRequest
                | RuntimeV3GameplayMessageKind::WaitRequest
                | RuntimeV3GameplayMessageKind::ReobserveRequest
                | RuntimeV3GameplayMessageKind::RecoverRequest
        ) {
            Ok(())
        } else {
            Err(RuntimeV3GameplayValidationError::ResultShape)
        }
    }

    /// Local construction helper; callers must validate the completed envelope.
    #[must_use]
    pub fn base(
        context: RuntimeV3GameplayContext,
        generation: u64,
        kind: RuntimeV3GameplayMessageKind,
    ) -> Self {
        Self {
            protocol_version: RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION.to_owned(),
            schema_digest: RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST.to_owned(),
            provenance: RuntimeV3GameplayProvenance::default(),
            correlation_id: context.correlation_id,
            instance_id: context.instance_id,
            session_id: context.session_id,
            lease_id: context.lease_id,
            lease_epoch: context.lease_epoch,
            generation,
            kind,
            state_id: None,
            operation_id: None,
            observation: None,
            legal_actions: None,
            action: None,
            status: None,
            transition: None,
            error_code: None,
            wait_for_millis: None,
            wait_outcome: None,
            recovery: None,
        }
    }
}

fn validate_actions(
    actions: &[RuntimeV3GameplayLegalAction],
) -> Result<(), RuntimeV3GameplayValidationError> {
    if actions.len() > RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS {
        return Err(RuntimeV3GameplayValidationError::CollectionBounds);
    }
    for (index, action) in actions.iter().enumerate() {
        action.validate()?;
        if actions[..index]
            .iter()
            .any(|previous| previous.action_id == action.action_id)
        {
            return Err(RuntimeV3GameplayValidationError::DuplicateAction);
        }
    }
    Ok(())
}
