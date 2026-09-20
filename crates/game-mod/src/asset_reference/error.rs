// SPDX-License-Identifier: MIT

//! Sanitized failures and the capabilities an asset read withholds.

/// Failure before an owned asset snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetSourceError {
    /// No supported asset source is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The installed-host policy permits no metadata or rendition retrieval.
    RetrievalNotPermitted,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for AssetSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AssetSourceError {}

/// Supported read seam availability for one asset source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssetReadAvailability {
    /// The seam is implemented only by an in-memory synthetic fixture.
    SyntheticFixtureOnly,
    /// Real host asset reads are not available on this target.
    UnavailableHost,
}

/// Capability a published asset rendition does not grant.
///
/// Retrieving permitted bytes is not authority to install, extract, execute or modify anything:
/// a rendition is never interpreted as markup or script, and a read never writes game data, so
/// every published rendition states the capability it withholds.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetRenditionAuthority {
    /// A rendition read never authorizes install, extraction, execution or mutation.
    NotGranted,
}

/// Sanitized failures while producing or reading source-only asset data.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetReferenceError {
    /// The source failed before an owned snapshot was available.
    NoActiveSource,
    /// The source denied a read.
    SourceAccessDenied,
    /// The source denied metadata or rendition retrieval by policy.
    RetrievalNotPermitted,
    /// The source returned malformed data.
    MalformedSource,
    /// The snapshot names another content manifest.
    ManifestMismatch,
    /// The snapshot uses another locale.
    LocaleMismatch,
    /// The snapshot uses another producer identity.
    ProducerVersionMismatch,
    /// The snapshot names another content-set revision than the manifest.
    ContentRevisionMismatch,
    /// The snapshot names another catalog generation than the manifest.
    GenerationMismatch,
    /// The declared media-class coverage does not state one class exactly once.
    ClassCoverageIncomplete,
    /// Declared and observed per-class asset counts disagree.
    ClassCountMismatch,
    /// An asset is published in a media class the source does not project.
    UnavailableClass,
    /// The asset's own manifest family is not handled by the content manifest.
    UnhandledManifestFamily(String),
    /// A referenced definition is absent from the content manifest.
    UnknownManifestReference {
        /// Manifest entity kind.
        entity_kind: String,
        /// Namespaced manifest identity.
        namespaced_id: String,
    },
    /// A handle is a filesystem path or a URL rather than an opaque identity.
    PathLikeHandle,
    /// Two assets repeat one opaque handle.
    DuplicateAsset(String),
    /// A media-type string is malformed or unsupported.
    UnsupportedMediaType(String),
    /// A media type that a host could interpret as executable markup was offered.
    MarkupMediaType(String),
    /// The media kind and the media properties describe different media.
    MediaKindMismatch,
    /// A retrievable or retrieval-denied asset states no media properties.
    MissingMediaProperties(&'static str),
    /// An asset the build does not contain carries properties or bytes.
    MissingAssetCarriesData(&'static str),
    /// A metadata-only surface was offered binary bytes.
    BinaryInMetadataSurface(&'static str),
    /// A rendition request or its limits are invalid.
    InvalidRenditionLimit(&'static str),
    /// A declared rendition exceeds the owner-controlled stored-byte bound.
    OversizedRendition {
        /// Configured bound.
        limit: u64,
        /// Measured size.
        actual: u64,
    },
    /// A declared rendition exceeds the owner-controlled dimension bound.
    ExcessiveDimensions {
        /// Configured bound.
        limit: u32,
        /// Measured dimension.
        actual: u32,
    },
    /// A declared rendition exceeds the owner-controlled duration bound.
    ExcessiveDuration {
        /// Configured bound.
        limit: u64,
        /// Measured duration.
        actual: u64,
    },
    /// A declared rendition's decoded size exceeds its own bound or decode ratio.
    InflatedRendition {
        /// Configured decoded-byte bound.
        limit: u64,
        /// Measured decoded size.
        actual: u64,
    },
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
    /// A single page did not cover every matching asset, so a retained reader is required.
    PartialPageRequiresReader,
    /// The asset is hidden by the requested visibility scope.
    ExcludedByScope,
    /// No asset has the requested handle.
    NotFound,
    /// The requested handle is absent from this catalog's handle set.
    ForgedHandle(String),
    /// The handle's generation has passed, so the reference no longer resolves.
    ExpiredHandle,
    /// The asset exists but this boundary cannot serve its bytes.
    RetrievalUnavailable,
    /// The installed build does not contain the linked asset.
    AssetMissing,
    /// A reference was produced for another manifest, locale, content revision or generation.
    StaleReference,
}

impl std::fmt::Display for AssetReferenceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AssetReferenceError {}

pub(super) fn map_source_error(error: AssetSourceError) -> AssetReferenceError {
    match error {
        AssetSourceError::NoActiveSource => AssetReferenceError::NoActiveSource,
        AssetSourceError::AccessDenied => AssetReferenceError::SourceAccessDenied,
        AssetSourceError::RetrievalNotPermitted => AssetReferenceError::RetrievalNotPermitted,
        AssetSourceError::Malformed => AssetReferenceError::MalformedSource,
    }
}
