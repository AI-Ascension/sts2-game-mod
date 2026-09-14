// SPDX-License-Identifier: MIT

use serde::Serialize;

use crate::context::{
    ARTIFACT, CanonicalContext, ClientError, GENERATOR, PROTOCOL_VERSION, SCHEMA_SOURCE,
    is_identity,
};

/// Provenance block required by every seeded-run message.
#[derive(Clone, Debug, Serialize)]
pub struct Provenance {
    /// Shared artifact identity.
    pub artifact: String,
    /// Schema source path.
    pub source: String,
    /// Generator identity.
    pub generator: String,
}

/// Selected context with its digest, as carried in a request.
#[derive(Clone, Debug, Serialize)]
pub struct SelectionContext {
    /// Canonical context fields.
    #[serde(flatten)]
    pub canonical: CanonicalContext,
    /// Canonical SHA-256 digest of the fields above.
    pub context_digest: String,
}

/// A complete `start_request` envelope for `POST /v2/seeded-run`.
#[derive(Clone, Debug, Serialize)]
pub struct StartRequest {
    /// Protocol version.
    pub protocol_version: String,
    /// Schema digest.
    pub schema_digest: String,
    /// Provenance block.
    pub provenance: Provenance,
    /// Caller correlation identity.
    pub correlation_id: String,
    /// Runtime instance identity.
    pub instance_id: String,
    /// Runtime session identity.
    pub session_id: String,
    /// Runtime lease identity.
    pub lease_id: String,
    /// Runtime lease epoch.
    pub lease_epoch: u64,
    /// Host generation.
    pub generation: u64,
    /// Message kind.
    pub kind: String,
    /// Operation identity.
    pub operation_id: String,
    /// Requested seed.
    pub requested_seed: String,
    /// Run mode.
    pub run_mode: String,
    /// Selected-context digest.
    pub context_digest: String,
    /// Selected context.
    pub selected_context: SelectionContext,
    /// Always null for a request.
    pub status: Option<()>,
    /// Always null for a request.
    pub canonical_seed: Option<String>,
    /// Always null for a request.
    pub observation: Option<()>,
    /// Always null for a request.
    pub effect_witness: Option<()>,
    /// Always null for a request.
    pub error_code: Option<String>,
}

/// Inputs for [`build_start_request`]; host-derived values are supplied by the caller.
#[derive(Clone, Debug)]
pub struct StartRequestParams {
    /// Schema digest.
    pub schema_digest: String,
    /// Caller correlation identity.
    pub correlation_id: String,
    /// Runtime instance identity.
    pub instance_id: String,
    /// Runtime session identity.
    pub session_id: String,
    /// Runtime lease identity.
    pub lease_id: String,
    /// Runtime lease epoch.
    pub lease_epoch: u64,
    /// Host generation.
    pub generation: u64,
    /// Operation identity.
    pub operation_id: String,
    /// Requested seed.
    pub requested_seed: String,
    /// Run mode (`seeded_training`, `seeded_replay`, `diagnostic`).
    pub run_mode: String,
    /// Canonical selected context.
    pub context: CanonicalContext,
}

/// Builds a validated `start_request`, computing the context digest locally.
pub fn build_start_request(params: &StartRequestParams) -> Result<StartRequest, ClientError> {
    params.context.validate()?;
    if !is_digest_value(&params.schema_digest) {
        return Err(ClientError::InvalidField("schema_digest"));
    }
    if !is_identity(&params.operation_id)
        || !is_identity(&params.correlation_id)
        || !is_identity(&params.instance_id)
        || !is_identity(&params.session_id)
        || !is_identity(&params.lease_id)
    {
        return Err(ClientError::InvalidField("identity"));
    }
    if !matches!(
        params.run_mode.as_str(),
        "seeded_training" | "seeded_replay" | "diagnostic"
    ) {
        return Err(ClientError::InvalidField("run_mode"));
    }
    if !is_seed(&params.requested_seed) {
        return Err(ClientError::InvalidField("requested_seed"));
    }
    let context_digest = params.context.digest()?;
    Ok(StartRequest {
        protocol_version: PROTOCOL_VERSION.to_owned(),
        schema_digest: params.schema_digest.clone(),
        provenance: Provenance {
            artifact: ARTIFACT.to_owned(),
            source: SCHEMA_SOURCE.to_owned(),
            generator: GENERATOR.to_owned(),
        },
        correlation_id: params.correlation_id.clone(),
        instance_id: params.instance_id.clone(),
        session_id: params.session_id.clone(),
        lease_id: params.lease_id.clone(),
        lease_epoch: params.lease_epoch,
        generation: params.generation,
        kind: "start_request".to_owned(),
        operation_id: params.operation_id.clone(),
        requested_seed: params.requested_seed.clone(),
        run_mode: params.run_mode.clone(),
        context_digest: context_digest.clone(),
        selected_context: SelectionContext {
            canonical: params.context.clone(),
            context_digest,
        },
        status: None,
        canonical_seed: None,
        observation: None,
        effect_witness: None,
        error_code: None,
    })
}

impl StartRequest {
    /// Serializes the request to compact JSON.
    pub fn to_json(&self) -> Result<String, ClientError> {
        serde_json::to_string(self).map_err(|_| ClientError::Encode)
    }
}

fn is_digest_value(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_seed(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && !value.chars().any(char::is_control)
}
