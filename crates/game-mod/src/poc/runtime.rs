// SPDX-License-Identifier: MIT

use super::contract::{
    PocAction, PocCorePort, PocCoreState, PocMessage, PocObservation, PocRoute, PocStatus,
    PocValidationError,
};
use crate::protocol_artifact::{
    ArtifactError, POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, verify_poc_artifact,
};

/// Maximum encoded request size accepted by the POC mapping.
pub const POC_MAX_REQUEST_BYTES: usize = 4 * 1024;
/// Maximum number of boundary records retained by one POC mapping.
pub const POC_MAX_EVIDENCE_RECORDS: usize = 1024;

/// Deterministic failures at the game-mod POC seam.
#[derive(Debug, Eq, PartialEq)]
pub enum PocModError {
    ArtifactLoad(ArtifactError),
    ArtifactMismatch,
    MalformedRequest,
    RequestTooLarge,
    EvidenceLimit,
    CoreStateBounds,
    CoreTransition,
    Encoding,
}

impl std::fmt::Display for PocModError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::ArtifactLoad(_) => "the copied POC artifact could not be loaded",
            Self::ArtifactMismatch => "the request does not use the copied POC artifact",
            Self::MalformedRequest => "the POC request is malformed",
            Self::RequestTooLarge => "the POC request exceeds its byte bound",
            Self::EvidenceLimit => "the POC evidence limit has been reached",
            Self::CoreStateBounds => "the core returned an out-of-bounds state",
            Self::CoreTransition => "the core transition did not settle atomically",
            Self::Encoding => "the POC message could not be encoded",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PocModError {}

/// A settled effect witness owned by the game-mod boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectWitness {
    pub protocol_version: &'static str,
    pub schema_digest: &'static str,
    pub correlation_id: String,
    pub instance_id: String,
    pub previous_generation: u64,
    pub generation: u64,
    pub available_units_before: u16,
    pub available_units_after: u16,
    pub settled_effects: u16,
    pub action_id: String,
    pub settled: bool,
}

/// A record proving which message crossed the mod-owned boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PocBoundaryRecord {
    pub protocol_version: &'static str,
    pub schema_digest: &'static str,
    pub correlation_id: String,
    pub instance_id: String,
    pub generation: u64,
    pub status: Option<PocStatus>,
}

/// The game-mod mapping from the copied message artifact to a core capability.
#[derive(Debug)]
pub struct PocMod<C> {
    core: C,
    witnesses: Vec<EffectWitness>,
    records: Vec<PocBoundaryRecord>,
}

impl<C: PocCorePort> PocMod<C> {
    /// Loads the checked-in artifact before constructing the mapping.
    pub fn new(core: C) -> Result<Self, PocModError> {
        verify_poc_artifact().map_err(PocModError::ArtifactLoad)?;
        Ok(Self {
            core,
            witnesses: Vec::new(),
            records: Vec::new(),
        })
    }

    /// Returns a validated core projection without exposing the core implementation.
    pub fn snapshot(&self) -> Result<PocCoreState, PocModError> {
        self.checked_snapshot()
    }

    /// Returns settled effect witnesses produced by accepted actions.
    #[must_use]
    pub fn witnesses(&self) -> &[EffectWitness] {
        &self.witnesses
    }

    /// Returns all valid requests that crossed this boundary.
    #[must_use]
    pub fn records(&self) -> &[PocBoundaryRecord] {
        &self.records
    }

    /// Translates one validated POC request and returns its encoded response.
    pub fn handle(&mut self, route: PocRoute, body: &[u8]) -> Result<Vec<u8>, PocModError> {
        if body.len() > POC_MAX_REQUEST_BYTES {
            return Err(PocModError::RequestTooLarge);
        }
        let request: PocMessage =
            serde_json::from_slice(body).map_err(|_| PocModError::MalformedRequest)?;
        match request.validate_request(route) {
            Ok(()) => {}
            Err(PocValidationError::Metadata | PocValidationError::Provenance) => {
                return Err(PocModError::ArtifactMismatch);
            }
            Err(_) => return Err(PocModError::MalformedRequest),
        }

        self.ensure_evidence_capacity(route)?;
        let state = self.checked_snapshot()?;
        let response = match route {
            PocRoute::State => {
                self.records.push(PocBoundaryRecord {
                    protocol_version: POC_PROTOCOL_VERSION,
                    schema_digest: POC_SCHEMA_DIGEST,
                    correlation_id: request.correlation_id.clone(),
                    instance_id: request.instance_id.clone(),
                    generation: state.generation,
                    status: None,
                });
                PocMessage::state_response(
                    &request.correlation_id,
                    &request.instance_id,
                    state.generation,
                    observation(state),
                )
            }
            PocRoute::Action => self.handle_action(request, state)?,
        };
        serde_json::to_vec(&response).map_err(|_| PocModError::Encoding)
    }

    fn handle_action(
        &mut self,
        request: PocMessage,
        state: PocCoreState,
    ) -> Result<PocMessage, PocModError> {
        let Some(action) = request.action.clone() else {
            return Ok(rejected_response(
                &request,
                state,
                PocAction::new("use_budget", 0),
                "sts2.game-core/missing_action",
            ));
        };
        let result = self.core.apply(request.generation, &action);
        let (status, output_state, error_code) = match result {
            Ok(output_state) => {
                output_state
                    .validate()
                    .map_err(|_| PocModError::CoreStateBounds)?;
                let observed_state = self.checked_snapshot()?;
                if observed_state != output_state
                    || !accepted_transition_is_settled(state, output_state, &action)
                {
                    return Err(PocModError::CoreTransition);
                }
                self.witnesses.push(EffectWitness {
                    protocol_version: POC_PROTOCOL_VERSION,
                    schema_digest: POC_SCHEMA_DIGEST,
                    correlation_id: request.correlation_id.clone(),
                    instance_id: request.instance_id.clone(),
                    previous_generation: state.generation,
                    generation: output_state.generation,
                    available_units_before: state.available_units,
                    available_units_after: output_state.available_units,
                    settled_effects: output_state.settled_effects,
                    action_id: action.action_id.clone(),
                    settled: true,
                });
                (PocStatus::Accepted, output_state, None)
            }
            Err(error) => {
                let observed_state = self.checked_snapshot()?;
                if observed_state != state {
                    return Err(PocModError::CoreTransition);
                }
                (
                    PocStatus::Rejected,
                    observed_state,
                    Some(error.code().to_owned()),
                )
            }
        };
        self.records.push(PocBoundaryRecord {
            protocol_version: POC_PROTOCOL_VERSION,
            schema_digest: POC_SCHEMA_DIGEST,
            correlation_id: request.correlation_id.clone(),
            instance_id: request.instance_id.clone(),
            generation: output_state.generation,
            status: Some(status),
        });
        Ok(PocMessage::action_response(
            &request.correlation_id,
            &request.instance_id,
            output_state.generation,
            action,
            status,
            observation(output_state),
            error_code,
        ))
    }

    fn checked_snapshot(&self) -> Result<PocCoreState, PocModError> {
        let state = self.core.snapshot();
        state.validate().map_err(|_| PocModError::CoreStateBounds)?;
        Ok(state)
    }

    fn ensure_evidence_capacity(&self, route: PocRoute) -> Result<(), PocModError> {
        if self.records.len() >= POC_MAX_EVIDENCE_RECORDS
            || (route == PocRoute::Action && self.witnesses.len() >= POC_MAX_EVIDENCE_RECORDS)
        {
            return Err(PocModError::EvidenceLimit);
        }
        Ok(())
    }
}

fn observation(state: PocCoreState) -> PocObservation {
    PocObservation {
        available_units: state.available_units,
        settled_effects: state.settled_effects,
    }
}

fn rejected_response(
    request: &PocMessage,
    state: PocCoreState,
    action: PocAction,
    error_code: &str,
) -> PocMessage {
    PocMessage::action_response(
        &request.correlation_id,
        &request.instance_id,
        state.generation,
        action,
        PocStatus::Rejected,
        observation(state),
        Some(error_code.to_owned()),
    )
}

fn accepted_transition_is_settled(
    before: PocCoreState,
    after: PocCoreState,
    action: &PocAction,
) -> bool {
    action.units != 0
        && before.generation.checked_add(1) == Some(after.generation)
        && before.available_units.checked_sub(action.units) == Some(after.available_units)
        && before.settled_effects.checked_add(1) == Some(after.settled_effects)
}
