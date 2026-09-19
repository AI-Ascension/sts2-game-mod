// SPDX-License-Identifier: MIT

use super::ReferenceFamily;

/// Failure before an owned reference-text snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceTextSourceError {
    /// No supported reference-text source is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for ReferenceTextSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReferenceTextSourceError {}

/// Sanitized failures while producing or reading source-only reference text.
///
/// Every variant is a closed token.  A host exception, install path or raw owner message is never
/// carried here, and a rejection never echoes the restricted value it refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReferenceTextError {
    /// No supported reference-text source is active.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A document or section ID repeats inside the snapshot.
    DuplicateReference {
        /// Reference family of the conflicting document.
        family: ReferenceFamily,
        /// Conflicting document or section identity.
        namespaced_id: String,
    },
    /// A definition referenced by a document or screen is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// One document's retained text exceeds the aggregate byte bound.
    TextTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Estimated actual size.
        actual: usize,
    },
    /// A stored value carries executable or otherwise unsafe presentation.
    UnsafePresentation(&'static str),
    /// A stored value carries text shaped as an instruction to an agent consumer.
    ///
    /// The value is refused rather than repaired, because a reference read must never become
    /// agent authority.
    InstructionBearingText(&'static str),
    /// A control declares itself unavailable without stating why.
    MissingUnavailableReason {
        /// Offending control identity.
        control_id: String,
    },
    /// A continuation belongs to another family filter or query.
    QueryMismatch,
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// A reference was produced for another manifest or producer identity.
    StaleReference,
    /// No reference in the catalog matches the request.
    NotFound,
}

impl std::fmt::Display for ReferenceTextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReferenceTextError {}

pub(super) fn map_source_error(error: ReferenceTextSourceError) -> ReferenceTextError {
    match error {
        ReferenceTextSourceError::NoActiveSource => ReferenceTextError::NoActiveSource,
        ReferenceTextSourceError::AccessDenied => ReferenceTextError::SourceAccessDenied,
        ReferenceTextSourceError::Malformed => ReferenceTextError::MalformedSource,
    }
}
