// SPDX-License-Identifier: MIT

use super::binding::{RetainedMapObservationState, RetainedMapTravelActionability};

/// Why retained map knowledge cannot currently be observed or disclosed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapUnavailableReason {
    /// No authorized source has supplied an observation.
    NoActiveSource,
    /// Exact-host visibility evidence is still required.
    ExactHostEvidenceRequired,
    /// The selected host/build has no supported extractor.
    UnsupportedBuild,
    /// The caller's visibility scope disallows the read.
    ScopeDenied,
    /// No permitted observation has happened yet.
    NeverObserved,
    /// Disclosure policy forbids the retained value.
    PolicyWithheld,
}

impl RetainedMapUnavailableReason {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoActiveSource => "no_active_source",
            Self::ExactHostEvidenceRequired => "exact_host_evidence_required",
            Self::UnsupportedBuild => "unsupported_build",
            Self::ScopeDenied => "scope_denied",
            Self::NeverObserved => "never_observed",
            Self::PolicyWithheld => "policy_withheld",
        }
    }
}

/// Sanitized failures before an owned retained map snapshot exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RetainedMapSourceError {
    /// No active source exists.
    NoActiveSource,
    /// The caller is not permitted to read the source.
    AccessDenied,
    /// A transient host resolution is in progress.
    Busy,
    /// The source changed while it was being copied.
    Stale,
    /// The map surface is not observable for a permitted observation.
    NotObservable,
    /// The source returned malformed data.
    Malformed,
}

impl std::fmt::Display for RetainedMapSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RetainedMapSourceError {}

/// Sanitized validation and read failures for retained map knowledge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RetainedMapError {
    /// A binding or nested identity was malformed.
    InvalidBinding(&'static str),
    /// A source-owned value violated a local bound or invariant.
    InvalidInput(&'static str),
    /// A source returned a duplicate node identity.
    DuplicateNode(String),
    /// A source returned a duplicate directed edge.
    DuplicateEdge {
        /// Edge source node.
        from: String,
        /// Edge destination node.
        to: String,
    },
    /// A source returned a duplicate travel action identity.
    DuplicateTravel(String),
    /// An edge or travel binding referenced an absent node.
    UnknownNode(String),
    /// A travel action identity collides with one of its endpoint identities.
    AmbiguousIdentity(&'static str),
    /// A static or live reference belongs to another catalog identity.
    CatalogMismatch,
    /// A replacement snapshot belongs to another game instance.
    GameInstanceMismatch,
    /// A replacement snapshot belongs to another run.
    RunMismatch,
    /// A replacement snapshot belongs to another mode.
    ModeMismatch,
    /// A replacement snapshot belongs to another act.
    ActMismatch,
    /// A replacement snapshot belongs to another map instance.
    MapInstanceMismatch,
    /// A source snapshot did not echo the expected live binding.
    StaleSource,
    /// A requested reference belongs to an older snapshot identity.
    StaleReference,
    /// A replacement snapshot did not advance the epoch.
    NonMonotonicEpoch {
        /// Current retained epoch.
        current: u64,
        /// Supplied replacement epoch.
        supplied: u64,
    },
    /// The map surface is closed or otherwise not observable; a read must not open it.
    MapNotObservable(RetainedMapObservationState),
    /// A retained travel reference is not actionable at the current freshness.
    TravelNotActionable(RetainedMapTravelActionability),
    /// A topology page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// A requested retained node is absent.
    NodeNotFound,
    /// A retained node is not visible under the selected scope.
    ScopeDenied(&'static str),
    /// A retained detail exceeded its local byte bound.
    DetailTooLarge {
        /// Maximum accepted bytes.
        limit: usize,
        /// Measured actual bytes.
        actual: usize,
    },
    /// The source is deliberately unavailable.
    Unavailable(RetainedMapUnavailableReason),
    /// The source reported a transient resolving read.
    Busy,
    /// The source reported a stale or changing read.
    SourceStale,
}

impl std::fmt::Display for RetainedMapError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBinding(field) => {
                write!(formatter, "invalid retained map binding: {field}")
            }
            Self::InvalidInput(field) => write!(formatter, "invalid retained map input: {field}"),
            Self::DuplicateNode(id) => write!(formatter, "duplicate retained map node: {id}"),
            Self::DuplicateEdge { from, to } => {
                write!(formatter, "duplicate retained map edge: {from}->{to}")
            }
            Self::DuplicateTravel(id) => {
                write!(formatter, "duplicate retained travel action: {id}")
            }
            Self::UnknownNode(id) => write!(formatter, "unknown retained map node: {id}"),
            Self::AmbiguousIdentity(field) => {
                write!(formatter, "retained map identity is ambiguous: {field}")
            }
            Self::CatalogMismatch => formatter.write_str("retained map catalog changed"),
            Self::GameInstanceMismatch => formatter.write_str("retained map game instance changed"),
            Self::RunMismatch => formatter.write_str("retained map run changed"),
            Self::ModeMismatch => formatter.write_str("retained map mode changed"),
            Self::ActMismatch => formatter.write_str("retained map act changed"),
            Self::MapInstanceMismatch => formatter.write_str("retained map instance changed"),
            Self::StaleSource => formatter.write_str("retained map source is stale"),
            Self::StaleReference => formatter.write_str("retained map reference is stale"),
            Self::NonMonotonicEpoch { current, supplied } => {
                write!(
                    formatter,
                    "retained map epoch {supplied} is not newer than {current}"
                )
            }
            Self::MapNotObservable(state) => {
                write!(formatter, "map surface is not observable: {}", state.code())
            }
            Self::TravelNotActionable(state) => {
                write!(
                    formatter,
                    "retained travel is not actionable: {}",
                    state.code()
                )
            }
            Self::InvalidPageSize => formatter.write_str("invalid retained map page size"),
            Self::InvalidContinuation => formatter.write_str("invalid retained map continuation"),
            Self::NodeNotFound => formatter.write_str("retained map node not found"),
            Self::ScopeDenied(field) => {
                write!(formatter, "retained map field is not visible: {field}")
            }
            Self::DetailTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "retained map detail is {actual} bytes; limit is {limit}"
                )
            }
            Self::Unavailable(reason) => {
                write!(formatter, "retained map unavailable: {}", reason.code())
            }
            Self::Busy => formatter.write_str("retained map read is busy"),
            Self::SourceStale => formatter.write_str("retained map source changed"),
        }
    }
}

impl std::error::Error for RetainedMapError {}

pub(super) fn map_source_error(error: RetainedMapSourceError) -> RetainedMapError {
    match error {
        RetainedMapSourceError::NoActiveSource => {
            RetainedMapError::Unavailable(RetainedMapUnavailableReason::NoActiveSource)
        }
        RetainedMapSourceError::AccessDenied => {
            RetainedMapError::Unavailable(RetainedMapUnavailableReason::ScopeDenied)
        }
        RetainedMapSourceError::Busy => RetainedMapError::Busy,
        RetainedMapSourceError::Stale => RetainedMapError::SourceStale,
        RetainedMapSourceError::NotObservable => {
            RetainedMapError::MapNotObservable(RetainedMapObservationState::Closed)
        }
        RetainedMapSourceError::Malformed => RetainedMapError::InvalidInput("source"),
    }
}
