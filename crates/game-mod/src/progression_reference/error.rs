// SPDX-License-Identifier: MIT

/// Failure before an owned progression snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressionSourceError {
    /// No supported progression source is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// No explicitly permitted profile was named, so nothing may be read.
    ProfileNotPermitted,
    /// The profile is offline or foreign and requires an explicitly supported read port.
    ForeignProfileRequiresReadPort,
    /// The profile revision moved while the read was in flight.
    RevisionChanged,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for ProgressionSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProgressionSourceError {}

/// Supported read seam availability for one progression source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressionReadAvailability {
    /// The seam is implemented only by an in-memory synthetic fixture.
    SyntheticFixtureOnly,
    /// Real host progression reads are not available on this target.
    UnavailableHost,
}

/// Capability a published progression read does not grant.
///
/// Reading progression is not authority to unlock content, purchase an item, write a save, or
/// select a profile, so every published read states the capability it withholds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProgressionReadAuthority {
    /// A progression read never authorizes unlock, purchase, save or selection.
    NotGranted,
}

/// Sanitized failures while producing or reading source-only progression data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProgressionReferenceError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// No explicit permission to read the named profile was established.
    ProfileNotPermitted,
    /// The profile is offline or foreign and this boundary has no supported read port for it.
    ForeignProfileRequiresReadPort,
    /// The query named the active profile under another identity, which would be a switch.
    ImplicitProfileSwitch,
    /// The profile revision the query named is not the revision of this snapshot.
    ProfileRevisionMismatch,
    /// The documented freshness witness names another user-data identity.
    ProfileBaselineMismatch,
    /// The snapshot names another content manifest.
    ManifestMismatch,
    /// The snapshot uses another locale.
    LocaleMismatch,
    /// The snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// Declared and observed per-domain entry counts disagree.
    DomainCountMismatch,
    /// The declared coverage does not state one domain exactly once.
    DomainCoverageIncomplete,
    /// An account-scoped domain was declared projected, which this boundary refuses.
    AccountScopedDomainClaimed,
    /// An entry in an account-scoped domain was published as game progression.
    AccountScopedEntryInProgression,
    /// The entry carries an account or private identifier beside game progression.
    AccountIdentifierInProgression,
    /// The domain's own manifest family is not handled by the content manifest.
    UnhandledDomainFamily,
    /// A referenced manifest family is absent or not handled.
    UnhandledManifestFamily(String),
    /// A referenced definition is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity kind.
        entity_kind: String,
        /// Namespaced manifest identity.
        namespaced_id: String,
    },
    /// The source declares the domain unavailable, so no entry in it can be read.
    UnavailableDomain,
    /// Two entries repeat one stable progression identity.
    DuplicateEntry(String),
    /// Two requirements on one entry repeat one identity.
    DuplicateRequirement(String),
    /// A locked entry states no requirement, so nothing explains the lock.
    LockedWithoutRequirement,
    /// An unlocked entry names a requirement the source reports as locked.
    UnlockedWithUnsatisfiedRequirement,
    /// An undiscovered entry was reported outside the compendium domain.
    UndiscoveredOutsideCompendium,
    /// A state that asserts nothing carries a progress or best value.
    StateCarriesValue(&'static str),
    /// A best value was reported outside the best-records domain.
    BestRecordOutsideBestRecords,
    /// A percentage is outside its own closed range and was not clamped.
    UnboundedPercentage,
    /// An entry omits a field row, so its coverage is not stated.
    MissingFieldCoverage(&'static str),
    /// An entry states one field row twice.
    DuplicateFieldCoverage(&'static str),
    /// A field row's status disagrees with the value the entry carries.
    InconsistentField(&'static str),
    /// A collection is stated available but is empty, so it states nothing.
    EmptyPresentCollection(&'static str),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// An entry exceeds the aggregate byte bound.
    EntryTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Measured aggregate size.
        actual: usize,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// A single page did not cover every matching entry, so a retained reader is required.
    PartialPageRequiresReader,
    /// The entry is hidden by the requested visibility scope.
    ExcludedByScope,
    /// No entry has the requested identity.
    NotFound,
    /// A reference was produced for another manifest, locale, profile or revision.
    StaleReference,
}

impl std::fmt::Display for ProgressionReferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProgressionReferenceError {}

pub(super) fn map_source_error(error: ProgressionSourceError) -> ProgressionReferenceError {
    match error {
        ProgressionSourceError::NoActiveSource => ProgressionReferenceError::NoActiveSource,
        ProgressionSourceError::AccessDenied => ProgressionReferenceError::SourceAccessDenied,
        ProgressionSourceError::ProfileNotPermitted => {
            ProgressionReferenceError::ProfileNotPermitted
        }
        ProgressionSourceError::ForeignProfileRequiresReadPort => {
            ProgressionReferenceError::ForeignProfileRequiresReadPort
        }
        ProgressionSourceError::RevisionChanged => {
            ProgressionReferenceError::ProfileRevisionMismatch
        }
        ProgressionSourceError::Malformed => ProgressionReferenceError::MalformedSource,
    }
}
