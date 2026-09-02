// SPDX-License-Identifier: MIT

use super::artifact::{
    RUNTIME_V2_MAX_GENERATION, RUNTIME_V2_MAX_LEASE_EPOCH, RUNTIME_V2_PROTOCOL_VERSION,
    RUNTIME_V2_SCHEMA_DIGEST,
};
use super::context::RuntimeV2Context;
use super::contract::{
    RuntimeV2Action, RuntimeV2EffectWitness, RuntimeV2Kind, RuntimeV2Message, RuntimeV2Observation,
    RuntimeV2Provenance, RuntimeV2Status, RuntimeV2ValidationError, validate_identity,
};

impl RuntimeV2Message {
    /// Creates a state request.
    #[must_use]
    pub fn state_request(context: &RuntimeV2Context, generation: u64) -> Self {
        Self::base(context, generation, RuntimeV2Kind::StateRequest)
    }

    /// Creates an argument-free `end_turn` request.
    #[must_use]
    pub fn action_request(
        context: &RuntimeV2Context,
        generation: u64,
        operation_id: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: Some(operation_id.into()),
            action: Some(RuntimeV2Action::end_turn()),
            ..Self::base(context, generation, RuntimeV2Kind::ActionRequest)
        }
    }

    /// Creates a reconciliation request for an existing operation.
    #[must_use]
    pub fn reconcile_request(
        context: &RuntimeV2Context,
        generation: u64,
        operation_id: impl Into<String>,
    ) -> Self {
        Self {
            operation_id: Some(operation_id.into()),
            ..Self::base(context, generation, RuntimeV2Kind::ReconcileRequest)
        }
    }

    pub(crate) fn state_response(request: &Self, observation: RuntimeV2Observation) -> Self {
        Self {
            observation: Some(observation),
            ..Self::base(
                &request.context(),
                observation.generation,
                RuntimeV2Kind::StateResponse,
            )
        }
    }

    pub(crate) fn action_response(
        request: &Self,
        status: RuntimeV2Status,
        generation: u64,
        observation: Option<RuntimeV2Observation>,
        effect_witness: Option<RuntimeV2EffectWitness>,
        error_code: Option<&str>,
    ) -> Self {
        Self {
            operation_id: request.operation_id.clone(),
            action: request.action.clone(),
            status: Some(status),
            observation,
            effect_witness,
            error_code: error_code.map(str::to_owned),
            ..Self::base(
                &request.context(),
                generation,
                RuntimeV2Kind::ActionResponse,
            )
        }
    }

    pub(crate) fn reconcile_response(
        request: &Self,
        action: &RuntimeV2Action,
        status: RuntimeV2Status,
        generation: u64,
        observation: Option<RuntimeV2Observation>,
        effect_witness: Option<RuntimeV2EffectWitness>,
        error_code: Option<&str>,
    ) -> Self {
        Self {
            operation_id: request.operation_id.clone(),
            action: Some(action.clone()),
            status: Some(status),
            observation,
            effect_witness,
            error_code: error_code.map(str::to_owned),
            ..Self::base(
                &request.context(),
                generation,
                RuntimeV2Kind::ReconcileResponse,
            )
        }
    }

    /// Validates the complete message shape and all bounded values.
    pub fn validate(&self) -> Result<(), RuntimeV2ValidationError> {
        if self.protocol_version != RUNTIME_V2_PROTOCOL_VERSION
            || self.schema_digest != RUNTIME_V2_SCHEMA_DIGEST
        {
            return Err(RuntimeV2ValidationError::Metadata);
        }
        self.provenance.validate()?;
        validate_identity(&self.correlation_id)?;
        validate_identity(&self.instance_id)?;
        validate_identity(&self.session_id)?;
        validate_identity(&self.lease_id)?;
        if self.lease_epoch > RUNTIME_V2_MAX_LEASE_EPOCH {
            return Err(RuntimeV2ValidationError::LeaseEpochBounds);
        }
        if self.generation > RUNTIME_V2_MAX_GENERATION {
            return Err(RuntimeV2ValidationError::GenerationBounds);
        }
        if let Some(operation_id) = &self.operation_id {
            validate_identity(operation_id)?;
        }
        if let Some(error_code) = &self.error_code {
            validate_identity(error_code)?;
        }
        if let Some(action) = &self.action {
            action.validate()?;
        }
        if let Some(observation) = self.observation {
            observation.validate()?;
        }
        if let Some(witness) = &self.effect_witness {
            witness.validate()?;
        }
        self.validate_shape()
    }

    pub(crate) fn validate_request(&self) -> Result<(), RuntimeV2ValidationError> {
        self.validate()?;
        match self.kind {
            RuntimeV2Kind::StateRequest
            | RuntimeV2Kind::ActionRequest
            | RuntimeV2Kind::ReconcileRequest => Ok(()),
            RuntimeV2Kind::StateResponse
            | RuntimeV2Kind::ActionResponse
            | RuntimeV2Kind::ReconcileResponse => Err(RuntimeV2ValidationError::RequestShape),
        }
    }

    pub(crate) fn context(&self) -> RuntimeV2Context {
        RuntimeV2Context::new(
            self.correlation_id.clone(),
            self.instance_id.clone(),
            self.session_id.clone(),
            self.lease_id.clone(),
            self.lease_epoch,
        )
    }

    fn base(context: &RuntimeV2Context, generation: u64, kind: RuntimeV2Kind) -> Self {
        Self {
            protocol_version: RUNTIME_V2_PROTOCOL_VERSION.to_owned(),
            schema_digest: RUNTIME_V2_SCHEMA_DIGEST.to_owned(),
            provenance: RuntimeV2Provenance::default(),
            correlation_id: context.correlation_id.clone(),
            instance_id: context.instance_id.clone(),
            session_id: context.session_id.clone(),
            lease_id: context.lease_id.clone(),
            lease_epoch: context.lease_epoch,
            generation,
            kind,
            operation_id: None,
            action: None,
            observation: None,
            status: None,
            effect_witness: None,
            error_code: None,
        }
    }

    fn validate_shape(&self) -> Result<(), RuntimeV2ValidationError> {
        match self.kind {
            RuntimeV2Kind::StateRequest => {
                if self.operation_id.is_none()
                    && self.action.is_none()
                    && self.observation.is_none()
                    && self.status.is_none()
                    && self.effect_witness.is_none()
                    && self.error_code.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::RequestShape)
                }
            }
            RuntimeV2Kind::StateResponse => {
                if self.observation.is_some()
                    && self.operation_id.is_none()
                    && self.action.is_none()
                    && self.status.is_none()
                    && self.effect_witness.is_none()
                    && self.error_code.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::ResponseShape)
                }
            }
            RuntimeV2Kind::ActionRequest => {
                if self.operation_id.is_some()
                    && self.action.is_some()
                    && self.observation.is_none()
                    && self.status.is_none()
                    && self.effect_witness.is_none()
                    && self.error_code.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::RequestShape)
                }
            }
            RuntimeV2Kind::ActionResponse => {
                if self.operation_id.is_none() || self.action.is_none() || self.status.is_none() {
                    return Err(RuntimeV2ValidationError::ResponseShape);
                }
                self.validate_result_shape()
            }
            RuntimeV2Kind::ReconcileRequest => {
                if self.operation_id.is_some()
                    && self.action.is_none()
                    && self.observation.is_none()
                    && self.status.is_none()
                    && self.effect_witness.is_none()
                    && self.error_code.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::RequestShape)
                }
            }
            RuntimeV2Kind::ReconcileResponse => {
                if self.operation_id.is_none() || self.action.is_none() || self.status.is_none() {
                    return Err(RuntimeV2ValidationError::ResponseShape);
                }
                self.validate_result_shape()
            }
        }
    }

    fn validate_result_shape(&self) -> Result<(), RuntimeV2ValidationError> {
        let status = self.status.ok_or(RuntimeV2ValidationError::ResponseShape)?;
        match status {
            RuntimeV2Status::Accepted => {
                if self.observation.is_some()
                    && self.error_code.is_none()
                    && self.effect_witness.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::ResponseShape)
                }
            }
            RuntimeV2Status::Settled => {
                let observation = self
                    .observation
                    .ok_or(RuntimeV2ValidationError::SettledEvidence)?;
                let witness = self
                    .effect_witness
                    .as_ref()
                    .ok_or(RuntimeV2ValidationError::SettledEvidence)?;
                if observation.generation != self.generation
                    || witness.generation != self.generation
                    || self.error_code.is_some()
                {
                    return Err(RuntimeV2ValidationError::SettledEvidence);
                }
                Ok(())
            }
            RuntimeV2Status::Rejected | RuntimeV2Status::Cancelled => {
                if self.observation.is_some()
                    && self.error_code.is_some()
                    && self.effect_witness.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::ResponseShape)
                }
            }
            RuntimeV2Status::Unknown => {
                if self.observation.is_none()
                    && self.error_code.is_some()
                    && self.effect_witness.is_none()
                {
                    Ok(())
                } else {
                    Err(RuntimeV2ValidationError::ResponseShape)
                }
            }
        }
    }
}
