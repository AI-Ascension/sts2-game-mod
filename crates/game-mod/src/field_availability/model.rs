// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

/// Identity that binds an owned read to one content manifest and one coherent observation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalReadReference {
    /// Opaque game-content manifest identity.
    pub content_manifest: String,
    /// Opaque coherent snapshot identity.
    pub snapshot: String,
    /// Monotonic owner-local epoch for the snapshot.
    pub epoch: u64,
}

impl LocalReadReference {
    /// Creates an owned reference without assigning protocol meaning to its strings.
    #[must_use]
    pub fn new(
        content_manifest: impl Into<String>,
        snapshot: impl Into<String>,
        epoch: u64,
    ) -> Self {
        Self {
            content_manifest: content_manifest.into(),
            snapshot: snapshot.into(),
            epoch,
        }
    }
}

/// Source provenance attached to every local field result.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalFieldOrigin {
    /// Owner-defined source kind; no cross-repository spelling is implied.
    pub source_kind: String,
    /// Manifest and snapshot identity for this read.
    pub reference: LocalReadReference,
}

impl LocalFieldOrigin {
    /// Creates provenance for one local source kind.
    #[must_use]
    pub fn new(source_kind: impl Into<String>, reference: LocalReadReference) -> Self {
        Self {
            source_kind: source_kind.into(),
            reference,
        }
    }
}

/// Availability state before any shared transport mapping.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocalFieldStatus {
    /// A value was observed, including zero and empty values.
    Available,
    /// The field has no meaning for this entity or phase.
    NotApplicable,
    /// The field is supported but outside the current basic observable surface.
    NotObserved,
    /// No extractor is supported for this kind or build.
    Unsupported,
    /// The caller's scope does not permit this field.
    Denied,
    /// The supplied identity no longer names the current observation.
    Stale,
    /// Bounded extraction failed.
    Failed,
    /// The source could not classify a value without inventing one.
    Unknown,
}

/// Stable local reason code. The eventual protocol adapter may map these codes explicitly.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocalReasonCode {
    /// Field is not meaningful for the current phase.
    PhaseNotApplicable,
    /// Supported field was omitted from the basic surface.
    SurfaceNotObserved,
    /// Field or kind has no supported extractor.
    UnsupportedField,
    /// A requested field name was not in the allowlist.
    UnknownField,
    /// Caller scope does not permit the field.
    ScopeDenied,
    /// Snapshot, epoch, manifest, or entity identity is stale.
    StaleReference,
    /// Bounded reflection or extraction failure.
    ExtractionFailed,
    /// Source observed a value but could not classify it.
    ValueUnknown,
    /// Detail payload exceeds the local bound.
    DetailTooLarge,
    /// Collection has more entries than this page.
    CollectionPartial,
    /// Source cannot provide a total count.
    TotalUnknown,
    /// Field group is not in the allowlist.
    UnknownFieldGroup,
    /// Continuation token does not match the originating read.
    InvalidContinuation,
}

impl LocalReasonCode {
    /// Returns a stable machine-readable spelling for local fixtures and diagnostics.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PhaseNotApplicable => "phase_not_applicable",
            Self::SurfaceNotObserved => "surface_not_observed",
            Self::UnsupportedField => "unsupported_field",
            Self::UnknownField => "unknown_field",
            Self::ScopeDenied => "scope_denied",
            Self::StaleReference => "stale_reference",
            Self::ExtractionFailed => "extraction_failed",
            Self::ValueUnknown => "value_unknown",
            Self::DetailTooLarge => "detail_too_large",
            Self::CollectionPartial => "collection_partial",
            Self::TotalUnknown => "total_unknown",
            Self::UnknownFieldGroup => "unknown_field_group",
            Self::InvalidContinuation => "invalid_continuation",
        }
    }
}

/// A bounded value that can cross the owner-local extraction boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalFieldValue {
    /// Signed numeric value; zero remains an available value.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
    /// Bounded text value.
    Text(String),
    /// Bounded text collection; an empty list remains available and empty.
    TextList(Vec<String>),
}

/// One field result. Unavailable states never carry an invented value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalFieldResult {
    /// The value was observed under the attached provenance.
    Available {
        /// Original typed value.
        value: LocalFieldValue,
        /// Source and identity witness.
        origin: LocalFieldOrigin,
    },
    /// The value was not made available for the stated reason.
    Unavailable {
        /// Explicit non-available state.
        status: LocalFieldStatus,
        /// Stable local reason.
        reason: LocalReasonCode,
        /// Source and identity witness.
        origin: LocalFieldOrigin,
    },
}

impl LocalFieldResult {
    /// Returns the explicit state without interpreting an unavailable result as empty.
    #[must_use]
    pub const fn status(&self) -> LocalFieldStatus {
        match self {
            Self::Available { .. } => LocalFieldStatus::Available,
            Self::Unavailable { status, .. } => *status,
        }
    }

    /// Returns the provenance attached to the result.
    #[must_use]
    pub fn origin(&self) -> &LocalFieldOrigin {
        match self {
            Self::Available { origin, .. } | Self::Unavailable { origin, .. } => origin,
        }
    }

    /// Returns the value only when it was actually observed.
    #[must_use]
    pub fn value(&self) -> Option<&LocalFieldValue> {
        match self {
            Self::Available { value, .. } => Some(value),
            Self::Unavailable { .. } => None,
        }
    }

    /// Returns the local reason, if the result is unavailable.
    #[must_use]
    pub const fn reason(&self) -> Option<LocalReasonCode> {
        match self {
            Self::Available { .. } => None,
            Self::Unavailable { reason, .. } => Some(*reason),
        }
    }
}

/// Allowlisted follow-up link emitted by a basic entity result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDetailLink {
    /// Owner-defined entity kind.
    pub entity_kind: String,
    /// Opaque live entity identity.
    pub entity_id: String,
    /// Allowlisted field group.
    pub field_group: String,
    /// Identity that the detail request must repeat exactly.
    pub origin: LocalFieldOrigin,
}

/// Bounded entity projection returned by a basic collection read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEntitySummary {
    /// Owner-defined entity kind.
    pub entity_kind: String,
    /// Opaque live entity identity.
    pub entity_id: String,
    /// Requested field results in stable key order.
    pub fields: BTreeMap<String, LocalFieldResult>,
    /// Explicitly supported expensive detail groups.
    pub detail_links: Vec<LocalDetailLink>,
}

/// Collection traversal state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LocalCompleteness {
    /// The page reached the end of the collection.
    Complete,
    /// More entries exist and a continuation is supplied.
    Partial,
}

/// Opaque continuation handle. The store, not the caller, owns its meaning.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LocalContinuation {
    token: String,
    scope: Arc<LocalContinuationScope>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct LocalContinuationScope;

impl LocalContinuation {
    /// Creates a handle for a token returned by the owner-local store.
    #[must_use]
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            scope: Arc::new(LocalContinuationScope),
        }
    }

    pub(super) fn scoped(token: impl Into<String>, scope: Arc<LocalContinuationScope>) -> Self {
        Self {
            token: token.into(),
            scope,
        }
    }

    pub(super) fn scope_matches(&self, scope: &Arc<LocalContinuationScope>) -> bool {
        Arc::ptr_eq(&self.scope, scope)
    }

    /// Returns the opaque token without exposing cursor semantics.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// One bounded collection page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalCollectionPage {
    /// Owner-defined entity kind.
    pub entity_kind: String,
    /// Entries returned in deterministic identity order.
    pub entries: Vec<LocalEntitySummary>,
    /// Total count when the source knows it.
    pub total: Option<usize>,
    /// Complete or explicitly partial.
    pub completeness: LocalCompleteness,
    /// Present only for a partial page.
    pub continuation: Option<LocalContinuation>,
    /// Identity shared by every entry.
    pub origin: LocalFieldOrigin,
}

impl LocalCollectionPage {
    /// Number of entries in this page.
    #[must_use]
    pub const fn returned_count(&self) -> usize {
        self.entries.len()
    }

    /// Whether the source supplied a total count.
    #[must_use]
    pub const fn count_known(&self) -> bool {
        self.total.is_some()
    }
}
