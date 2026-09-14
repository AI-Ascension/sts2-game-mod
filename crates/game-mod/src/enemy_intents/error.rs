// SPDX-License-Identifier: MIT

/// Why a structured enemy-intent source cannot currently provide a snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemyIntentUnavailableReason {
    /// No authorized source is attached.
    NoActiveSource,
    /// Exact-host field evidence is still required.
    ExactHostEvidenceRequired,
    /// The selected host/build has no supported extractor.
    UnsupportedBuild,
    /// The caller's scope cannot read the source.
    ScopeDenied,
}

impl EnemyIntentUnavailableReason {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoActiveSource => "no_active_source",
            Self::ExactHostEvidenceRequired => "exact_host_evidence_required",
            Self::UnsupportedBuild => "unsupported_build",
            Self::ScopeDenied => "scope_denied",
        }
    }
}

/// Sanitized failures before a source-owned enemy snapshot exists.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnemyIntentSourceError {
    /// No active source exists.
    NoActiveSource,
    /// The caller is not permitted to read the source.
    AccessDenied,
    /// A transient host resolution is in progress.
    Busy,
    /// The source changed while it was being copied.
    Stale,
    /// The source returned malformed data.
    Malformed,
}

impl std::fmt::Display for EnemyIntentSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EnemyIntentSourceError {}

/// Sanitized validation and read failures for the owner-local intent projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyIntentError {
    /// A binding or nested identity was malformed.
    InvalidBinding(&'static str),
    /// A source-owned value violated a local bound or invariant.
    InvalidInput(&'static str),
    /// A source returned a duplicate enemy instance identity.
    DuplicateEnemy(String),
    /// A source returned a duplicate status identity.
    DuplicateStatus(String),
    /// A source returned duplicate component identity within one intent.
    DuplicateComponent(String),
    /// A source returned duplicate target identity within one target set.
    DuplicateTarget(String),
    /// A source returned duplicate parameter identity within one component.
    DuplicateParameter(String),
    /// An intent ID was reused as one of its enclosing IDs.
    AmbiguousIdentity(&'static str),
    /// A live enemy references an invalid intent linkage.
    InvalidLinkage(&'static str),
    /// A static or live reference belongs to another manifest/locale/producer.
    CatalogMismatch,
    /// A replacement snapshot belongs to another game instance.
    GameInstanceMismatch,
    /// A replacement snapshot belongs to another run.
    RunMismatch,
    /// A replacement snapshot belongs to another combat.
    CombatMismatch,
    /// A source snapshot did not echo the expected binding.
    StaleSource,
    /// A requested reference belongs to an older snapshot identity.
    StaleReference,
    /// A replacement snapshot did not advance the epoch.
    NonMonotonicEpoch { current: u64, supplied: u64 },
    /// The source capability is deliberately unavailable.
    Unavailable(EnemyIntentUnavailableReason),
    /// The source reported a transient resolving read.
    Busy,
    /// The source reported a stale or changing read.
    SourceStale,
    /// A requested live enemy instance is absent.
    EnemyNotFound,
    /// A requested intent component is absent.
    ComponentNotFound,
    /// A field cannot be returned under the selected visibility scope.
    VisibilityDenied(&'static str),
    /// A live detail exceeded its local byte bound.
    DetailTooLarge { limit: usize, actual: usize },
}

impl std::fmt::Display for EnemyIntentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBinding(field) => {
                write!(formatter, "invalid enemy intent binding: {field}")
            }
            Self::InvalidInput(field) => write!(formatter, "invalid enemy intent input: {field}"),
            Self::DuplicateEnemy(id) => write!(formatter, "duplicate enemy instance: {id}"),
            Self::DuplicateStatus(id) => write!(formatter, "duplicate enemy status: {id}"),
            Self::DuplicateComponent(id) => write!(formatter, "duplicate intent component: {id}"),
            Self::DuplicateTarget(id) => write!(formatter, "duplicate intent target: {id}"),
            Self::DuplicateParameter(id) => write!(formatter, "duplicate intent parameter: {id}"),
            Self::AmbiguousIdentity(field) => {
                write!(formatter, "intent identity is ambiguous: {field}")
            }
            Self::InvalidLinkage(field) => {
                write!(formatter, "invalid intent linkage: {field}")
            }
            Self::CatalogMismatch => formatter.write_str("enemy intent catalog changed"),
            Self::GameInstanceMismatch => formatter.write_str("enemy intent game instance changed"),
            Self::RunMismatch => formatter.write_str("enemy intent run changed"),
            Self::CombatMismatch => formatter.write_str("enemy intent combat changed"),
            Self::StaleSource => formatter.write_str("enemy intent source is stale"),
            Self::StaleReference => formatter.write_str("enemy intent reference is stale"),
            Self::NonMonotonicEpoch { current, supplied } => {
                write!(
                    formatter,
                    "enemy intent epoch {supplied} is not newer than {current}"
                )
            }
            Self::Unavailable(reason) => {
                write!(
                    formatter,
                    "enemy intent source unavailable: {}",
                    reason.code()
                )
            }
            Self::Busy => formatter.write_str("enemy intent read is busy"),
            Self::SourceStale => formatter.write_str("enemy intent source changed"),
            Self::EnemyNotFound => formatter.write_str("enemy intent enemy not found"),
            Self::ComponentNotFound => formatter.write_str("enemy intent component not found"),
            Self::VisibilityDenied(field) => {
                write!(formatter, "enemy intent field is not visible: {field}")
            }
            Self::DetailTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "enemy intent detail is {actual} bytes; limit is {limit}"
                )
            }
        }
    }
}

impl std::error::Error for EnemyIntentError {}

pub(super) fn map_source_error(error: EnemyIntentSourceError) -> EnemyIntentError {
    match error {
        EnemyIntentSourceError::NoActiveSource => {
            EnemyIntentError::Unavailable(EnemyIntentUnavailableReason::NoActiveSource)
        }
        EnemyIntentSourceError::AccessDenied => {
            EnemyIntentError::Unavailable(EnemyIntentUnavailableReason::ScopeDenied)
        }
        EnemyIntentSourceError::Busy => EnemyIntentError::Busy,
        EnemyIntentSourceError::Stale => EnemyIntentError::SourceStale,
        EnemyIntentSourceError::Malformed => EnemyIntentError::InvalidInput("source"),
    }
}
