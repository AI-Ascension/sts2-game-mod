// SPDX-License-Identifier: MIT

use super::{
    ExternalInputEvidence, RNG_AUDIT_MAX_EXTERNAL_INPUTS, RNG_AUDIT_MAX_STREAMS,
    RNG_AUDIT_MAX_WITNESS_BYTES, RngAuditBinding, RngAuditError, RngCoverageStatus,
    RngStreamEvidence, fingerprint, validation,
};

/// A validated, private initialization witness.
///
/// The raw cursor and state evidence stay inside this value and are intentionally
/// absent from [`super::projection::RngAuditProjection`].
#[derive(Clone, Eq, PartialEq)]
pub struct RngAuditWitness {
    pub(super) binding: RngAuditBinding,
    pub(super) coverage: RngCoverageStatus,
    pub(super) streams: Vec<RngStreamEvidence>,
    pub(super) external_inputs: Vec<ExternalInputEvidence>,
    pub(super) private_size_bytes: usize,
}

impl RngAuditWitness {
    /// Builds and validates a private witness from owned source evidence.
    pub fn new(
        binding: RngAuditBinding,
        coverage: RngCoverageStatus,
        mut streams: Vec<RngStreamEvidence>,
        mut external_inputs: Vec<ExternalInputEvidence>,
    ) -> Result<Self, RngAuditError> {
        validation::validate_binding(&binding)?;
        if coverage != RngCoverageStatus::Complete {
            return Err(RngAuditError::IncompleteCoverage { status: coverage });
        }
        if streams.is_empty() {
            return Err(RngAuditError::NoStreams);
        }
        if streams.len() > RNG_AUDIT_MAX_STREAMS {
            return Err(RngAuditError::TooManyStreams {
                limit: RNG_AUDIT_MAX_STREAMS,
            });
        }
        if external_inputs.len() > RNG_AUDIT_MAX_EXTERNAL_INPUTS {
            return Err(RngAuditError::TooManyExternalInputs {
                limit: RNG_AUDIT_MAX_EXTERNAL_INPUTS,
            });
        }

        validation::validate_streams(&mut streams)?;
        validation::validate_external_inputs(&binding, &mut external_inputs)?;
        validation::validate_independent_seed_links(&streams, &external_inputs)?;

        let private_size_bytes =
            fingerprint::canonical_private_bytes(&binding, coverage, &streams, &external_inputs)
                .len();
        if private_size_bytes > RNG_AUDIT_MAX_WITNESS_BYTES {
            return Err(RngAuditError::WitnessTooLarge {
                limit: RNG_AUDIT_MAX_WITNESS_BYTES,
                actual: private_size_bytes,
            });
        }
        Ok(Self {
            binding,
            coverage,
            streams,
            external_inputs,
            private_size_bytes,
        })
    }

    /// Returns the deterministic private witness fingerprint.
    ///
    /// The digest is an audit identity, not a public gameplay observation. The
    /// raw state bytes and cursors are never returned.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        fingerprint::fingerprint(
            &self.binding,
            self.coverage,
            &self.streams,
            &self.external_inputs,
        )
    }

    /// Returns a projection-safe capability and provenance view.
    #[must_use]
    pub fn projection(&self) -> super::projection::RngAuditProjection {
        super::projection::RngAuditProjection::from_witness(self)
    }

    /// Returns the validated binding metadata.
    #[must_use]
    pub fn binding(&self) -> &RngAuditBinding {
        &self.binding
    }

    /// Returns the validated coverage status.
    #[must_use]
    pub const fn coverage(&self) -> RngCoverageStatus {
        self.coverage
    }

    /// Returns the number of audited streams.
    #[must_use]
    pub fn stream_count(&self) -> usize {
        self.streams.len()
    }

    /// Returns the private witness encoded-size estimate.
    #[must_use]
    pub const fn private_size_bytes(&self) -> usize {
        self.private_size_bytes
    }
}
