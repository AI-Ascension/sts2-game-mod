// SPDX-License-Identifier: MIT

/// Failure before an owned result snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunResultSourceError {
    /// No supported run-result registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for RunResultSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RunResultSourceError {}

/// Sanitized failures while producing or reading source-only run-result data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunResultError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another locale.
    LocaleMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// Source and declared result or summary counts disagree.
    FamilyCountMismatch,
    /// A result is reported while the source declares results unavailable.
    UnknownResult(String),
    /// Two results repeat one opaque identity.
    DuplicateResult(String),
    /// Two summaries repeat one opaque identity.
    DuplicateSummary(String),
    /// Two records repeat one opaque run identity.
    DuplicateRun(String),
    /// A summary names a detail result the catalog does not carry.
    MissingResultDetail(String),
    /// An identity contains a control byte, a path separator or a traversal segment.
    NonOpaqueIdentity(&'static str),
    /// A field's status and value disagree.
    InconsistentField(&'static str),
    /// A result claims finalization its persistence evidence does not support.
    UnconfirmedFinalization,
    /// A run with no terminal state is published as a finalized result.
    UnfinishedResultPublishedAsFinalized,
    /// A run with no terminal state carries a score.
    ScoreForUnfinishedRun,
    /// A terminal outcome carries no score in a mode that reports one.
    OutcomeWithoutScore,
    /// The score authority contradicts the record's origin.
    EvaluatorScoreAsHostScore,
    /// The score authority and the declared score mode contradict each other.
    ScoreModeConflict,
    /// Summed components disagree with the displayed total.
    ScoreTotalMismatch {
        /// Total as displayed by the host.
        displayed: i64,
        /// Total summed from the published components.
        summed: i64,
    },
    /// A component carries an authority other than the score's own.
    ComponentAuthorityMismatch(String),
    /// A componentized mode omits its components or its total.
    UnreconciledScore,
    /// Two score components repeat one identity.
    DuplicateScoreComponent(String),
    /// Two statistics repeat one identity.
    DuplicateStatistic(String),
    /// Two ending-deck or ending-inventory entries repeat one identity.
    DuplicateEndingEntry(String),
    /// An ending entry states a count below one.
    ZeroCountEntry(String),
    /// A present collection is empty, so it states nothing.
    EmptyPresentCollection(&'static str),
    /// A summary publishes a detail that is not finalized.
    PendingDetailPublished,
    /// One run identity is shared by two profiles.
    ProfileIsolationViolation(String),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A record exceeds the aggregate byte bound.
    ResultTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Measured aggregate size.
        actual: usize,
    },
    /// A referenced definition is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity kind.
        entity_kind: String,
        /// Namespaced manifest identity.
        namespaced_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// The source declares the result family unavailable, so no record can be read.
    UnavailableFamily,
    /// The requested summary carries no readable detail, so none is reconstructed.
    DetailUnavailable(String),
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The record is hidden by the selected visibility scope.
    ExcludedByScope,
    /// The caller is not observing the owning profile.
    ProfileNotObserved,
    /// No record has the requested identity.
    NotFound,
    /// A live current-result read is missing its fence.
    MissingLiveFence,
    /// A history read carried a live fence it does not need.
    UnexpectedLiveFence,
    /// A live fence is incomplete or names another instance or run.
    StaleLiveFence,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for RunResultError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for RunResultError {}

/// Capability a published result read does not grant.
///
/// Reading a completed run is not authority to act on the active run, select a profile or touch a
/// save, so every published read states the capability it withholds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RunResultReadAuthority {
    /// A result read never authorizes selection, save loading or a run start.
    NotGranted,
}

pub(super) fn map_source_error(error: RunResultSourceError) -> RunResultError {
    match error {
        RunResultSourceError::NoActiveSource => RunResultError::NoActiveSource,
        RunResultSourceError::AccessDenied => RunResultError::SourceAccessDenied,
        RunResultSourceError::Malformed => RunResultError::MalformedSource,
    }
}
