// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};

use super::{
    ExternalInputEvidence, RngAuditBinding, RngCoverageStatus, RngCursorEvidence, RngStreamEvidence,
};

pub(super) fn canonical_private_bytes(
    binding: &RngAuditBinding,
    coverage: RngCoverageStatus,
    streams: &[RngStreamEvidence],
    external_inputs: &[ExternalInputEvidence],
) -> Vec<u8> {
    let mut canonical = Vec::new();
    field(&mut canonical, "schema", super::RNG_AUDIT_SCHEMA);
    field(&mut canonical, "profile", super::RNG_AUDIT_PROFILE);
    field(&mut canonical, "binding.game_build", &binding.game_build);
    field(
        &mut canonical,
        "binding.adapter_compatibility",
        &binding.adapter_compatibility,
    );
    field(
        &mut canonical,
        "binding.supported_mode",
        &binding.supported_mode,
    );
    field(
        &mut canonical,
        "binding.profile_compatibility",
        &binding.profile_compatibility,
    );
    optional_field(
        &mut canonical,
        "binding.content_manifest",
        binding.content_manifest.as_deref(),
    );
    field(
        &mut canonical,
        "binding.canonical_seed",
        &binding.canonical_seed,
    );
    field(
        &mut canonical,
        "binding.seed_derivation_version",
        &binding.seed_derivation_version,
    );
    field(
        &mut canonical,
        "binding.seeded_boundary",
        &binding.seeded_boundary,
    );
    field(
        &mut canonical,
        "binding.external_input_declaration",
        binding.external_input_declaration.code(),
    );
    field(&mut canonical, "coverage", coverage.code());
    field(&mut canonical, "stream.count", &streams.len().to_string());
    for stream in streams {
        field(&mut canonical, "stream.id", &stream.stream_id);
        field(&mut canonical, "stream.category", stream.category.code());
        field(&mut canonical, "stream.owner", &stream.owner);
        optional_field(
            &mut canonical,
            "stream.algorithm_version",
            stream.algorithm_version.as_deref(),
        );
        field(
            &mut canonical,
            "stream.seed_origin",
            stream.seed_origin.kind(),
        );
        match &stream.seed_origin {
            super::RngSeedOrigin::MasterDerived { derivation_version } => {
                field(
                    &mut canonical,
                    "stream.seed_derivation_version",
                    derivation_version,
                );
            }
            super::RngSeedOrigin::Independent { source } => {
                field(&mut canonical, "stream.independent_source", source);
            }
            super::RngSeedOrigin::Unavailable => {}
        }
        state_fields(&mut canonical, &stream.initial_state);
        field(
            &mut canonical,
            "stream.creation_boundary",
            &stream.creation_boundary,
        );
        field(
            &mut canonical,
            "stream.reset_boundary",
            &stream.reset_boundary,
        );
        field(
            &mut canonical,
            "stream.call_count",
            &stream.call_categories.len().to_string(),
        );
        for call in &stream.call_categories {
            field(&mut canonical, "stream.call", call);
        }
        field(
            &mut canonical,
            "stream.serialization",
            stream.serialization.code(),
        );
        field(&mut canonical, "stream.gameplay", stream.gameplay.code());
        field(&mut canonical, "stream.evidence", &stream.evidence);
    }
    field(
        &mut canonical,
        "external.count",
        &external_inputs.len().to_string(),
    );
    for input in external_inputs {
        field(&mut canonical, "external.kind", input.kind.code());
        field(&mut canonical, "external.owner", &input.owner);
        field(&mut canonical, "external.gameplay", input.gameplay.code());
        field(&mut canonical, "external.control", input.control.code());
        field(&mut canonical, "external.evidence", &input.evidence);
    }
    canonical
}

pub(super) fn fingerprint(
    binding: &RngAuditBinding,
    coverage: RngCoverageStatus,
    streams: &[RngStreamEvidence],
    external_inputs: &[ExternalInputEvidence],
) -> String {
    let bytes = canonical_private_bytes(binding, coverage, streams, external_inputs);
    let mut hasher = Sha256::new();
    hasher.update(super::RNG_AUDIT_FINGERPRINT_DOMAIN);
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut result = String::from("asc-rng-audit:v1:sha256:");
    for byte in digest {
        result.push_str(&format!("{byte:02x}"));
    }
    result
}

fn state_fields(canonical: &mut Vec<u8>, state: &RngCursorEvidence) {
    match state {
        RngCursorEvidence::Known {
            state_digest,
            cursor,
        } => {
            field(canonical, "stream.state", "known");
            field(canonical, "stream.state_digest", state_digest);
            field(canonical, "stream.cursor", &cursor.to_string());
        }
        RngCursorEvidence::KnownZero { cursor } => {
            field(canonical, "stream.state", "known_zero");
            field(canonical, "stream.cursor", &cursor.to_string());
        }
        RngCursorEvidence::Unavailable => field(canonical, "stream.state", "unavailable"),
    }
}

fn field(canonical: &mut Vec<u8>, name: &str, value: &str) {
    canonical.extend_from_slice(name.as_bytes());
    canonical.push(b':');
    canonical.extend_from_slice(value.len().to_string().as_bytes());
    canonical.push(b':');
    canonical.extend_from_slice(value.as_bytes());
    canonical.push(b'|');
}

fn optional_field(canonical: &mut Vec<u8>, name: &str, value: Option<&str>) {
    match value {
        Some(value) => {
            field(canonical, name, "present");
            let value_name = format!("{name}.value");
            field(canonical, &value_name, value);
        }
        None => field(canonical, name, "unknown"),
    }
}
