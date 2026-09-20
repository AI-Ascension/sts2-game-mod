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
    ASSET_MAX_ASSETS, AssetClassState, AssetEntryInput, AssetField, AssetMediaClass,
    AssetReferenceError, ContentManifest,
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

fn fences(manifest: &ContentManifest) -> sts2_game_mod::AssetCatalogSnapshot {
    snapshot(manifest, fixture_assets(manifest))
}

#[test]
fn a_snapshot_naming_another_manifest_is_refused() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    snapshot.manifest.catalog_generation += 1;
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::ManifestMismatch)
    );
}

#[test]
fn a_snapshot_using_another_locale_is_refused() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    snapshot.locale = "de-DE".to_owned();
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::LocaleMismatch)
    );
}

#[test]
fn a_snapshot_using_another_producer_identity_is_refused() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    snapshot.producer_version = "asset-producer-v0".to_owned();
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::ProducerVersionMismatch)
    );
}

#[test]
fn a_snapshot_naming_another_content_revision_is_refused() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    snapshot.content_revision = "revision.other".to_owned();
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::ContentRevisionMismatch)
    );
}

#[test]
fn a_snapshot_naming_another_generation_is_refused() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    snapshot.generation = manifest.catalog_generation + 1;
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::GenerationMismatch)
    );
}

#[test]
fn a_snapshot_beyond_the_asset_bound_is_refused() {
    let manifest = fixture_manifest();
    let template = icon_entry(&manifest);
    let mut entries = Vec::new();
    for index in 0..=ASSET_MAX_ASSETS {
        let mut entry = template.clone();
        entry.handle = format!("asset.bulk.{index}");
        entries.push(entry);
    }
    assert_eq!(entries.len(), ASSET_MAX_ASSETS + 1);
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::InvalidInput("assets")
    );
}

#[test]
fn class_coverage_must_state_every_class_exactly_once() {
    let manifest = fixture_manifest();
    let mut missing = fences(&manifest);
    missing.classes.truncate(1);
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(missing)),
        Err(AssetReferenceError::ClassCoverageIncomplete)
    );

    let mut duplicated = fences(&manifest);
    duplicated.classes[1].class = AssetMediaClass::Image;
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(duplicated)),
        Err(AssetReferenceError::ClassCoverageIncomplete)
    );
}

#[test]
fn declared_and_observed_class_counts_must_agree() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    snapshot.classes[0].asset_count += 1;
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::ClassCountMismatch)
    );
}

#[test]
fn an_asset_in_an_unprojected_class_is_refused() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    let audio = snapshot
        .classes
        .iter_mut()
        .find(|row| row.class == AssetMediaClass::Audio)
        .expect("audio row");
    assert_eq!(audio.asset_count, 2);
    audio.state = AssetClassState::Unsupported;
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::UnavailableClass)
    );
}

#[test]
fn a_class_row_may_not_leave_a_field_it_does_not_project_at_available() {
    let manifest = fixture_manifest();
    let mut snapshot = fences(&manifest);
    let image = snapshot
        .classes
        .iter_mut()
        .find(|row| row.class == AssetMediaClass::Image)
        .expect("image row");
    image.unsupported_fields = vec![AssetField::MediaProperties];
    assert_eq!(
        produce_with(&manifest, &FixturePort::answering(snapshot)),
        Err(AssetReferenceError::InconsistentField("media_properties"))
    );
}

#[test]
fn an_asset_linked_to_an_unhandled_family_is_refused() {
    let manifest = fixture_manifest();
    let mut entries = fixture_assets(&manifest);
    entries[0].definition = definition("encounter", "encounter.gremlin");
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::UnhandledManifestFamily("encounter".to_owned())
    );
}

#[test]
fn an_asset_linked_to_an_absent_definition_is_refused() {
    let manifest = fixture_manifest();
    let mut entries = fixture_assets(&manifest);
    entries[0].definition = definition("card", "card.unknown");
    assert_eq!(
        refused(&manifest, entries),
        AssetReferenceError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card.unknown".to_owned(),
        }
    );
}

#[test]
fn an_asset_linked_to_a_definition_another_manifest_handles_is_refused() {
    let manifest = manifest_fixture::unhandled_manifest();
    let mut entry = icon_entry(&manifest);
    entry.definition = definition("card", "card.strike");
    assert_eq!(
        refused(&manifest, vec![entry]),
        AssetReferenceError::UnhandledManifestFamily("card".to_owned())
    );
}
