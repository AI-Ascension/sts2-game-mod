// SPDX-License-Identifier: MIT

//! Producer identity, local bounds and the shared identity validator.

use super::AssetReferenceError;

/// Source-only producer identity; this is not a wire or native ABI version.
pub const ASSET_REFERENCE_PRODUCER_VERSION: &str = "game-asset-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const ASSET_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one media-type string.
pub const ASSET_MAX_MEDIA_TYPE_BYTES: usize = 128;
/// Maximum aggregate bytes retained for one asset entry, excluding rendition bytes.
pub const ASSET_MAX_ENTRY_BYTES: usize = 256 * 1024;
/// Maximum asset entries in one source snapshot.
pub const ASSET_MAX_ASSETS: usize = 4_096;
/// Maximum declared field rows on one asset entry.
pub const ASSET_MAX_FIELD_ROWS: usize = 16;
/// Maximum assets returned by one bounded page.
pub const ASSET_MAX_PAGE_ITEMS: usize = 64;
/// Hard ceiling for one permitted rendition's stored bytes.
pub const ASSET_MAX_RENDITION_BYTES: u64 = 4 * 1024 * 1024;
/// Hard ceiling for one permitted rendition's width or height.
pub const ASSET_MAX_RENDITION_DIMENSION: u32 = 4_096;
/// Hard ceiling for one permitted audio rendition's duration.
pub const ASSET_MAX_RENDITION_DURATION_MS: u64 = 300_000;
/// Hard ceiling for one permitted rendition's decoded bytes.
pub const ASSET_MAX_DECODED_BYTES: u64 = 16 * 1024 * 1024;
/// Hard ceiling for one permitted rendition's decoded-to-stored ratio.
pub const ASSET_MAX_DECODE_RATIO: u64 = 64;

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), AssetReferenceError> {
    if value.is_empty()
        || value.len() > ASSET_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(AssetReferenceError::InvalidInput(field));
    }
    Ok(())
}
