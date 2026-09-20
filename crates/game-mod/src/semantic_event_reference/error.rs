// SPDX-License-Identifier: MIT

//! Sanitized failures, the source boundary's own error, the read scope a query asks for, and the
//! capability every published history read withholds.

use super::model::SemanticEventScope;

/// Failure before an owned semantic-event snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticEventSourceError {
    /// No supported event boundary is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for SemanticEventSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SemanticEventSourceError {}

/// Sanitized failures while producing or reading source-only semantic gameplay history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticEventError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// Source and declared event or gap counts disagree.
    FamilyCountMismatch,
    /// Two records repeat one opaque event identity.
    DuplicateEvent(String),
    /// Two records occupy one sequence number.
    DuplicateSequence(u64),
    /// A sequence number does not increase inside one scope.
    NonMonotonicSequence(u64),
    /// A sequence number is zero, or below the point capture began.
    InvalidSequenceStart(u64),
    /// An identity contains a control byte, a path separator or a traversal segment.
    NonOpaqueIdentity(&'static str),
    /// A referenced definition is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity kind.
        entity_kind: String,
        /// Namespaced manifest identity.
        namespaced_id: String,
    },
    /// The source declares the history family unavailable, so no record can be read.
    UnavailableFamily,
    /// A present collection is empty, so it states nothing.
    EmptyPresentCollection(&'static str),
    /// An input field is invalid or exceeds a local bound.
    InvalidInput(&'static str),
    /// A history exceeds a local count or byte bound.
    HistoryTooLarge {
        /// Configured bound.
        limit: usize,
        /// Measured size.
        actual: usize,
    },
    /// An observed record states no kind.
    MissingKind(String),
    /// An observed record states no origin.
    MissingOrigin(String),
    /// A disclosed gap carries an observed field.
    UnexpectedKind(String),
    /// A record's kind and the detail it carries disagree.
    KindDetailMismatch(String),
    /// A kind requires a subject role the record does not carry.
    MissingSubject(&'static str),
    /// A kind that acts on no target names one, or one role is named twice.
    DuplicateSubjectRole(&'static str),
    /// A kind that acts on no target names a target anyway.
    UnexpectedSubjectRole(&'static str),
    /// A target role names a namespace an acted-on subject cannot have.
    WrongSubjectNamespace(&'static str),
    /// Two equal tokens are used as different namespaces, or an event token is reused as a subject.
    IdentityNamespaceCollision(&'static str),
    /// A causal parent is stated where the kind, the origin or the record does not admit one.
    CausalityNotAdmitted(String),
    /// A kind that admits a cause states neither a parent nor its explicit absence.
    MissingCausalParent(String),
    /// A parent's stated flag and its named parent disagree.
    CausalProvenanceMismatch(String),
    /// A stated parent is not an observed event of this history.
    StatedParentUnknown(String),
    /// A stated parent does not precede its child.
    StatedParentNotBefore(String),
    /// A captured event sits inside a declared gap, or a gap sits outside one.
    CoverageContradiction(u64),
    /// A span inside the captured range is incomplete without being declared.
    UndeclaredGap(u64),
    /// A declared interval is reversed, overlapping, unordered, or claims to be captured.
    InvalidCoverageInterval(u64),
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// A reference was produced for another manifest or producer.
    StaleReference,
    /// The requested scope is outside the scope this read observes.
    ExcludedByScope,
    /// A history fence is incomplete or names another run, branch or episode.
    StaleHistoryFence,
    /// A history fence names an observation epoch other than the one the catalog holds.
    EpochMismatch {
        /// Epoch the fence names.
        expected: u64,
        /// Epoch the catalog holds.
        actual: u64,
    },
    /// No record has the requested identity.
    NotFound,
}

impl std::fmt::Display for SemanticEventError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SemanticEventError {}

/// Capability a published history read does not grant.
///
/// Observing what happened is not authority to replay it, to rewind to it, to re-run it, or to
/// change gameplay, so every published read states the capability it withholds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticHistoryAuthority {
    /// A history read never authorizes a replay, a rollback, or a gameplay action.
    NotGranted,
}

/// How much of one history a query asks to observe.
///
/// The scope is stated rather than assumed, because a question about one episode is not a question
/// about the whole run: a query whose scope does not cover the history it names is refused instead
/// of being answered with a wider or narrower record than it asked for.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticHistoryScope {
    /// One exact run, branch, episode and epoch.
    History,
    /// Every episode and epoch of one branch.
    Branch,
    /// Every epoch of one episode.
    Episode,
}

impl SemanticHistoryScope {
    /// Every scope, in a stable order.
    pub const ALL: [Self; 3] = [Self::History, Self::Branch, Self::Episode];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::History => "history",
            Self::Branch => "branch",
            Self::Episode => "episode",
        }
    }

    /// Returns whether a read at this scope covers the history a query names.
    #[must_use]
    pub fn observes(self, asked: &SemanticEventScope, held: &SemanticEventScope) -> bool {
        match self {
            Self::History => asked == held,
            Self::Branch => asked.run_id == held.run_id && asked.branch_id == held.branch_id,
            Self::Episode => {
                asked.run_id == held.run_id
                    && asked.branch_id == held.branch_id
                    && asked.episode == held.episode
            }
        }
    }
}

pub(super) fn map_source_error(error: SemanticEventSourceError) -> SemanticEventError {
    match error {
        SemanticEventSourceError::NoActiveSource => SemanticEventError::NoActiveSource,
        SemanticEventSourceError::AccessDenied => SemanticEventError::SourceAccessDenied,
        SemanticEventSourceError::Malformed => SemanticEventError::MalformedSource,
    }
}
