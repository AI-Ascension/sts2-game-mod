// SPDX-License-Identifier: MIT

use crate::protocol_artifact::{
    ArtifactError, POC_ARTIFACT, POC_GENERATOR, POC_MAX_SETTLED_EFFECTS, POC_MAX_UNITS,
    POC_PROTOCOL_VERSION, POC_SCHEMA_DIGEST, POC_SCHEMA_SOURCE,
};

/// The two fixed routes owned by the game-mod POC seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PocRoute {
    State,
    Action,
}

/// The four message shapes in the copied POC contract.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PocMessageKind {
    StateRequest,
    StateResponse,
    ActionRequest,
    ActionResponse,
}

/// The only result statuses in the copied POC contract.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PocStatus {
    Accepted,
    Rejected,
}

/// The one typed action exposed by the POC seam.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct PocAction {
    pub action_id: String,
    pub units: u16,
}

impl PocAction {
    /// Creates the typed action used by the fake slice.
    #[must_use]
    pub fn new(action_id: &str, units: u16) -> Self {
        Self {
            action_id: action_id.to_owned(),
            units,
        }
    }

    fn validate(&self) -> Result<(), PocValidationError> {
        if self.action_id != "use_budget" || self.units > POC_MAX_UNITS {
            return Err(PocValidationError::ActionBounds);
        }
        Ok(())
    }
}

/// The bounded observation translated at the mod/core boundary.
#[derive(Clone, Copy, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct PocObservation {
    pub available_units: u16,
    pub settled_effects: u16,
}

impl PocObservation {
    /// Validates the bounded observation values.
    pub fn validate(&self) -> Result<(), PocValidationError> {
        if self.available_units > POC_MAX_UNITS || self.settled_effects > POC_MAX_SETTLED_EFFECTS {
            return Err(PocValidationError::ObservationBounds);
        }
        Ok(())
    }
}

/// Provenance carried by every POC message.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct PocProvenance {
    pub artifact: String,
    pub source: String,
    pub generator: String,
}

impl Default for PocProvenance {
    fn default() -> Self {
        Self {
            artifact: POC_ARTIFACT.to_owned(),
            source: POC_SCHEMA_SOURCE.to_owned(),
            generator: POC_GENERATOR.to_owned(),
        }
    }
}

impl PocProvenance {
    fn validate(&self) -> Result<(), PocValidationError> {
        if self.artifact != POC_ARTIFACT
            || self.source != POC_SCHEMA_SOURCE
            || self.generator != POC_GENERATOR
        {
            return Err(PocValidationError::Provenance);
        }
        Ok(())
    }
}

/// A complete request or response in the copied POC contract.
#[derive(Clone, Debug, serde::Deserialize, Eq, PartialEq, serde::Serialize)]
pub struct PocMessage {
    pub protocol_version: String,
    pub schema_digest: String,
    pub provenance: PocProvenance,
    pub correlation_id: String,
    pub instance_id: String,
    pub generation: u64,
    pub kind: PocMessageKind,
    pub observation: Option<PocObservation>,
    pub action: Option<PocAction>,
    pub status: Option<PocStatus>,
    pub error_code: Option<String>,
}

impl PocMessage {
    /// Creates a state request with the fixed release-like metadata.
    #[must_use]
    pub fn state_request(correlation_id: &str, instance_id: &str) -> Self {
        Self::base(correlation_id, instance_id, 0, PocMessageKind::StateRequest)
    }

    /// Creates a state response with one bounded observation.
    #[must_use]
    pub fn state_response(
        correlation_id: &str,
        instance_id: &str,
        generation: u64,
        observation: PocObservation,
    ) -> Self {
        Self {
            observation: Some(observation),
            ..Self::base(
                correlation_id,
                instance_id,
                generation,
                PocMessageKind::StateResponse,
            )
        }
    }

    /// Creates an action request with the expected state generation.
    #[must_use]
    pub fn action_request(
        correlation_id: &str,
        instance_id: &str,
        generation: u64,
        action: PocAction,
    ) -> Self {
        Self {
            action: Some(action),
            ..Self::base(
                correlation_id,
                instance_id,
                generation,
                PocMessageKind::ActionRequest,
            )
        }
    }

    /// Creates an action response containing the post-operation observation.
    #[must_use]
    pub fn action_response(
        correlation_id: &str,
        instance_id: &str,
        generation: u64,
        action: PocAction,
        status: PocStatus,
        observation: PocObservation,
        error_code: Option<String>,
    ) -> Self {
        Self {
            observation: Some(observation),
            action: Some(action),
            status: Some(status),
            error_code,
            ..Self::base(
                correlation_id,
                instance_id,
                generation,
                PocMessageKind::ActionResponse,
            )
        }
    }

    pub(super) fn validate_request(&self, route: PocRoute) -> Result<(), PocValidationError> {
        if self.protocol_version != POC_PROTOCOL_VERSION || self.schema_digest != POC_SCHEMA_DIGEST
        {
            return Err(PocValidationError::Metadata);
        }
        self.provenance.validate()?;
        validate_identity(&self.correlation_id)?;
        validate_identity(&self.instance_id)?;
        if self.generation > u64::MAX / 2 {
            return Err(PocValidationError::GenerationBounds);
        }
        if self.observation.is_some() || self.status.is_some() || self.error_code.is_some() {
            return Err(PocValidationError::RequestShape);
        }
        match (route, self.kind, self.action.as_ref()) {
            (PocRoute::State, PocMessageKind::StateRequest, None) => Ok(()),
            (PocRoute::Action, PocMessageKind::ActionRequest, Some(action)) => action.validate(),
            (PocRoute::State, _, _) | (PocRoute::Action, _, _) => {
                Err(PocValidationError::RequestShape)
            }
        }
    }

    fn base(
        correlation_id: &str,
        instance_id: &str,
        generation: u64,
        kind: PocMessageKind,
    ) -> Self {
        Self {
            protocol_version: POC_PROTOCOL_VERSION.to_owned(),
            schema_digest: POC_SCHEMA_DIGEST.to_owned(),
            provenance: PocProvenance::default(),
            correlation_id: correlation_id.to_owned(),
            instance_id: instance_id.to_owned(),
            generation,
            kind,
            observation: None,
            action: None,
            status: None,
            error_code: None,
        }
    }
}

/// A small state projection owned by the core port.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PocCoreState {
    pub generation: u64,
    pub available_units: u16,
    pub settled_effects: u16,
}

/// Core legality failures preserved as stable POC error identities.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PocCoreError {
    StaleGeneration,
    ZeroUnits,
    InsufficientUnits,
}

impl PocCoreError {
    pub(super) const fn code(self) -> &'static str {
        match self {
            Self::StaleGeneration => "sts2.game-core/stale_generation",
            Self::ZeroUnits => "sts2.game-core/zero_units",
            Self::InsufficientUnits => "sts2.game-core/insufficient_units",
        }
    }
}

/// The only core capability required by the game-mod POC mapping.
pub trait PocCorePort {
    /// Reads the bounded state projection.
    fn snapshot(&self) -> PocCoreState;

    /// Applies one typed action against an expected generation.
    fn apply(
        &mut self,
        expected_generation: u64,
        action: &PocAction,
    ) -> Result<PocCoreState, PocCoreError>;
}

/// Deterministic failures at the game-mod POC seam.
#[derive(Debug, Eq, PartialEq)]
pub enum PocModError {
    ArtifactLoad(ArtifactError),
    ArtifactMismatch,
    MalformedRequest,
    Encoding,
}

impl std::fmt::Display for PocModError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::ArtifactLoad(_) => "the copied POC artifact could not be loaded",
            Self::ArtifactMismatch => "the request does not use the copied POC artifact",
            Self::MalformedRequest => "the POC request is malformed",
            Self::Encoding => "the POC message could not be encoded",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for PocModError {}

/// A deterministic validation failure for a POC request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PocValidationError {
    Metadata,
    Provenance,
    InvalidIdentity,
    GenerationBounds,
    ObservationBounds,
    ActionBounds,
    RequestShape,
}

fn validate_identity(value: &str) -> Result<(), PocValidationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-/".contains(&byte))
    {
        return Err(PocValidationError::InvalidIdentity);
    }
    Ok(())
}
