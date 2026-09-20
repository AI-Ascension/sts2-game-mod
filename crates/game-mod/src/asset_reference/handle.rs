// SPDX-License-Identifier: MIT

//! Opaque asset handles, owner-controlled rendition bounds and the bounded rendition result.

use super::identity::is_opaque_handle;
use super::model::{
    ASSET_MAX_DECODE_RATIO, ASSET_MAX_DECODED_BYTES, ASSET_MAX_RENDITION_BYTES,
    ASSET_MAX_RENDITION_DIMENSION, ASSET_MAX_RENDITION_DURATION_MS,
};
use super::value::validate_media_type;
use super::{AssetReferenceError, AssetRenditionAuthority};

/// Opaque asset handle.
///
/// A handle is an identity, never a locator: a value that names a filesystem path or a URL is
/// refused here, so no caller can turn permitted retrieval into arbitrary filesystem export.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetHandle {
    value: String,
}

impl AssetHandle {
    /// Validates and states one opaque handle.
    pub fn new(value: &str) -> Result<Self, AssetReferenceError> {
        if value.is_empty() || value.len() > super::ASSET_MAX_IDENTITY_BYTES {
            return Err(AssetReferenceError::InvalidInput("handle"));
        }
        if !is_opaque_handle(value) {
            return Err(AssetReferenceError::PathLikeHandle);
        }
        Ok(Self {
            value: value.to_owned(),
        })
    }

    /// Returns the opaque handle text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// One handle with the generation after which it no longer resolves.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetHandleLease {
    /// Opaque asset handle.
    pub handle: AssetHandle,
    /// Catalog generation at which this reference stops resolving.
    pub expires_at_generation: u64,
}

impl AssetHandleLease {
    /// Returns whether this reference has expired at one catalog generation.
    #[must_use]
    pub const fn is_expired_at(&self, generation: u64) -> bool {
        generation >= self.expires_at_generation
    }
}

/// Owner-controlled bounds for one permitted rendition.
///
/// Every limit is fixed by the owner and validated against a hard ceiling, so a caller cannot ask
/// for an unbounded byte, dimension or duration transfer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetRenditionLimits {
    max_bytes: u64,
    max_dimension: u32,
    max_duration_ms: u64,
    max_decoded_bytes: u64,
}

impl Default for AssetRenditionLimits {
    fn default() -> Self {
        Self {
            max_bytes: ASSET_MAX_RENDITION_BYTES,
            max_dimension: ASSET_MAX_RENDITION_DIMENSION,
            max_duration_ms: ASSET_MAX_RENDITION_DURATION_MS,
            max_decoded_bytes: ASSET_MAX_DECODED_BYTES,
        }
    }
}

impl AssetRenditionLimits {
    /// Validates one set of owner-controlled rendition bounds.
    pub fn new(
        max_bytes: u64,
        max_dimension: u32,
        max_duration_ms: u64,
        max_decoded_bytes: u64,
    ) -> Result<Self, AssetReferenceError> {
        for (name, valid) in [
            (
                "max_bytes",
                max_bytes > 0 && max_bytes <= ASSET_MAX_RENDITION_BYTES,
            ),
            (
                "max_dimension",
                max_dimension > 0 && max_dimension <= ASSET_MAX_RENDITION_DIMENSION,
            ),
            (
                "max_duration_ms",
                max_duration_ms > 0 && max_duration_ms <= ASSET_MAX_RENDITION_DURATION_MS,
            ),
            (
                "max_decoded_bytes",
                max_decoded_bytes > 0 && max_decoded_bytes <= ASSET_MAX_DECODED_BYTES,
            ),
        ] {
            if !valid {
                return Err(AssetReferenceError::InvalidRenditionLimit(name));
            }
        }
        Ok(Self {
            max_bytes,
            max_dimension,
            max_duration_ms,
            max_decoded_bytes,
        })
    }

    /// Returns the stored-byte bound.
    #[must_use]
    pub const fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    /// Returns the dimension bound.
    #[must_use]
    pub const fn max_dimension(&self) -> u32 {
        self.max_dimension
    }

    /// Returns the duration bound.
    #[must_use]
    pub const fn max_duration_ms(&self) -> u64 {
        self.max_duration_ms
    }

    /// Returns the decoded-byte bound.
    #[must_use]
    pub const fn max_decoded_bytes(&self) -> u64 {
        self.max_decoded_bytes
    }

    /// Returns the decode-ratio bound that also applies.
    #[must_use]
    pub const fn max_decode_ratio(&self) -> u64 {
        ASSET_MAX_DECODE_RATIO
    }
}

/// One bounded, explicitly typed rendition request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRenditionRequest {
    /// Opaque handle of the requested asset.
    pub handle: AssetHandle,
    /// Owner-controlled bounds for this transfer.
    pub limits: AssetRenditionLimits,
    /// Exact media type the caller expects, when it states one.
    ///
    /// The requested type is always re-checked, so a caller cannot talk an audio asset into being
    /// served as an image, and a markup type is refused even when the source offers it.
    pub requested_media_type: Option<String>,
}

impl AssetRenditionRequest {
    /// States that the requested media type, when present, is exactly this one.
    pub fn new(
        handle: AssetHandle,
        limits: AssetRenditionLimits,
        requested_media_type: Option<&str>,
    ) -> Result<Self, AssetReferenceError> {
        if let Some(media_type) = requested_media_type {
            validate_media_type(media_type)?;
        }
        Ok(Self {
            handle,
            limits,
            requested_media_type: requested_media_type.map(str::to_owned),
        })
    }
}

/// Bounded payload of one rendition.
///
/// The two classes are separate types of value, not one value with a flag, so bytes can never be
/// read out of a metadata-only answer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AssetRenditionPayload {
    /// Metadata only: no bytes travel on this surface.
    MetadataOnly,
    /// Permitted bytes with the exact media type they were served as.
    Binary {
        /// Exact media type of the served bytes.
        media_type: String,
        /// Served bytes, bounded by the request limits.
        bytes: Vec<u8>,
    },
}

impl AssetRenditionPayload {
    /// Returns whether this payload carries bytes.
    #[must_use]
    pub const fn carries_binary(&self) -> bool {
        matches!(self, Self::Binary { .. })
    }

    /// Returns the served media type, when bytes were served.
    #[must_use]
    pub fn media_type(&self) -> Option<&str> {
        match self {
            Self::MetadataOnly => None,
            Self::Binary { media_type, .. } => Some(media_type),
        }
    }

    /// Returns the served bytes, or `None` for a metadata-only payload.
    #[must_use]
    pub fn bytes(&self) -> Option<&[u8]> {
        match self {
            Self::MetadataOnly => None,
            Self::Binary { bytes, .. } => Some(bytes),
        }
    }
}

/// Availability class of one rendition, keeping "cannot serve" apart from "does not exist".
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AssetRenditionAvailability {
    /// The bytes were served within the owner-controlled bounds.
    Available,
    /// Metadata is stated and bytes are withheld at this scope.
    MetadataOnly,
    /// The installed-host policy permits no retrieval of these bytes.
    DeniedByPolicy,
    /// The asset exists but no supported adapter can serve its bytes here.
    RetrievalUnavailable,
    /// The installed build contains no such asset.
    AssetMissing,
}

/// One bounded rendition result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRendition {
    /// Opaque handle this rendition was requested for.
    pub handle: AssetHandle,
    /// Availability class of this rendition.
    pub availability: AssetRenditionAvailability,
    /// Bounded payload, metadata-only whenever no bytes were served.
    pub payload: AssetRenditionPayload,
    /// Capability this rendition withholds.
    pub authority: AssetRenditionAuthority,
}

impl AssetRendition {
    /// Returns whether the availability class and the payload agree.
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        matches!(self.availability, AssetRenditionAvailability::Available)
            == self.payload.carries_binary()
    }

    /// Returns whether this rendition carries bytes.
    #[must_use]
    pub const fn carries_binary(&self) -> bool {
        self.payload.carries_binary()
    }
}
