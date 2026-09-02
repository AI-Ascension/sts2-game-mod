// SPDX-License-Identifier: MIT

use super::contract::{
    PocAction, PocCorePort, PocCoreState, PocMessage, PocModError, PocObservation, PocRoute,
    PocStatus, PocValidationError,
};
use crate::protocol_artifact::{POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, verify_poc_artifact};

/// A settled effect witness owned by the game-mod boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectWitness {
    pub protocol_version: &'static str,
    pub schema_digest: &'static str,
    pub correlation_id: String,
    pub instance_id: String,
    pub generation: u64,
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

    /// Returns the core projection without exposing the core implementation.
    #[must_use]
    pub fn snapshot(&self) -> PocCoreState {
        self.core.snapshot()
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
        let request: PocMessage =
            serde_json::from_slice(body).map_err(|_| PocModError::Encoding)?;
        match request.validate_request(route) {
            Ok(()) => {}
            Err(PocValidationError::Metadata | PocValidationError::Provenance) => {
                return Err(PocModError::ArtifactMismatch);
            }
            Err(_) => return Err(PocModError::MalformedRequest),
        }

        let state = self.core.snapshot();
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
            PocRoute::Action => self.handle_action(request, state),
        };
        serde_json::to_vec(&response).map_err(|_| PocModError::Encoding)
    }

    fn handle_action(&mut self, request: PocMessage, state: PocCoreState) -> PocMessage {
        let Some(action) = request.action.clone() else {
            return rejected_response(
                &request,
                state,
                PocAction::new("use_budget", 0),
                "sts2.game-core/missing_action",
            );
        };
        let result = self.core.apply(request.generation, &action);
        let (status, output_state, error_code) = match result {
            Ok(output_state) => {
                self.witnesses.push(EffectWitness {
                    protocol_version: POC_PROTOCOL_VERSION,
                    schema_digest: POC_SCHEMA_DIGEST,
                    correlation_id: request.correlation_id.clone(),
                    instance_id: request.instance_id.clone(),
                    generation: output_state.generation,
                    action_id: action.action_id.clone(),
                    settled: true,
                });
                (PocStatus::Accepted, output_state, None)
            }
            Err(error) => (
                PocStatus::Rejected,
                self.core.snapshot(),
                Some(error.code().to_owned()),
            ),
        };
        self.records.push(PocBoundaryRecord {
            protocol_version: POC_PROTOCOL_VERSION,
            schema_digest: POC_SCHEMA_DIGEST,
            correlation_id: request.correlation_id.clone(),
            instance_id: request.instance_id.clone(),
            generation: output_state.generation,
            status: Some(status),
        });
        PocMessage::action_response(
            &request.correlation_id,
            &request.instance_id,
            output_state.generation,
            action,
            status,
            observation(output_state),
            error_code,
        )
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
