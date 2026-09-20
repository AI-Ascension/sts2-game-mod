// SPDX-License-Identifier: MIT

//! Per-asset eligibility: provenance, media consistency, retrieval state and bounded renditions.

use crate::ContentManifest;

use super::super::definition::{entry_bytes, validate_field_rows};
use super::super::{
    ASSET_MAX_ENTRY_BYTES, AssetCatalogBinding, AssetFieldStatus, AssetFieldValue, AssetHandle,
    AssetReferenceError, AssetRenditionDescriptor, AssetRenditionLimits, AssetRenditionPayload,
    AssetRetrievalState, validate_identity,
};
use super::validate_definition_reference;

/// Validates one asset, then splits its opaque handle from its metadata-only descriptor.
pub(crate) fn validate_asset(
    input: &super::super::AssetEntryInput,
    manifest: &ContentManifest,
    binding: &AssetCatalogBinding,
    limits: &AssetRenditionLimits,
) -> Result<(AssetHandle, AssetFieldValue<AssetRenditionDescriptor>), AssetReferenceError> {
    let handle = AssetHandle::new(&input.handle)?;
    validate_identity(&input.origin.package_id, "package_id")?;
    if input.origin.content_revision != binding.content_revision {
        return Err(AssetReferenceError::ContentRevisionMismatch);
    }
    validate_definition_reference(manifest, &input.definition)?;
    validate_media(input)?;
    validate_retrieval_state(input)?;
    validate_rendition(input, limits)?;
    validate_field_rows(input)?;
    let actual = entry_bytes(input);
    if actual > ASSET_MAX_ENTRY_BYTES {
        return Err(AssetReferenceError::EntryTooLarge {
            limit: ASSET_MAX_ENTRY_BYTES,
            actual,
        });
    }
    Ok((handle, descriptor(input)))
}

/// Refuses properties that describe media other than the declared kind.
fn validate_media(input: &super::super::AssetEntryInput) -> Result<(), AssetReferenceError> {
    let Some(properties) = input.media_properties.value() else {
        return Ok(());
    };
    let matches = match input.media_kind.media_class() {
        super::super::AssetMediaClass::Image => properties.is_image(),
        super::super::AssetMediaClass::Audio => properties.is_audio(),
    };
    if !matches {
        return Err(AssetReferenceError::MediaKindMismatch);
    }
    Ok(())
}

/// Keeps a missing asset apart from a retrieval this boundary cannot serve.
fn validate_retrieval_state(
    input: &super::super::AssetEntryInput,
) -> Result<(), AssetReferenceError> {
    if !input.retrieval_state.has_observable_media() {
        for (name, absent) in [
            ("media_properties", input.media_properties.is_available()),
            ("byte_size", input.byte_size.is_available()),
            ("rendition", input.rendition.is_available()),
        ] {
            if absent {
                return Err(AssetReferenceError::MissingAssetCarriesData(name));
            }
        }
        return Ok(());
    }
    if !input.media_properties.is_available() {
        return Err(AssetReferenceError::MissingMediaProperties(
            "media_properties",
        ));
    }
    if !input.byte_size.is_available() {
        return Err(AssetReferenceError::MissingMediaProperties("byte_size"));
    }
    if !input.rendition.is_available() {
        return Err(AssetReferenceError::MissingMediaProperties("rendition"));
    }
    Ok(())
}

/// Refuses bytes on a metadata surface and enforces the owner-controlled rendition bounds.
fn validate_rendition(
    input: &super::super::AssetEntryInput,
    limits: &AssetRenditionLimits,
) -> Result<(), AssetReferenceError> {
    let Some(properties) = input.media_properties.value() else {
        return Ok(());
    };
    let Some(size) = input.byte_size.value() else {
        return Ok(());
    };
    enforce_media_limits(properties, size, limits)?;
    let Some(payload) = input.rendition.value() else {
        if input.retrieval_state.is_retrievable() {
            return Err(AssetReferenceError::InconsistentField("rendition"));
        }
        return Ok(());
    };
    if !input.retrieval_state.is_retrievable() {
        if payload.carries_binary() {
            return Err(AssetReferenceError::BinaryInMetadataSurface("rendition"));
        }
        return Ok(());
    }
    let AssetRenditionPayload::Binary { media_type, bytes } = payload else {
        return Err(AssetReferenceError::InconsistentField("rendition"));
    };
    if media_type != properties.media_type() {
        return Err(AssetReferenceError::MediaKindMismatch);
    }
    if bytes.is_empty() {
        return Err(AssetReferenceError::EmptyPresentCollection(
            "rendition_bytes",
        ));
    }
    if bytes.len() as u64 != size.stored_bytes() {
        return Err(AssetReferenceError::InconsistentField("byte_size"));
    }
    if bytes.len() as u64 > limits.max_bytes() {
        return Err(AssetReferenceError::OversizedRendition {
            limit: limits.max_bytes(),
            actual: bytes.len() as u64,
        });
    }
    Ok(())
}

/// Enforces the dimension, duration, decoded-byte and decode-ratio bounds.
fn enforce_media_limits(
    properties: &super::super::AssetMediaProperties,
    size: &super::super::AssetByteSize,
    limits: &AssetRenditionLimits,
) -> Result<(), AssetReferenceError> {
    if let Some(dimension) = properties.max_dimension()
        && dimension > limits.max_dimension()
    {
        return Err(AssetReferenceError::ExcessiveDimensions {
            limit: limits.max_dimension(),
            actual: dimension,
        });
    }
    if let Some(duration) = properties.duration_ms()
        && duration > limits.max_duration_ms()
    {
        return Err(AssetReferenceError::ExcessiveDuration {
            limit: limits.max_duration_ms(),
            actual: duration,
        });
    }
    if size.decoded_bytes() > limits.max_decoded_bytes() {
        return Err(AssetReferenceError::InflatedRendition {
            limit: limits.max_decoded_bytes(),
            actual: size.decoded_bytes(),
        });
    }
    if size.decode_ratio() > limits.max_decode_ratio() {
        return Err(AssetReferenceError::InflatedRendition {
            limit: limits.max_decode_ratio(),
            actual: size.decode_ratio(),
        });
    }
    Ok(())
}

/// States rendition availability as metadata that carries no bytes.
fn descriptor(input: &super::super::AssetEntryInput) -> AssetFieldValue<AssetRenditionDescriptor> {
    let status = input.rendition.status();
    if status != AssetFieldStatus::Available {
        return match status {
            AssetFieldStatus::NotApplicable => AssetFieldValue::not_applicable(),
            AssetFieldStatus::NotObserved => AssetFieldValue::not_observed(),
            AssetFieldStatus::Unsupported => AssetFieldValue::unsupported(),
            AssetFieldStatus::Withheld => AssetFieldValue::withheld(),
            AssetFieldStatus::Available => AssetFieldValue::not_observed(),
        };
    }
    let availability = match input.retrieval_state {
        AssetRetrievalState::Retrievable => super::super::AssetRenditionAvailability::Available,
        AssetRetrievalState::MetadataOnly => super::super::AssetRenditionAvailability::MetadataOnly,
        AssetRetrievalState::RetrievalUnavailable => {
            super::super::AssetRenditionAvailability::RetrievalUnavailable
        }
        AssetRetrievalState::AssetMissing => super::super::AssetRenditionAvailability::AssetMissing,
    };
    let media_type = input
        .media_properties
        .value()
        .map(|properties| properties.media_type().to_owned());
    let decoded_bytes = input
        .byte_size
        .value()
        .map(super::super::AssetByteSize::decoded_bytes);
    AssetFieldValue::available(AssetRenditionDescriptor {
        availability,
        media_type,
        decoded_bytes,
    })
}
