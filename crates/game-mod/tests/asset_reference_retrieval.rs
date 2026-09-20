// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/asset_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/asset_reference.rs"]
mod support;
#[path = "support/asset_reference_port.rs"]
mod support_port;

use manifest_fixture::fixture_manifest;
use sts2_game_mod::{
    ASSET_MAX_DECODE_RATIO, ASSET_MAX_DECODED_BYTES, ASSET_MAX_RENDITION_BYTES,
    ASSET_MAX_RENDITION_DIMENSION, ASSET_MAX_RENDITION_DURATION_MS, AssetByteSize, AssetCatalog,
    AssetCatalogSnapshot, AssetEntryInput, AssetFieldValue, AssetMediaProperties,
    AssetReferenceError, AssetRenditionAuthority, AssetRenditionAvailability, AssetRenditionLimits,
    AssetRenditionPayload, AssetRenditionRequest, AssetVisibilityScope, ContentManifest,
};
use support::*;
use support_port::*;

fn request(name: &str, limits: AssetRenditionLimits) -> AssetRenditionRequest {
    AssetRenditionRequest::new(handle(name), limits, None).expect("request")
}

fn custom_limits(
    max_bytes: u64,
    max_dimension: u32,
    max_duration_ms: u64,
    max_decoded_bytes: u64,
) -> AssetRenditionLimits {
    AssetRenditionLimits::new(max_bytes, max_dimension, max_duration_ms, max_decoded_bytes)
        .expect("limits")
}

fn snapshot_of(
    manifest: &ContentManifest,
    edit: impl FnOnce(&mut AssetEntryInput),
) -> AssetCatalogSnapshot {
    let mut entries: Vec<AssetEntryInput> = fixture_assets(manifest);
    edit(&mut entries[0]);
    snapshot(manifest, entries)
}

#[test]
fn a_retrievable_asset_serves_its_bounded_bytes() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let rendition = reader
        .retrieve(
            &request("asset.icon.strike", limits()),
            AssetVisibilityScope::Owner,
        )
        .expect("rendition");

    assert_eq!(
        rendition.availability,
        AssetRenditionAvailability::Available
    );
    assert_eq!(rendition.authority, AssetRenditionAuthority::NotGranted);
    assert!(rendition.carries_binary());
    assert!(rendition.is_consistent());
    assert_eq!(rendition.payload.media_type(), Some("image/png"));
    assert_eq!(rendition.payload.bytes().expect("bytes").len(), 512);
    assert_eq!(rendition.handle.as_str(), "asset.icon.strike");
}

#[test]
fn a_metadata_only_asset_states_that_its_bytes_are_withheld() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let rendition = reader
        .retrieve(
            &request("asset.icon.metadata", limits()),
            AssetVisibilityScope::Owner,
        )
        .expect("rendition");

    assert_eq!(
        rendition.availability,
        AssetRenditionAvailability::MetadataOnly
    );
    assert_eq!(rendition.payload, AssetRenditionPayload::MetadataOnly);
    assert!(!rendition.carries_binary());
    assert!(rendition.is_consistent());
    assert_eq!(rendition.payload.bytes(), None);
    assert_eq!(rendition.payload.media_type(), None);
    assert_eq!(rendition.authority, AssetRenditionAuthority::NotGranted);
}

#[test]
fn an_unservable_asset_stays_distinct_from_a_missing_one() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let unavailable = reader
        .retrieve(
            &request("asset.icon.unavailable", limits()),
            AssetVisibilityScope::Owner,
        )
        .expect("rendition");
    let missing = reader
        .retrieve(
            &request("asset.missing.spare", limits()),
            AssetVisibilityScope::Owner,
        )
        .expect("rendition");

    assert_eq!(
        unavailable.availability,
        AssetRenditionAvailability::RetrievalUnavailable
    );
    assert_eq!(
        missing.availability,
        AssetRenditionAvailability::AssetMissing
    );
    assert_ne!(unavailable.availability, missing.availability);
    assert_eq!(unavailable.payload, AssetRenditionPayload::MetadataOnly);
    assert_eq!(missing.payload, AssetRenditionPayload::MetadataOnly);
    assert!(!missing.carries_binary());
}

#[test]
fn retrieving_with_a_forged_handle_is_refused() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    assert_eq!(
        reader.retrieve(
            &request("asset.forged.ghost", limits()),
            AssetVisibilityScope::Owner
        ),
        Err(AssetReferenceError::ForgedHandle(
            "asset.forged.ghost".to_owned()
        ))
    );
}

#[test]
fn an_expired_reference_stops_resolving() {
    let manifest = fixture_manifest();
    let expired = expired_entry(&manifest);
    assert_eq!(expired.expires_at_generation, manifest.catalog_generation);
    let catalog = produce(&manifest, vec![expired, icon_entry(&manifest)]);
    let reader = catalog.reader();
    assert_eq!(catalog.binding().generation, manifest.catalog_generation);
    assert_eq!(
        reader.retrieve(
            &request("asset.icon.expired", limits()),
            AssetVisibilityScope::Owner
        ),
        Err(AssetReferenceError::ExpiredHandle)
    );
    assert!(
        reader
            .retrieve(
                &request("asset.icon.strike", limits()),
                AssetVisibilityScope::Owner
            )
            .is_ok()
    );
}

#[test]
fn a_rendition_a_scope_may_not_observe_is_refused() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    assert_eq!(
        reader.retrieve(
            &request("asset.owner.ambience", limits()),
            AssetVisibilityScope::Public
        ),
        Err(AssetReferenceError::ExcludedByScope)
    );
    assert_eq!(
        reader.retrieve(
            &request("asset.hidden.marker", limits()),
            AssetVisibilityScope::Reference
        ),
        Err(AssetReferenceError::ExcludedByScope)
    );
    let owner_only = reader
        .retrieve(
            &request("asset.owner.ambience", limits()),
            AssetVisibilityScope::Owner,
        )
        .expect("rendition");
    assert_eq!(
        owner_only.availability,
        AssetRenditionAvailability::MetadataOnly
    );
}

#[test]
fn a_requested_media_type_that_contradicts_the_asset_is_refused() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let mismatched =
        AssetRenditionRequest::new(handle("asset.icon.strike"), limits(), Some("image/jpeg"))
            .expect("request");
    assert_eq!(
        reader.retrieve(&mismatched, AssetVisibilityScope::Owner),
        Err(AssetReferenceError::UnsupportedMediaType(
            "image/jpeg".to_owned()
        ))
    );
    let matching =
        AssetRenditionRequest::new(handle("asset.icon.strike"), limits(), Some("image/png"))
            .expect("request");
    assert!(
        reader
            .retrieve(&matching, AssetVisibilityScope::Owner)
            .is_ok()
    );
}

#[test]
fn a_request_bound_below_the_declared_bytes_refuses_the_rendition() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let tight = custom_limits(
        256,
        ASSET_MAX_RENDITION_DIMENSION,
        ASSET_MAX_RENDITION_DURATION_MS,
        ASSET_MAX_DECODED_BYTES,
    );
    assert_eq!(
        reader.retrieve(
            &request("asset.icon.strike", tight),
            AssetVisibilityScope::Owner
        ),
        Err(AssetReferenceError::OversizedRendition {
            limit: 256,
            actual: 512,
        })
    );
}

#[test]
fn a_decompression_heavy_asset_is_refused_when_it_is_declared() {
    let manifest = fixture_manifest();
    let media = image_properties(64, 64);
    let snapshot = snapshot_of(&manifest, |entry| {
        entry.media_properties = AssetFieldValue::available(media.clone());
        entry.byte_size = AssetFieldValue::available(AssetByteSize::new(64, 8_192).expect("size"));
        entry.rendition = AssetFieldValue::available(binary(media.media_type(), 64));
    });
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::InflatedRendition {
            limit: ASSET_MAX_DECODE_RATIO,
            actual: 128,
        })
    );
}

#[test]
fn a_declared_asset_beyond_the_decoded_byte_bound_is_refused() {
    let manifest = fixture_manifest();
    let media = image_properties(64, 64);
    let decoded = 20 * 1024 * 1024;
    let stored = 8 * 1024 * 1024;
    let snapshot = snapshot_of(&manifest, |entry| {
        entry.media_properties = AssetFieldValue::available(media.clone());
        entry.byte_size =
            AssetFieldValue::available(AssetByteSize::new(stored, decoded).expect("size"));
        entry.rendition = AssetFieldValue::available(binary(media.media_type(), stored));
    });
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::InflatedRendition {
            limit: ASSET_MAX_DECODED_BYTES,
            actual: decoded,
        })
    );
}

#[test]
fn a_request_bound_below_the_declared_decoded_size_refuses_the_rendition() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let tight = custom_limits(
        ASSET_MAX_RENDITION_BYTES,
        ASSET_MAX_RENDITION_DIMENSION,
        ASSET_MAX_RENDITION_DURATION_MS,
        1_024,
    );
    assert_eq!(
        reader.retrieve(
            &request("asset.art.bash", tight),
            AssetVisibilityScope::Owner
        ),
        Err(AssetReferenceError::InflatedRendition {
            limit: 1_024,
            actual: 2_048,
        })
    );
}

#[test]
fn a_declared_asset_beyond_the_dimension_bound_is_refused() {
    let manifest = fixture_manifest();
    let media = image_properties(8_192, 64);
    let snapshot = snapshot_of(&manifest, |entry| {
        entry.media_properties = AssetFieldValue::available(media.clone());
    });
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::ExcessiveDimensions {
            limit: ASSET_MAX_RENDITION_DIMENSION,
            actual: 8_192,
        })
    );

    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let tight = custom_limits(
        ASSET_MAX_RENDITION_BYTES,
        32,
        ASSET_MAX_RENDITION_DURATION_MS,
        ASSET_MAX_DECODED_BYTES,
    );
    assert_eq!(
        reader.retrieve(
            &request("asset.icon.strike", tight),
            AssetVisibilityScope::Owner
        ),
        Err(AssetReferenceError::ExcessiveDimensions {
            limit: 32,
            actual: 64,
        })
    );
}

#[test]
fn a_declared_asset_beyond_the_duration_bound_is_refused() {
    let manifest = fixture_manifest();
    let mut entries = fixture_assets(&manifest);
    entries[2].media_properties = AssetFieldValue::available(audio_properties(400_000));
    assert_eq!(
        produce_with(
            &manifest,
            &FixturePort::answering(snapshot(&manifest, entries))
        )
        .expect_err("refusal"),
        AssetReferenceError::ExcessiveDuration {
            limit: ASSET_MAX_RENDITION_DURATION_MS,
            actual: 400_000,
        }
    );

    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    let tight = custom_limits(
        ASSET_MAX_RENDITION_BYTES,
        ASSET_MAX_RENDITION_DIMENSION,
        1_000,
        ASSET_MAX_DECODED_BYTES,
    );
    assert_eq!(
        reader.retrieve(
            &request("asset.audio.blood", tight),
            AssetVisibilityScope::Owner
        ),
        Err(AssetReferenceError::ExcessiveDuration {
            limit: 1_000,
            actual: 12_000,
        })
    );
}

#[test]
fn media_properties_carry_no_hidden_unit() {
    let (_, catalog) = fixture_catalog();
    let audio = catalog
        .get(
            &entry_reference(&catalog, "asset.audio.blood"),
            AssetVisibilityScope::Owner,
            None,
        )
        .expect("entry");
    let properties: &AssetMediaProperties = audio.media_properties.value().expect("properties");
    assert_eq!(properties.duration_ms(), Some(12_000));
    assert_eq!(properties.width(), None);
    assert_eq!(properties.max_dimension(), None);
    assert!(properties.is_audio());

    let icon = catalog
        .get(
            &entry_reference(&catalog, "asset.icon.strike"),
            AssetVisibilityScope::Owner,
            None,
        )
        .expect("entry");
    let icon_properties = icon.media_properties.value().expect("properties");
    assert_eq!(icon_properties.duration_ms(), None);
    assert_eq!(icon_properties.width(), Some(64));
}

#[test]
fn a_catalog_reader_serves_one_catalog_under_its_own_authority() {
    let (_, catalog) = fixture_catalog();
    let reader = catalog.reader();
    assert_eq!(reader.authority(), AssetRenditionAuthority::NotGranted);
    assert_eq!(reader.revision(), catalog.revision());
    assert_eq!(reader.catalog().binding(), catalog.binding());
    assert_eq!(reader.catalog().locale(), "en-US");
}

#[test]
fn the_fixture_catalog_is_stable_across_productions() {
    let first: AssetCatalog = produce(&fixture_manifest(), fixture_assets(&fixture_manifest()));
    let second = produce(&fixture_manifest(), fixture_assets(&fixture_manifest()));
    assert_eq!(first, second);
    assert_eq!(first.binding(), second.binding());
}
