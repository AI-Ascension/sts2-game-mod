// SPDX-License-Identifier: MIT

use super::{EventSemanticReferenceKind, EventUnavailableReason};

/// Failure before an owned event snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventSourceError {
    /// No supported event registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for EventSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EventSourceError {}

/// Sanitized failures while producing or reading source-only event reference data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventCatalogError {
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
    /// The manifest does not inventory an event family.
    MissingFamily,
    /// The source advertises a family it cannot project.
    UnsupportedFamily,
    /// The source family is temporarily unavailable.
    UnavailableFamily,
    /// The family identity in the source snapshot is wrong.
    FamilyIdentityMismatch,
    /// Source and manifest family counts disagree.
    FamilyCountMismatch,
    /// A source definition is absent from the manifest.
    UnknownDefinition(String),
    /// A manifest definition is absent from the source snapshot.
    MissingDefinition(String),
    /// A source definition repeats an identity.
    DuplicateDefinition(String),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A definition exceeds the aggregate byte bound.
    DefinitionTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Estimated actual size.
        actual: usize,
    },
    /// A typed reference is absent from the manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// An option, cost, effect, page, or requirement references a page absent from its event.
    UnknownPageReference {
        /// Owning event definition identity.
        event_id: String,
        /// Missing page identity.
        page_id: String,
    },
    /// A reference names an option absent from its event.
    UnknownOptionReference {
        /// Owning event definition identity.
        event_id: String,
        /// Missing option identity.
        option_id: String,
    },
    /// A visible option or outcome reveals a page the source marked hidden.
    HiddenFutureLeak {
        /// Owning event definition identity.
        event_id: String,
        /// Hidden page identity that would have leaked.
        page_id: String,
    },
    /// A record more visible than its target would disclose a restricted page, option, or event.
    ///
    /// The restricted identity is deliberately omitted so the rejection itself cannot disclose it.
    HiddenReferenceLeak {
        /// Owning event definition identity.
        event_id: String,
        /// Reference family whose target is more restricted.
        reference_kind: EventSemanticReferenceKind,
    },
    /// A defined option is not offered by any narrative page.
    UncoveredOption {
        /// Owning event definition identity.
        event_id: String,
        /// Option absent from every page's offered set.
        option_id: String,
    },
    /// A defined option is offered by more than one narrative page.
    DuplicateOptionMembership {
        /// Owning event definition identity.
        event_id: String,
        /// Option offered by multiple pages.
        option_id: String,
    },
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// The definition is hidden by the selected visibility scope.
    ExcludedByScope,
    /// A requested field is explicitly unavailable for the stated reason.
    UnavailableField(EventUnavailableReason),
    /// No definition has the requested identity.
    NotFound,
    /// A reference was produced for another manifest/locale/producer.
    StaleReference,
}

impl std::fmt::Display for EventCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EventCatalogError {}

pub(super) fn map_source_error(error: EventSourceError) -> EventCatalogError {
    match error {
        EventSourceError::NoActiveSource => EventCatalogError::NoActiveSource,
        EventSourceError::AccessDenied => EventCatalogError::SourceAccessDenied,
        EventSourceError::Malformed => EventCatalogError::MalformedSource,
    }
}
