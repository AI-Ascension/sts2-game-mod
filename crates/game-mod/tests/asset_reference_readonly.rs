// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/asset_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/asset_reference.rs"]
mod support;
#[path = "support/asset_reference_port.rs"]
mod support_port;

use sts2_game_mod::{
    ASSET_MAX_DECODE_RATIO, ASSET_MAX_DECODED_BYTES, ASSET_MAX_RENDITION_BYTES,
    ASSET_MAX_RENDITION_DIMENSION, ASSET_MAX_RENDITION_DURATION_MS, AssetFieldStatus,
    AssetFieldValue, AssetHandle, AssetMediaProperties, AssetReferenceError,
    AssetRenditionAuthority, AssetRenditionLimits, AssetRenditionPayload, AssetVisibilityScope,
    is_markup_media_type, is_opaque_handle,
};
use support_port::*;

#[test]
fn no_metadata_surface_carries_bytes() {
    let (_, catalog) = fixture_catalog();
    let page = catalog
        .list(&query("en-US", AssetVisibilityScope::Owner, 64))
        .expect("page");
    assert!(page.assets.iter().all(|summary| !summary.carries_binary()));

    let entry = catalog
        .get(
            &entry_reference(&catalog, "asset.icon.strike"),
            AssetVisibilityScope::Owner,
            None,
        )
        .expect("entry");
    let descriptor = entry.rendition.value().expect("descriptor");
    assert_eq!(descriptor.decoded_bytes, Some(1_024));
    assert_eq!(descriptor.media_type.as_deref(), Some("image/png"));
    assert_eq!(entry.byte_size.value().expect("size").stored_bytes(), 512);
}

#[test]
fn a_handle_cannot_name_a_locator() {
    for opaque in ["asset.icon.strike", "art.card_bash-1", "icon#2"] {
        assert!(is_opaque_handle(opaque), "{opaque}");
        assert!(AssetHandle::new(opaque).is_ok(), "{opaque}");
    }
    assert!(!is_opaque_handle(""));
    for locator in [
        "a/b",
        "a\\b",
        "file:x",
        "anchor..dot",
        "C:/assets/icon.png",
        "https://example.invalid/x.png",
    ] {
        assert!(!is_opaque_handle(locator), "{locator}");
        assert_eq!(
            AssetHandle::new(locator),
            Err(AssetReferenceError::PathLikeHandle),
            "{locator}"
        );
    }
}

#[test]
fn media_types_a_host_could_run_as_markup_are_refused() {
    for markup in [
        "text/plain",
        "text/html",
        "application/xml",
        "image/svg+xml",
        "imagepng",
    ] {
        assert!(is_markup_media_type(markup), "{markup}");
    }
    for inert in ["image/png", "audio/ogg", "image/webp", "audio/mpeg"] {
        assert!(!is_markup_media_type(inert), "{inert}");
    }

    assert_eq!(
        AssetMediaProperties::new("text/plain", None, None, None),
        Err(AssetReferenceError::MarkupMediaType(
            "text/plain".to_owned()
        ))
    );
    assert_eq!(
        AssetMediaProperties::new("image/svg+xml", Some(8), Some(8), None),
        Err(AssetReferenceError::MarkupMediaType(
            "image/svg+xml".to_owned()
        ))
    );
    assert!(AssetMediaProperties::new("image/png", Some(8), Some(8), None).is_ok());
}

#[test]
fn malformed_media_types_are_refused() {
    for malformed in [
        "",
        "image/PNG",
        "imagepng",
        "image/png ",
        "image//png",
        "é/png",
    ] {
        assert_eq!(
            AssetMediaProperties::new(malformed, None, None, None),
            Err(AssetReferenceError::UnsupportedMediaType(
                malformed.to_owned()
            )),
            "{malformed}"
        );
    }
}

#[test]
fn rendition_limits_are_clamped_to_the_hard_ceilings() {
    let full = (
        ASSET_MAX_RENDITION_BYTES,
        ASSET_MAX_RENDITION_DIMENSION,
        ASSET_MAX_RENDITION_DURATION_MS,
        ASSET_MAX_DECODED_BYTES,
    );
    assert!(AssetRenditionLimits::new(full.0, full.1, full.2, full.3).is_ok());

    for (arguments, field) in [
        ((0, full.1, full.2, full.3), "max_bytes"),
        ((full.0 + 1, full.1, full.2, full.3), "max_bytes"),
        ((full.0, 0, full.2, full.3), "max_dimension"),
        ((full.0, full.1 + 1, full.2, full.3), "max_dimension"),
        ((full.0, full.1, 0, full.3), "max_duration_ms"),
        ((full.0, full.1, full.2 + 1, full.3), "max_duration_ms"),
        ((full.0, full.1, full.2, 0), "max_decoded_bytes"),
        ((full.0, full.1, full.2, full.3 + 1), "max_decoded_bytes"),
    ] {
        assert_eq!(
            AssetRenditionLimits::new(arguments.0, arguments.1, arguments.2, arguments.3),
            Err(AssetReferenceError::InvalidRenditionLimit(field)),
            "{field}"
        );
    }

    let default = AssetRenditionLimits::default();
    assert_eq!(default.max_bytes(), ASSET_MAX_RENDITION_BYTES);
    assert_eq!(default.max_dimension(), ASSET_MAX_RENDITION_DIMENSION);
    assert_eq!(default.max_duration_ms(), ASSET_MAX_RENDITION_DURATION_MS);
    assert_eq!(default.max_decoded_bytes(), ASSET_MAX_DECODED_BYTES);
    assert_eq!(default.max_decode_ratio(), ASSET_MAX_DECODE_RATIO);
}

#[test]
fn a_rendition_states_the_capability_it_withholds() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    assert_eq!(reader.authority(), AssetRenditionAuthority::NotGranted);
    let page = catalog
        .list(&query("en-US", AssetVisibilityScope::Owner, 64))
        .expect("page");
    assert_eq!(page.authority, AssetRenditionAuthority::NotGranted);

    let request =
        sts2_game_mod::AssetRenditionRequest::new(handle("asset.icon.strike"), limits(), None)
            .expect("request");
    let rendition = reader
        .retrieve(&request, AssetVisibilityScope::Owner)
        .expect("rendition");
    assert_eq!(rendition.authority, AssetRenditionAuthority::NotGranted);
}

#[test]
fn a_field_value_never_states_a_value_without_its_status() {
    let available = AssetFieldValue::available(7_u32);
    assert_eq!(available.status(), AssetFieldStatus::Available);
    assert!(available.is_available());
    assert!(!available.status().is_stated());
    assert_eq!(available.value(), Some(&7));
    assert!(available.is_consistent());
    assert_eq!(available.into_value(), Some(7));

    for absent in [
        AssetFieldValue::<u32>::not_applicable(),
        AssetFieldValue::<u32>::not_observed(),
        AssetFieldValue::<u32>::unsupported(),
        AssetFieldValue::<u32>::withheld(),
    ] {
        assert!(!absent.is_available());
        assert!(absent.status().is_stated());
        assert_eq!(absent.value(), None);
        assert!(absent.is_consistent());
        assert_eq!(absent.into_value(), None);
    }

    assert_eq!(
        AssetRenditionPayload::MetadataOnly.bytes(),
        None,
        "a metadata-only payload never yields bytes"
    );
}
