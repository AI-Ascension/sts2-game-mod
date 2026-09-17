// SPDX-License-Identifier: MIT

use super::super::canonical::CanonicalError;
use super::super::capability::{CheckpointBoundary, CheckpointCapabilities};
use super::super::error::CheckpointCaptureRejection;

/// Why a value is not a valid v1 checkpoint payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointPayloadError {
    /// The canonical text could not be strictly parsed.
    Canonical(CanonicalError),
    /// The boundary is in the owner matrix but has no v1 payload schema.
    UnsupportedBoundary {
        /// Requested boundary.
        boundary: CheckpointBoundary,
    },
    /// The boundary token is not in the owner matrix at all.
    UnknownBoundary,
    /// A pinned constant (`schema`, `canonical_profile`) does not match.
    SchemaMismatch {
        /// Field name.
        field: &'static str,
    },
    /// A required field is absent.
    MissingField {
        /// Field name.
        field: &'static str,
    },
    /// An object carries a field the closed schema does not declare.
    UnexpectedField {
        /// Enclosing object name.
        within: &'static str,
    },
    /// A field has the wrong canonical type.
    InvalidType {
        /// Field name.
        field: &'static str,
    },
    /// A text field violates the identifier grammar or length bound.
    InvalidIdentifier {
        /// Field name.
        field: &'static str,
    },
    /// An integer is outside its declared range.
    OutOfRange {
        /// Field name.
        field: &'static str,
    },
    /// A collection exceeds its declared bound or is below its minimum.
    BoundExceeded {
        /// Field name.
        field: &'static str,
    },
    /// A coverage token is not `captured`, `unknown`, or `not_applicable`.
    InvalidCoverage {
        /// Family name.
        family: &'static str,
    },
    /// A family required at this boundary reported `unknown` coverage.
    RequiredFamilyUnknown {
        /// Family name.
        family: &'static str,
    },
    /// A family required at this boundary was not captured.
    FamilyNotCaptured {
        /// Family name.
        family: &'static str,
    },
    /// A family that has no state at this boundary was captured anyway.
    FamilyNotApplicable {
        /// Family name.
        family: &'static str,
    },
    /// Two RNG streams share one identity.
    DuplicateStreamId,
    /// Two card instances share one identity.
    DuplicateCardInstance,
    /// A deck or pile reference does not resolve to a card instance.
    UnresolvedCardReference,
    /// The turn witness does not support the claimed combat boundary.
    TurnWitnessMismatch,
    /// Effects are still outstanding, so the boundary is not settled.
    PendingEffectsOutstanding,
}

impl CheckpointPayloadError {
    /// Returns the stable machine-readable error code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Canonical(_) => "canonical",
            Self::UnsupportedBoundary { .. } => "unsupported_boundary",
            Self::UnknownBoundary => "unknown_boundary",
            Self::SchemaMismatch { .. } => "schema_mismatch",
            Self::MissingField { .. } => "missing_field",
            Self::UnexpectedField { .. } => "unexpected_field",
            Self::InvalidType { .. } => "invalid_type",
            Self::InvalidIdentifier { .. } => "invalid_identifier",
            Self::OutOfRange { .. } => "out_of_range",
            Self::BoundExceeded { .. } => "bound_exceeded",
            Self::InvalidCoverage { .. } => "invalid_coverage",
            Self::RequiredFamilyUnknown { .. } => "required_family_unknown",
            Self::FamilyNotCaptured { .. } => "family_not_captured",
            Self::FamilyNotApplicable { .. } => "family_not_applicable",
            Self::DuplicateStreamId => "duplicate_stream_id",
            Self::DuplicateCardInstance => "duplicate_card_instance",
            Self::UnresolvedCardReference => "unresolved_card_reference",
            Self::TurnWitnessMismatch => "turn_witness_mismatch",
            Self::PendingEffectsOutstanding => "pending_effects_outstanding",
        }
    }

    /// Maps this payload error to the owner's typed capture rejection.
    ///
    /// A boundary without a payload schema returns the existing matrix
    /// rejection for that boundary; no new rejection kind is introduced.
    #[must_use]
    pub const fn rejection(self) -> CheckpointCaptureRejection {
        match self {
            Self::UnsupportedBoundary { boundary } => CheckpointCapabilities
                .for_boundary(boundary)
                .rejection(boundary),
            Self::UnknownBoundary | Self::PendingEffectsOutstanding => {
                CheckpointCaptureRejection::UnsafeBoundary
            }
            Self::InvalidCoverage { .. }
            | Self::RequiredFamilyUnknown { .. }
            | Self::FamilyNotCaptured { .. }
            | Self::FamilyNotApplicable { .. }
            | Self::TurnWitnessMismatch => CheckpointCaptureRejection::UnsupportedCoverage,
            Self::Canonical(_)
            | Self::SchemaMismatch { .. }
            | Self::MissingField { .. }
            | Self::UnexpectedField { .. }
            | Self::InvalidType { .. }
            | Self::InvalidIdentifier { .. }
            | Self::OutOfRange { .. }
            | Self::BoundExceeded { .. }
            | Self::DuplicateStreamId
            | Self::DuplicateCardInstance
            | Self::UnresolvedCardReference => CheckpointCaptureRejection::InvalidCanonicalBytes,
        }
    }
}

impl std::fmt::Display for CheckpointPayloadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Canonical(error) => write!(formatter, "canonical: {error}"),
            Self::UnsupportedBoundary { boundary } => {
                write!(formatter, "no payload schema for {}", boundary.code())
            }
            Self::SchemaMismatch { field }
            | Self::MissingField { field }
            | Self::InvalidType { field }
            | Self::InvalidIdentifier { field }
            | Self::OutOfRange { field }
            | Self::BoundExceeded { field } => write!(formatter, "{}: {field}", self.code()),
            Self::UnexpectedField { within } => {
                write!(formatter, "unexpected field within {within}")
            }
            Self::InvalidCoverage { family }
            | Self::RequiredFamilyUnknown { family }
            | Self::FamilyNotCaptured { family }
            | Self::FamilyNotApplicable { family } => {
                write!(formatter, "{}: {family}", self.code())
            }
            _ => formatter.write_str(self.code()),
        }
    }
}

impl std::error::Error for CheckpointPayloadError {}

impl From<CanonicalError> for CheckpointPayloadError {
    fn from(error: CanonicalError) -> Self {
        Self::Canonical(error)
    }
}
