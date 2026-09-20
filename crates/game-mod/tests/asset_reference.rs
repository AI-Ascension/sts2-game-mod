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
    ASSET_REFERENCE_PRODUCER_VERSION, AssetCatalogProducer, AssetClassState, AssetField,
    AssetFieldStatus, AssetMediaClass, AssetMediaKind, AssetReadAvailability, AssetReadPort,
    AssetReferenceError, AssetRetrievalState, AssetSourceError, AssetVisibilityScope,
    UnavailableAssetHost,
};
use support::*;
use support_port::*;

fn entry(catalog: &sts2_game_mod::AssetCatalog, name: &str) -> sts2_game_mod::AssetEntry {
    catalog
        .get(
            &entry_reference(catalog, name),
            AssetVisibilityScope::Owner,
            None,
        )
        .expect("entry")
}

#[test]
fn producing_binds_every_asset_to_one_manifest_and_locale() {
    let manifest = fixture_manifest();
    let port = FixturePort::answering(snapshot(&manifest, fixture_assets(&manifest)));
    let catalog = produce_with(&manifest, &port).expect("catalog");

    assert_eq!(port.reads(), 1);
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        ASSET_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(
        catalog.binding().content_revision,
        manifest.content_set_revision
    );
    assert_eq!(catalog.binding().generation, manifest.catalog_generation);
    assert_eq!(catalog.revision().generation, manifest.catalog_generation);
    assert_eq!(
        catalog.revision().content_revision,
        manifest.content_set_revision
    );
}

#[test]
fn icon_art_and_audio_stay_distinct_kinds_and_classes() {
    let (_, catalog) = fixture_catalog();
    let icon = entry(&catalog, "asset.icon.strike");
    let art = entry(&catalog, "asset.art.bash");
    let audio = entry(&catalog, "asset.audio.blood");

    assert_eq!(icon.media_kind, AssetMediaKind::Icon);
    assert_eq!(art.media_kind, AssetMediaKind::Art);
    assert_eq!(audio.media_kind, AssetMediaKind::Audio);
    assert_eq!(icon.media_kind.media_class(), AssetMediaClass::Image);
    assert_eq!(art.media_kind.media_class(), AssetMediaClass::Image);
    assert_eq!(audio.media_kind.media_class(), AssetMediaClass::Audio);
    assert_ne!(icon.media_kind, art.media_kind);

    let icon_properties = icon.media_properties.value().expect("properties");
    let audio_properties = audio.media_properties.value().expect("properties");
    assert!(icon_properties.is_image());
    assert!(!icon_properties.is_audio());
    assert!(audio_properties.is_audio());
    assert_eq!(icon_properties.width(), Some(64));
    assert_eq!(icon_properties.height(), Some(64));
    assert_eq!(icon_properties.max_dimension(), Some(64));
    assert_eq!(audio_properties.width(), None);
    assert_eq!(audio_properties.max_dimension(), None);
    assert_eq!(audio_properties.duration_ms(), Some(12_000));
    assert_ne!(icon_properties.media_type(), audio_properties.media_type());
}

#[test]
fn retrievable_unavailable_and_missing_states_stay_distinct() {
    let (_, catalog) = fixture_catalog();

    assert_eq!(
        entry(&catalog, "asset.icon.strike").retrieval_state,
        AssetRetrievalState::Retrievable
    );
    assert_eq!(
        entry(&catalog, "asset.icon.metadata").retrieval_state,
        AssetRetrievalState::MetadataOnly
    );
    assert_eq!(
        entry(&catalog, "asset.icon.unavailable").retrieval_state,
        AssetRetrievalState::RetrievalUnavailable
    );
    assert_eq!(
        entry(&catalog, "asset.missing.spare").retrieval_state,
        AssetRetrievalState::AssetMissing
    );

    assert_ne!(
        AssetRetrievalState::MetadataOnly,
        AssetRetrievalState::RetrievalUnavailable
    );
    assert_ne!(
        AssetRetrievalState::RetrievalUnavailable,
        AssetRetrievalState::AssetMissing
    );
    assert!(AssetRetrievalState::Retrievable.is_retrievable());
    assert!(!AssetRetrievalState::MetadataOnly.is_retrievable());
    assert!(AssetRetrievalState::AssetMissing.is_absent_from_build());
    assert!(!AssetRetrievalState::RetrievalUnavailable.is_absent_from_build());
    assert!(!AssetRetrievalState::MetadataOnly.is_absent_from_build());
    assert!(AssetRetrievalState::MetadataOnly.is_stated());
}

#[test]
fn a_missing_asset_states_no_media_rather_than_zero() {
    let (_, catalog) = fixture_catalog();
    let missing = entry(&catalog, "asset.missing.spare");

    assert!(missing.media_properties.value().is_none());
    assert!(missing.byte_size.value().is_none());
    assert!(missing.rendition.value().is_none());
    assert_eq!(
        missing.field_status(AssetField::MediaProperties),
        Some(AssetFieldStatus::NotApplicable)
    );
    assert_eq!(
        missing.field_status(AssetField::ByteSize),
        Some(AssetFieldStatus::NotObserved)
    );
    assert!(!missing.retrieval_state.has_observable_media());
    assert!(!missing.retrieval_state.is_retrievable());
}

#[test]
fn metadata_only_and_unavailable_assets_state_their_media_without_bytes() {
    let (_, catalog) = fixture_catalog();
    for name in ["asset.icon.metadata", "asset.icon.unavailable"] {
        let asset = entry(&catalog, name);
        assert!(asset.media_properties.is_available());
        assert!(asset.byte_size.is_available());
        assert!(asset.retrieval_state.has_observable_media());
        assert!(!asset.retrieval_state.is_retrievable());
        let descriptor = asset.rendition.value().expect("descriptor");
        assert_eq!(
            descriptor.media_type.as_deref(),
            asset.media_properties.value().map(|it| it.media_type())
        );
    }
}

#[test]
fn every_retained_entry_states_every_field_row() {
    let (_, catalog) = fixture_catalog();
    let page = catalog
        .list(&query("en-US", AssetVisibilityScope::Owner, 64))
        .expect("page");
    assert!(page.complete);
    assert_eq!(page.total, 8);

    for summary in &page.assets {
        let asset = catalog
            .get(&summary.reference, AssetVisibilityScope::Owner, None)
            .expect("entry");
        assert!(asset.fields_are_declared());
        for field in AssetField::all() {
            assert!(asset.field_status(field).is_some(), "{}", field.name());
        }
        assert_eq!(
            asset.field_status(AssetField::MediaProperties),
            Some(asset.media_properties.status())
        );
        assert_eq!(
            asset.field_status(AssetField::ByteSize),
            Some(asset.byte_size.status())
        );
        assert_eq!(
            asset.field_status(AssetField::Rendition),
            Some(asset.rendition.status())
        );
    }
}

#[test]
fn class_coverage_states_every_class_with_the_observed_count() {
    let (_, catalog) = fixture_catalog();
    assert_eq!(catalog.classes().len(), AssetMediaClass::all().len());

    let image = catalog.class_state(AssetMediaClass::Image).expect("image");
    let audio = catalog.class_state(AssetMediaClass::Audio).expect("audio");
    assert_eq!(image.state, AssetClassState::Projected);
    assert_eq!(audio.state, AssetClassState::Projected);
    assert_eq!(image.asset_count, 6);
    assert_eq!(audio.asset_count, 2);
    assert!(image.unsupported_fields.is_empty());
    assert!(audio.unsupported_fields.is_empty());
    assert_ne!(image.class, audio.class);
}

#[test]
fn a_failing_source_produces_no_catalog() {
    let manifest = fixture_manifest();
    for (error, expected) in [
        (
            AssetSourceError::NoActiveSource,
            AssetReferenceError::NoActiveSource,
        ),
        (
            AssetSourceError::AccessDenied,
            AssetReferenceError::SourceAccessDenied,
        ),
        (
            AssetSourceError::RetrievalNotPermitted,
            AssetReferenceError::RetrievalNotPermitted,
        ),
        (
            AssetSourceError::Malformed,
            AssetReferenceError::MalformedSource,
        ),
    ] {
        let port = FixturePort::failing(error);
        assert_eq!(produce_with(&manifest, &port), Err(expected));
        assert_eq!(port.reads(), 1);
    }
}

#[test]
fn the_unavailable_host_states_no_active_source() {
    let manifest = fixture_manifest();
    let host = UnavailableAssetHost;
    assert_eq!(
        host.read_availability(),
        AssetReadAvailability::UnavailableHost
    );
    assert_eq!(
        AssetCatalogProducer::new().produce(&manifest, &host, &limits()),
        Err(AssetReferenceError::NoActiveSource)
    );
}

#[test]
fn the_fixture_port_states_a_synthetic_fixture_only_seam() {
    let port = FixturePort::answering(snapshot(&fixture_manifest(), Vec::new()));
    assert_eq!(
        port.read_availability(),
        AssetReadAvailability::SyntheticFixtureOnly
    );
}
