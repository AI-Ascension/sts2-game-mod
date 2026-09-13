// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    ExternalInputControl, ExternalInputDeclaration, ExternalInputEvidence, GameplayImpact,
    RNG_AUDIT_MAX_CALL_CATEGORIES, RNG_AUDIT_MAX_TEXT_BYTES, RngAuditBinding, RngAuditError,
    RngCursorEvidence, RngSeedOrigin, RngStreamEvidence,
};

pub(super) fn validate_binding(binding: &RngAuditBinding) -> Result<(), RngAuditError> {
    if binding.game_build.is_empty() {
        return Err(RngAuditError::MissingBuildBinding);
    }
    if binding.seed_derivation_version.is_empty() {
        return Err(RngAuditError::MissingSeedDerivation);
    }
    required_text("game_build", &binding.game_build)?;
    required_text("adapter_compatibility", &binding.adapter_compatibility)?;
    required_text("supported_mode", &binding.supported_mode)?;
    required_text("profile_compatibility", &binding.profile_compatibility)?;
    required_text("canonical_seed", &binding.canonical_seed)?;
    required_text("seed_derivation_version", &binding.seed_derivation_version)?;
    required_text("seeded_boundary", &binding.seeded_boundary)?;
    if let Some(content_manifest) = &binding.content_manifest {
        required_text("content_manifest", content_manifest)?;
    }
    Ok(())
}

pub(super) fn validate_streams(streams: &mut [RngStreamEvidence]) -> Result<(), RngAuditError> {
    let mut identities = BTreeSet::new();
    for stream in streams.iter_mut() {
        required_text("stream_id", &stream.stream_id)?;
        required_text("stream_owner", &stream.owner)?;
        required_text("creation_boundary", &stream.creation_boundary)?;
        required_text("reset_boundary", &stream.reset_boundary)?;
        required_text("stream_evidence", &stream.evidence)?;
        if !identities.insert(stream.stream_id.as_str()) {
            return Err(RngAuditError::DuplicateStreamIdentity {
                stream_id: stream.stream_id.clone(),
            });
        }
        if let Some(algorithm_version) = &stream.algorithm_version {
            required_text("algorithm_version", algorithm_version)?;
        }
        if stream.call_categories.is_empty() {
            return Err(RngAuditError::Empty {
                field: "call_categories",
            });
        }
        if stream.call_categories.len() > RNG_AUDIT_MAX_CALL_CATEGORIES {
            return Err(RngAuditError::TooManyCallCategories {
                stream_id: stream.stream_id.clone(),
                limit: RNG_AUDIT_MAX_CALL_CATEGORIES,
            });
        }
        let mut calls = BTreeSet::new();
        for call in &stream.call_categories {
            required_text("call_category", call)?;
            if !calls.insert(call.as_str()) {
                return Err(RngAuditError::MalformedState {
                    stream_id: stream.stream_id.clone(),
                });
            }
        }
        match &stream.seed_origin {
            RngSeedOrigin::MasterDerived { derivation_version } => {
                required_text("stream_derivation_version", derivation_version)?;
            }
            RngSeedOrigin::Independent { source } => {
                required_text("independent_seed_source", source)?;
            }
            RngSeedOrigin::Unavailable if stream.gameplay == GameplayImpact::AffectsGameplay => {
                return Err(RngAuditError::MissingSeedOrigin {
                    stream_id: stream.stream_id.clone(),
                });
            }
            RngSeedOrigin::Unavailable => {}
        }
        match &stream.initial_state {
            RngCursorEvidence::Known { state_digest, .. } => {
                validate_digest(state_digest).map_err(|()| RngAuditError::MalformedState {
                    stream_id: stream.stream_id.clone(),
                })?;
            }
            RngCursorEvidence::KnownZero { .. } => {}
            RngCursorEvidence::Unavailable
                if stream.gameplay == GameplayImpact::AffectsGameplay =>
            {
                return Err(RngAuditError::MissingStateEvidence {
                    stream_id: stream.stream_id.clone(),
                });
            }
            RngCursorEvidence::Unavailable => {}
        }
        if stream.gameplay == GameplayImpact::Unknown {
            return Err(RngAuditError::UnknownGameplayImpact {
                source: stream.stream_id.clone(),
            });
        }
        if stream.gameplay == GameplayImpact::CosmeticOnly && stream.evidence.is_empty() {
            return Err(RngAuditError::MissingCosmeticEvidence {
                source: stream.stream_id.clone(),
            });
        }
        if stream.gameplay == GameplayImpact::AffectsGameplay
            && stream.serialization == super::RngSerialization::Unknown
        {
            return Err(RngAuditError::MalformedState {
                stream_id: stream.stream_id.clone(),
            });
        }
        stream.call_categories.sort();
    }
    streams.sort_by(|left, right| left.stream_id.cmp(&right.stream_id));
    Ok(())
}

pub(super) fn validate_external_inputs(
    binding: &RngAuditBinding,
    inputs: &mut [ExternalInputEvidence],
) -> Result<(), RngAuditError> {
    match binding.external_input_declaration {
        ExternalInputDeclaration::Unknown => {
            return Err(RngAuditError::InvalidExternalDeclaration);
        }
        ExternalInputDeclaration::NoneObserved if !inputs.is_empty() => {
            return Err(RngAuditError::InvalidExternalDeclaration);
        }
        ExternalInputDeclaration::Enumerated if inputs.is_empty() => {
            return Err(RngAuditError::InvalidExternalDeclaration);
        }
        ExternalInputDeclaration::NoneObserved | ExternalInputDeclaration::Enumerated => {}
    }

    let mut kinds = BTreeSet::new();
    for input in inputs.iter() {
        required_text("external_input_owner", &input.owner)?;
        required_text("external_input_evidence", &input.evidence)?;
        if !kinds.insert(input.kind) {
            return Err(RngAuditError::DuplicateExternalInput { kind: input.kind });
        }
        if input.gameplay == GameplayImpact::Unknown {
            return Err(RngAuditError::UnknownGameplayImpact {
                source: input.kind.code().to_owned(),
            });
        }
        if input.gameplay == GameplayImpact::CosmeticOnly && input.evidence.is_empty() {
            return Err(RngAuditError::MissingCosmeticEvidence {
                source: input.kind.code().to_owned(),
            });
        }
        if input.gameplay == GameplayImpact::AffectsGameplay
            && matches!(
                input.control,
                ExternalInputControl::Uncontrolled | ExternalInputControl::Unknown
            )
        {
            return Err(RngAuditError::UncontrolledExternalInput { kind: input.kind });
        }
    }
    inputs.sort_by_key(|input| input.kind);
    Ok(())
}

pub(super) fn validate_independent_seed_links(
    streams: &[RngStreamEvidence],
    inputs: &[ExternalInputEvidence],
) -> Result<(), RngAuditError> {
    for stream in streams {
        let RngSeedOrigin::Independent { source } = &stream.seed_origin else {
            continue;
        };
        if stream.gameplay != GameplayImpact::AffectsGameplay {
            continue;
        }
        let linked = inputs
            .iter()
            .find(|input| input.kind.code() == source.as_str());
        if linked.is_none_or(|input| {
            input.gameplay != GameplayImpact::AffectsGameplay
                || input.control != ExternalInputControl::Controlled
        }) {
            return Err(RngAuditError::IndependentSeedNotLinked {
                stream_id: stream.stream_id.clone(),
            });
        }
    }
    Ok(())
}

fn required_text(field: &'static str, value: &str) -> Result<(), RngAuditError> {
    if value.is_empty() {
        return Err(RngAuditError::Empty { field });
    }
    if value.len() > RNG_AUDIT_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(RngAuditError::TooLong { field });
    }
    Ok(())
}

fn validate_digest(value: &str) -> Result<(), ()> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err(());
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(());
    }
    Ok(())
}
