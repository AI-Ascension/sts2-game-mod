// SPDX-License-Identifier: MIT

//! Typed asset values whose availability is stated, so an unknown value is never a zero.

use super::model::{ASSET_MAX_MEDIA_TYPE_BYTES, validate_identity};
use super::{AssetFieldStatus, AssetReferenceError};

/// A value whose availability is explicit.
///
/// The status and the value are one field, not two, because a caller that can read the value
/// without reading its status will eventually read a missing value as a default.  The constructors
/// are the only way to build one, so `Available` always carries a value and every other status
/// carries none.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetFieldValue<T> {
    status: AssetFieldStatus,
    value: Option<T>,
}

impl<T> AssetFieldValue<T> {
    /// States that a value was reported.
    #[must_use]
    pub fn available(value: T) -> Self {
        Self {
            status: AssetFieldStatus::Available,
            value: Some(value),
        }
    }

    /// States that the field does not apply to this asset.
    #[must_use]
    pub const fn not_applicable() -> Self {
        Self {
            status: AssetFieldStatus::NotApplicable,
            value: None,
        }
    }

    /// States that the source did not report the field for this read.
    #[must_use]
    pub const fn not_observed() -> Self {
        Self {
            status: AssetFieldStatus::NotObserved,
            value: None,
        }
    }

    /// States that the supported build does not carry the field.
    #[must_use]
    pub const fn unsupported() -> Self {
        Self {
            status: AssetFieldStatus::Unsupported,
            value: None,
        }
    }

    /// States that the field is withheld at this scope.
    #[must_use]
    pub const fn withheld() -> Self {
        Self {
            status: AssetFieldStatus::Withheld,
            value: None,
        }
    }

    /// Returns the availability status.
    #[must_use]
    pub const fn status(&self) -> AssetFieldStatus {
        self.status
    }

    /// Returns whether a value was reported.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.status.is_available()
    }

    /// Returns the reported value, or `None` when the status states no value.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        self.value.as_ref()
    }

    /// Consumes the field and returns the reported value, or `None`.
    #[must_use]
    pub fn into_value(self) -> Option<T> {
        self.value
    }

    /// Returns whether the status and the value agree.
    #[must_use]
    pub const fn is_consistent(&self) -> bool {
        self.status.is_available() == self.value.is_some()
    }
}

/// Returns whether a media type names executable markup rather than inert media.
///
/// A rendition whose type is `text/*`, `application/xml`, or any `+xml` subtype may be interpreted
/// as markup by a host that renders it, so it is refused instead of being delivered.
#[must_use]
pub fn is_markup_media_type(media_type: &str) -> bool {
    let Some((kind, subtype)) = media_type.split_once('/') else {
        return true;
    };
    kind == "text" || subtype == "xml" || subtype.ends_with("+xml")
}

/// Validates one media-type string and refuses executable markup.
pub(super) fn validate_media_type(media_type: &str) -> Result<(), AssetReferenceError> {
    let malformed = media_type.is_empty()
        || media_type.len() > ASSET_MAX_MEDIA_TYPE_BYTES
        || media_type.chars().any(char::is_control)
        || !media_type.is_ascii()
        || media_type.bytes().any(|byte| {
            !byte.is_ascii_lowercase()
                && !byte.is_ascii_digit()
                && !matches!(byte, b'/' | b'.' | b'+' | b'-')
        })
        || media_type.matches('/').count() != 1;
    if malformed {
        return Err(AssetReferenceError::UnsupportedMediaType(
            media_type.to_owned(),
        ));
    }
    if is_markup_media_type(media_type) {
        return Err(AssetReferenceError::MarkupMediaType(media_type.to_owned()));
    }
    Ok(())
}

/// Media type, dimensions and duration of one asset.
///
/// Every unit is stated with the value, because a bare number whose unit is left to the caller
/// cannot be compared with the host's own value, and an absent dimension is always `NotApplicable`
/// or `NotObserved` rather than zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetMediaProperties {
    media_type: String,
    width: Option<u32>,
    height: Option<u32>,
    duration_ms: Option<u64>,
}

impl AssetMediaProperties {
    /// Validates and states one asset's media properties.
    pub fn new(
        media_type: &str,
        width: Option<u32>,
        height: Option<u32>,
        duration_ms: Option<u64>,
    ) -> Result<Self, AssetReferenceError> {
        validate_media_type(media_type)?;
        if width.is_some() != height.is_some() {
            return Err(AssetReferenceError::MissingMediaProperties("dimensions"));
        }
        if let Some(width) = width
            && width == 0
        {
            return Err(AssetReferenceError::MissingMediaProperties("width"));
        }
        if let Some(height) = height
            && height == 0
        {
            return Err(AssetReferenceError::MissingMediaProperties("height"));
        }
        if duration_ms == Some(0) {
            return Err(AssetReferenceError::MissingMediaProperties("duration_ms"));
        }
        Ok(Self {
            media_type: media_type.to_owned(),
            width,
            height,
            duration_ms,
        })
    }

    /// Returns the exact media type.
    #[must_use]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// Returns whether the type names an image media class.
    #[must_use]
    pub fn is_image(&self) -> bool {
        self.media_type.starts_with("image/")
    }

    /// Returns whether the type names an audio media class.
    #[must_use]
    pub fn is_audio(&self) -> bool {
        self.media_type.starts_with("audio/")
    }

    /// Returns the reported width, when the source states one.
    #[must_use]
    pub const fn width(&self) -> Option<u32> {
        self.width
    }

    /// Returns the reported height, when the source states one.
    #[must_use]
    pub const fn height(&self) -> Option<u32> {
        self.height
    }

    /// Returns the reported duration, when the source states one.
    #[must_use]
    pub const fn duration_ms(&self) -> Option<u64> {
        self.duration_ms
    }

    /// Returns the larger reported dimension, or `None` for audio media.
    #[must_use]
    pub fn max_dimension(&self) -> Option<u32> {
        match (self.width, self.height) {
            (Some(width), Some(height)) => Some(width.max(height)),
            _ => None,
        }
    }
}

/// Stored and decoded byte sizes of one asset.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetByteSize {
    stored_bytes: u64,
    decoded_bytes: u64,
}

impl AssetByteSize {
    /// Creates one byte size, refusing a decoded size below the stored size.
    pub fn new(stored_bytes: u64, decoded_bytes: u64) -> Result<Self, AssetReferenceError> {
        if stored_bytes == 0 {
            return Err(AssetReferenceError::InvalidInput("stored_bytes"));
        }
        if decoded_bytes < stored_bytes {
            return Err(AssetReferenceError::InvalidInput("decoded_bytes"));
        }
        Ok(Self {
            stored_bytes,
            decoded_bytes,
        })
    }

    /// Returns the stored byte count.
    #[must_use]
    pub const fn stored_bytes(&self) -> u64 {
        self.stored_bytes
    }

    /// Returns the decoded byte count.
    #[must_use]
    pub const fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes
    }

    /// Returns the decoded-to-stored ratio, rounded up, or zero when nothing is stored.
    #[must_use]
    pub const fn decode_ratio(&self) -> u64 {
        self.decoded_bytes.div_ceil(self.stored_bytes)
    }
}

/// Package and content-revision provenance of one asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetOrigin {
    /// Owning content package identity.
    pub package_id: String,
    /// Package version, when the source states one.
    pub package_version: Option<String>,
    /// Content-set revision the asset was read at.
    pub content_revision: String,
}

impl AssetOrigin {
    /// Validates and states one asset's provenance.
    pub fn new(
        package_id: &str,
        package_version: Option<&str>,
        content_revision: &str,
    ) -> Result<Self, AssetReferenceError> {
        validate_identity(package_id, "package_id")?;
        validate_identity(content_revision, "content_revision")?;
        if let Some(version) = package_version {
            validate_identity(version, "package_version")?;
        }
        Ok(Self {
            package_id: package_id.to_owned(),
            package_version: package_version.map(str::to_owned),
            content_revision: content_revision.to_owned(),
        })
    }
}

/// Metadata-only statement of whether one asset's bytes may be served.
///
/// This type deliberately carries no bytes, so the retained catalog and every listing surface stay
/// text-only and no rendition blob can leak into ordinary search or log output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetRenditionDescriptor {
    /// Whether the bytes may be served within the owner-controlled bounds.
    pub availability: super::AssetRenditionAvailability,
    /// Media type of the served bytes, when a rendition may be served.
    pub media_type: Option<String>,
    /// Decoded byte count of the served bytes, when a rendition may be served.
    pub decoded_bytes: Option<u64>,
}
