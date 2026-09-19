// SPDX-License-Identifier: MIT

use super::LocalePluralCategory;

/// Failure before an owned locale snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocaleSourceError {
    /// No supported locale registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for LocaleSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LocaleSourceError {}

/// Sanitized failures while producing or reading source-only locale reference data.
///
/// Every variant is a closed token.  A host exception, install path or raw owner message is never
/// carried here, and a rejection never echoes the restricted value it refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocaleCatalogError {
    /// No supported locale registry is active.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source returned malformed data.
    MalformedSource,
    /// The source snapshot names another content manifest.
    ManifestMismatch,
    /// The source snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The catalog declares no supported locales.
    MissingSupportedLocales,
    /// The catalog's default locale is not one of its supported locales.
    MissingDefaultLocale,
    /// A requested locale is not in the catalog's supported set.
    UnsupportedLocale(String),
    /// A locale tag repeats inside the supported set.
    DuplicateLocale(String),
    /// An explicit fallback chain is missing, unordered, cyclic, or too deep.
    InvalidFallbackChain(&'static str),
    /// An input field is invalid or exceeds a local collection bound.
    InvalidInput(&'static str),
    /// A definition referenced by an entry or segment is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
    },
    /// Two entries claim the same entity, locale and plural key.
    DuplicateEntry {
        /// Owning entity family.
        entity_kind: String,
        /// Namespaced definition identity.
        namespaced_id: String,
        /// Conflicting locale.
        locale: String,
        /// Conflicting plural category.
        plural: LocalePluralCategory,
    },
    /// One entry's retained text exceeds the aggregate byte bound.
    TextTooLarge {
        /// Configured aggregate bound.
        limit: usize,
        /// Estimated actual size.
        actual: usize,
    },
    /// A rendered value carries executable or otherwise unsafe presentation.
    UnsafePresentation(&'static str),
    /// A supplied placeholder name is not declared by the stored text.
    UnknownPlaceholder(String),
    /// A continuation belongs to another locale or another catalog revision.
    LocaleMismatch,
    /// A list page size is zero or exceeds its local bound.
    InvalidPageSize,
    /// A continuation is stale, reused, or bound to another query.
    InvalidContinuation,
    /// A reference was produced for another manifest or producer identity.
    StaleReference,
    /// No locale in the fallback chain carries text for the requested reference.
    NotFound,
}

impl std::fmt::Display for LocaleCatalogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LocaleCatalogError {}

pub(super) fn map_source_error(error: LocaleSourceError) -> LocaleCatalogError {
    match error {
        LocaleSourceError::NoActiveSource => LocaleCatalogError::NoActiveSource,
        LocaleSourceError::AccessDenied => LocaleCatalogError::SourceAccessDenied,
        LocaleSourceError::Malformed => LocaleCatalogError::MalformedSource,
    }
}
