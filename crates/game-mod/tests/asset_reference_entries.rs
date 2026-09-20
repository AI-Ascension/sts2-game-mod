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
    AssetByteSize, AssetEntryInput, AssetField, AssetFieldStatus, AssetFieldValue, AssetHandle,
    AssetMediaProperties, AssetReferenceError, AssetRenditionPayload, ContentManifest,
};
use support::*;
use support_port::*;

fn refused(manifest: &ContentManifest, entries: Vec<AssetEntryInput>) -> AssetReferenceError {
    produce_with(
        manifest,
        &FixturePort::answering(snapshot(manifest, entries)),
    )
    .expect_err("refusal")
}

fn breaking(
    manifest: &ContentManifest,
    index: usize,
    edit: impl FnOnce(&mut AssetEntryInput),
) -> Vec<AssetEntryInput> {
    let mut entries = fixture_assets(manifest);
    edit(&mut entries[index]);
    entries
}

#[test]
fn a_handle_that_names_a_path_or_a_url_is_refused() {
    for locator in [
        "../secrets.png",
        "assets/icon.png",
        "https://example.invalid/icon.png",
        "C:\\assets\\icon.png",
        "file:icon.png",
        "icon..png",
    ] {
        assert_eq!(
            AssetHandle::new(locator),
            Err(AssetReferenceError::PathLikeHandle),
            "{locator}"
        );
    }
    assert!(AssetHandle::new("asset.icon.strike").is_ok());
}

#[test]
fn an_empty_or_oversized_handle_is_refused() {
    assert_eq!(
        AssetHandle::new(""),
        Err(AssetReferenceError::InvalidInput("handle"))
    );
    assert_eq!(
        AssetHandle::new(&"a".repeat(257)),
        Err(AssetReferenceError::InvalidInput("handle"))
    );
}

#[test]
fn an_entry_whose_handle_is_a_locator_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        entry.handle = "../../etc/passwd".to_owned();
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::PathLikeHandle
    );
}

#[test]
fn two_assets_may_not_share_one_handle() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 1, |entry| {
        entry.handle = "asset.icon.strike".to_owned();
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::DuplicateAsset("asset.icon.strike".to_owned())
    );
}

#[test]
fn media_properties_describing_another_kind_are_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        entry.media_properties = AssetFieldValue::available(audio_properties(1_000));
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::MediaKindMismatch
    );
}

#[test]
fn a_retrievable_asset_must_state_its_media() {
    let manifest = fixture_manifest();
    let mut entries = breaking(&manifest, 0, |entry| {
        entry.media_properties = AssetFieldValue::not_observed();
    });
    entries[0].fields = field_rows(
        AssetFieldStatus::NotObserved,
        AssetFieldStatus::Available,
        AssetFieldStatus::Available,
    );
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::MissingMediaProperties("media_properties")
    );

    let mut entries = breaking(&manifest, 0, |entry| {
        entry.byte_size = AssetFieldValue::not_observed();
    });
    entries[0].fields = field_rows(
        AssetFieldStatus::Available,
        AssetFieldStatus::NotObserved,
        AssetFieldStatus::Available,
    );
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::MissingMediaProperties("byte_size")
    );

    let mut entries = breaking(&manifest, 0, |entry| {
        entry.rendition = AssetFieldValue::not_observed();
    });
    entries[0].fields = field_rows(
        AssetFieldStatus::Available,
        AssetFieldStatus::Available,
        AssetFieldStatus::NotObserved,
    );
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::MissingMediaProperties("rendition")
    );
}

#[test]
fn a_missing_asset_carrying_declared_data_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 5, |entry| {
        assert_eq!(
            entry.retrieval_state,
            sts2_game_mod::AssetRetrievalState::AssetMissing
        );
        entry.media_properties = AssetFieldValue::available(image_properties(8, 8));
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::MissingAssetCarriesData("media_properties")
    );
}

#[test]
fn bytes_on_a_metadata_surface_are_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 3, |entry| {
        entry.rendition = AssetFieldValue::available(binary("image/png", 256));
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::BinaryInMetadataSurface("rendition")
    );
}

#[test]
fn a_metadata_only_payload_on_a_retrievable_asset_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        entry.rendition = AssetFieldValue::available(AssetRenditionPayload::MetadataOnly);
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::InconsistentField("rendition")
    );
}

#[test]
fn bytes_that_contradict_the_declared_stored_size_are_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        entry.rendition = AssetFieldValue::available(binary("image/png", 128));
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::InconsistentField("byte_size")
    );
}

#[test]
fn an_entry_omitting_a_field_row_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        entry
            .fields
            .retain(|row| row.field != AssetField::MediaProperties);
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::MissingFieldCoverage("media_properties")
    );
}

#[test]
fn an_entry_stating_a_field_row_twice_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        let row = entry.fields[0];
        entry.fields.push(row);
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::DuplicateFieldCoverage("handle")
    );
}

#[test]
fn a_field_row_disagreeing_with_its_carrier_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        entry.fields[0].status = AssetFieldStatus::Withheld;
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::InconsistentField("handle")
    );
}

#[test]
fn zero_and_half_stated_media_measurements_are_refused() {
    assert_eq!(
        AssetMediaProperties::new("image/png", Some(0), Some(0), None),
        Err(AssetReferenceError::MissingMediaProperties("width"))
    );
    assert_eq!(
        AssetMediaProperties::new("image/png", Some(4), Some(0), None),
        Err(AssetReferenceError::MissingMediaProperties("height"))
    );
    assert_eq!(
        AssetMediaProperties::new("image/png", Some(4), None, None),
        Err(AssetReferenceError::MissingMediaProperties("dimensions"))
    );
    assert_eq!(
        AssetMediaProperties::new("audio/ogg", None, None, Some(0)),
        Err(AssetReferenceError::MissingMediaProperties("duration_ms"))
    );
}

#[test]
fn a_byte_size_below_its_decoded_size_is_refused() {
    assert_eq!(
        AssetByteSize::new(0, 1),
        Err(AssetReferenceError::InvalidInput("stored_bytes"))
    );
    assert_eq!(
        AssetByteSize::new(10, 5),
        Err(AssetReferenceError::InvalidInput("decoded_bytes"))
    );
    let size = AssetByteSize::new(64, 4_096).expect("size");
    assert_eq!(size.decode_ratio(), 64);
    assert_eq!(
        AssetByteSize::new(512, 512).expect("size").decode_ratio(),
        1
    );
}

#[test]
fn an_asset_declaring_more_bytes_than_the_hard_ceiling_is_refused() {
    let manifest = fixture_manifest();
    let entries = breaking(&manifest, 0, |entry| {
        let media = image_properties(64, 64);
        entry.media_properties = AssetFieldValue::available(media.clone());
        entry.byte_size = AssetFieldValue::available(
            AssetByteSize::new(
                sts2_game_mod::ASSET_MAX_RENDITION_BYTES + 1,
                8 * 1024 * 1024,
            )
            .expect("size"),
        );
        entry.rendition = AssetFieldValue::available(binary(
            media.media_type(),
            sts2_game_mod::ASSET_MAX_RENDITION_BYTES + 1,
        ));
    });
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::OversizedRendition {
            limit: sts2_game_mod::ASSET_MAX_RENDITION_BYTES,
            actual: sts2_game_mod::ASSET_MAX_RENDITION_BYTES + 1,
        }
    );
}
