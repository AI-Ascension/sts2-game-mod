// SPDX-License-Identifier: MIT

use super::{ExternalInputKind, RngCoverageStatus};

/// Fail-closed validation errors for a synthetic or host-provided witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RngAuditError {
    /// A binding/evidence text field was empty.
    Empty { field: &'static str },
    /// A binding/evidence text field exceeded the local bound.
    TooLong { field: &'static str },
    /// The build binding is absent.
    MissingBuildBinding,
    /// The seed derivation version is absent.
    MissingSeedDerivation,
    /// Coverage is not complete.
    IncompleteCoverage { status: RngCoverageStatus },
    /// A stream identity appeared more than once.
    DuplicateStreamIdentity { stream_id: String },
    /// An external input category appeared more than once.
    DuplicateExternalInput { kind: ExternalInputKind },
    /// No stream inventory was supplied.
    NoStreams,
    /// Too many streams were supplied.
    TooManyStreams { limit: usize },
    /// Too many external inputs were supplied.
    TooManyExternalInputs { limit: usize },
    /// Too many call categories were supplied for one stream.
    TooManyCallCategories { stream_id: String, limit: usize },
    /// State evidence was malformed.
    MalformedState { stream_id: String },
    /// A gameplay-affecting stream had no state/cursor evidence.
    MissingStateEvidence { stream_id: String },
    /// A gameplay-affecting stream had no seed origin.
    MissingSeedOrigin { stream_id: String },
    /// A gameplay-affecting source was not classified.
    UnknownGameplayImpact { source: String },
    /// A cosmetic-only source had no supporting evidence note.
    MissingCosmeticEvidence { source: String },
    /// A gameplay-affecting external input is not controlled.
    UncontrolledExternalInput { kind: ExternalInputKind },
    /// An independently seeded gameplay stream is not linked to a controlled input.
    IndependentSeedNotLinked { stream_id: String },
    /// The external-input declaration does not match the entries.
    InvalidExternalDeclaration,
    /// The private witness exceeds the local size bound.
    WitnessTooLarge { limit: usize, actual: usize },
}

impl std::fmt::Display for RngAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty { field } => write!(formatter, "{field} is empty"),
            Self::TooLong { field } => write!(formatter, "{field} is too long"),
            Self::MissingBuildBinding => formatter.write_str("game build binding is missing"),
            Self::MissingSeedDerivation => {
                formatter.write_str("seed derivation version is missing")
            }
            Self::IncompleteCoverage { status } => {
                write!(formatter, "RNG coverage is {}", status.code())
            }
            Self::DuplicateStreamIdentity { stream_id } => {
                write!(formatter, "duplicate stream identity: {stream_id}")
            }
            Self::DuplicateExternalInput { kind } => {
                write!(formatter, "duplicate external input: {}", kind.code())
            }
            Self::NoStreams => formatter.write_str("stream inventory is empty"),
            Self::TooManyStreams { limit } => write!(formatter, "too many streams (limit {limit})"),
            Self::TooManyExternalInputs { limit } => {
                write!(formatter, "too many external inputs (limit {limit})")
            }
            Self::TooManyCallCategories { stream_id, limit } => {
                write!(
                    formatter,
                    "too many call categories for {stream_id} (limit {limit})"
                )
            }
            Self::MalformedState { stream_id } => {
                write!(formatter, "malformed state evidence for {stream_id}")
            }
            Self::MissingStateEvidence { stream_id } => {
                write!(formatter, "missing state evidence for {stream_id}")
            }
            Self::MissingSeedOrigin { stream_id } => {
                write!(formatter, "missing seed origin for {stream_id}")
            }
            Self::UnknownGameplayImpact { source } => {
                write!(formatter, "unknown gameplay impact for {source}")
            }
            Self::MissingCosmeticEvidence { source } => {
                write!(formatter, "missing cosmetic-only evidence for {source}")
            }
            Self::UncontrolledExternalInput { kind } => {
                write!(formatter, "uncontrolled gameplay input: {}", kind.code())
            }
            Self::IndependentSeedNotLinked { stream_id } => {
                write!(
                    formatter,
                    "independent gameplay seed for {stream_id} is not linked to a controlled input"
                )
            }
            Self::InvalidExternalDeclaration => {
                formatter.write_str("external-input declaration does not match entries")
            }
            Self::WitnessTooLarge { limit, actual } => {
                write!(formatter, "witness is {actual} bytes, over limit {limit}")
            }
        }
    }
}

impl std::error::Error for RngAuditError {}

/// Reasons an initialization witness cannot currently be advertised.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RngAuditUnavailableReason {
    /// Exact-host evidence has not been collected.
    ExactHostEvidenceRequired,
    /// The host build is not bound to this witness contract.
    BuildBindingMissing,
    /// Stream inventory is incomplete or unknown.
    CoverageUnknown,
    /// A gameplay-affecting state/cursor is unavailable.
    StateUnavailable,
    /// A gameplay-affecting external input is uncontrolled.
    ExternalInputUncontrolled,
}

impl RngAuditUnavailableReason {
    /// Returns the stable machine-readable spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExactHostEvidenceRequired => "exact_host_evidence_required",
            Self::BuildBindingMissing => "build_binding_missing",
            Self::CoverageUnknown => "coverage_unknown",
            Self::StateUnavailable => "state_unavailable",
            Self::ExternalInputUncontrolled => "external_input_uncontrolled",
        }
    }
}

/// Errors from a host-thread audit port.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RngAuditReadError {
    /// The capability is explicitly unavailable.
    Unavailable(RngAuditUnavailableReason),
    /// A producer returned malformed evidence.
    Invalid(RngAuditError),
}

impl std::fmt::Display for RngAuditReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(formatter, "audit unavailable: {}", reason.code()),
            Self::Invalid(error) => write!(formatter, "invalid audit evidence: {error}"),
        }
    }
}

impl std::error::Error for RngAuditReadError {}
