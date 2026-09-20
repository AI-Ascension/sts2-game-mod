// SPDX-License-Identifier: MIT

//! The closed field inventory of one asset entry.

/// Field of one asset entry.
///
/// Every entry states a row for each variant, so a field the source does not project is a declared
/// coverage failure rather than a silently missing key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetField {
    /// Opaque asset handle.
    Handle,
    /// Content definition the asset is linked to.
    Definition,
    /// Media kind the source declares.
    MediaKind,
    /// Whether the asset is retrievable, metadata-only, unavailable or missing.
    RetrievalState,
    /// Media type, dimensions and duration.
    MediaProperties,
    /// Stored and decoded byte sizes.
    ByteSize,
    /// Package and content-revision provenance.
    Origin,
    /// Availability of a permitted rendition.
    Rendition,
}

impl AssetField {
    /// Returns every field an entry must state.
    #[must_use]
    pub const fn all() -> [Self; 8] {
        [
            Self::Handle,
            Self::Definition,
            Self::MediaKind,
            Self::RetrievalState,
            Self::MediaProperties,
            Self::ByteSize,
            Self::Origin,
            Self::Rendition,
        ]
    }

    /// Returns the stable field name used in diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Handle => "handle",
            Self::Definition => "definition",
            Self::MediaKind => "media_kind",
            Self::RetrievalState => "retrieval_state",
            Self::MediaProperties => "media_properties",
            Self::ByteSize => "byte_size",
            Self::Origin => "origin",
            Self::Rendition => "rendition",
        }
    }
}

/// Availability of one asset field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetFieldStatus {
    /// A value was reported.
    Available,
    /// The field does not apply to this asset, such as dimensions on an audio asset.
    NotApplicable,
    /// The source did not report the field for this read.
    NotObserved,
    /// The supported build does not carry the field.
    Unsupported,
    /// The field exists but is withheld at this scope.
    Withheld,
}

impl AssetFieldStatus {
    /// Returns whether a value may be published for this status.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }

    /// Returns whether the status names a reason the field carries no value.
    #[must_use]
    pub const fn is_stated(self) -> bool {
        !self.is_available()
    }
}

/// Declared availability of one entry field.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetFieldAvailability {
    /// Field this row states.
    pub field: AssetField,
    /// Availability of that field.
    pub status: AssetFieldStatus,
}
