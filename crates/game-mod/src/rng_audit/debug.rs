// SPDX-License-Identifier: MIT

use std::fmt;

use super::{
    ExternalInputEvidence, RNG_AUDIT_PROFILE, RNG_AUDIT_SCHEMA, RngAuditBinding, RngAuditWitness,
    RngCursorEvidence, RngSeedOrigin, RngStreamEvidence,
};

impl fmt::Debug for RngCursorEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kind = match self {
            Self::Known { .. } => "known",
            Self::KnownZero { .. } => "known_zero",
            Self::Unavailable => "unavailable",
        };
        formatter
            .debug_struct("RngCursorEvidence")
            .field("kind", &kind)
            .finish()
    }
}

impl fmt::Debug for RngSeedOrigin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RngSeedOrigin")
            .field("kind", &self.kind())
            .finish()
    }
}

impl fmt::Debug for RngStreamEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RngStreamEvidence")
            .field("category", &self.category)
            .field("initial_state", &self.initial_state)
            .field("seed_origin", &self.seed_origin)
            .field("call_category_count", &self.call_categories.len())
            .field("serialization", &self.serialization)
            .field("gameplay", &self.gameplay)
            .finish()
    }
}

impl fmt::Debug for ExternalInputEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ExternalInputEvidence")
            .field("kind", &self.kind)
            .field("gameplay", &self.gameplay)
            .field("control", &self.control)
            .finish()
    }
}

impl fmt::Debug for RngAuditBinding {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RngAuditBinding")
            .field("content_manifest_present", &self.content_manifest.is_some())
            .field(
                "external_input_declaration",
                &self.external_input_declaration,
            )
            .finish()
    }
}

impl fmt::Debug for RngAuditWitness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RngAuditWitness")
            .field("schema", &RNG_AUDIT_SCHEMA)
            .field("profile", &RNG_AUDIT_PROFILE)
            .field("coverage", &self.coverage)
            .field("stream_count", &self.streams.len())
            .field("external_input_count", &self.external_inputs.len())
            .field("private_size_bytes", &self.private_size_bytes)
            .finish()
    }
}
