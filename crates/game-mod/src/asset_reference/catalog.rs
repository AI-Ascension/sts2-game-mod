// SPDX-License-Identifier: MIT

//! The owner-local snapshot, read port and producer that bind assets to one manifest.

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::error::map_source_error;
use super::validation::{validate_asset, validate_coverage};
use super::{
    ASSET_MAX_ASSETS, ASSET_REFERENCE_PRODUCER_VERSION, AssetCatalogBinding, AssetClassCoverage,
    AssetEntry, AssetEntryInput, AssetReadAvailability, AssetReferenceError, AssetRenditionLimits,
    AssetRenditionPayload, AssetSourceError, catalog_reader::AssetCatalog,
};

/// Bounded source snapshot used to construct one immutable asset catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetCatalogSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Locale every localized asset value was reported in.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
    /// Content-set revision the assets were read at.
    pub content_revision: String,
    /// Catalog generation the assets were read at.
    pub generation: u64,
    /// Per-class support state with the counts the source reports.
    pub classes: Vec<AssetClassCoverage>,
    /// Asset entries.
    pub assets: Vec<AssetEntryInput>,
}

/// Explicitly supported read seam for game-asset metadata and renditions.
pub trait AssetReadPort {
    /// Reports which read seam this port actually implements.
    fn read_availability(&self) -> AssetReadAvailability;

    /// Copies bounded asset records without changing game state.
    fn read_assets(
        &self,
        manifest: &ContentManifest,
    ) -> Result<AssetCatalogSnapshot, AssetSourceError>;
}

/// Fail-closed host boundary used until an exact-host asset read is authorized and verified.
#[derive(Debug, Default)]
pub struct UnavailableAssetHost;

impl AssetReadPort for UnavailableAssetHost {
    fn read_availability(&self) -> AssetReadAvailability {
        AssetReadAvailability::UnavailableHost
    }

    fn read_assets(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<AssetCatalogSnapshot, AssetSourceError> {
        Err(AssetSourceError::NoActiveSource)
    }
}

/// Producer that binds asset records to one manifest, locale, content revision and generation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AssetCatalogProducer;

impl AssetCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<P: AssetReadPort>(
        &self,
        manifest: &ContentManifest,
        port: &P,
        limits: &AssetRenditionLimits,
    ) -> Result<AssetCatalog, AssetReferenceError> {
        let snapshot = port.read_assets(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let binding = AssetCatalogBinding {
            manifest: snapshot.manifest,
            locale: snapshot.locale,
            content_revision: snapshot.content_revision,
            generation: snapshot.generation,
            producer_version: snapshot.producer_version,
        };
        let (assets, renditions) = collect_assets(snapshot.assets, manifest, &binding, limits)?;
        validate_coverage(&snapshot.classes, &assets)?;
        Ok(AssetCatalog::from_parts(
            binding,
            snapshot.classes,
            assets,
            renditions,
        ))
    }
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &AssetCatalogSnapshot,
) -> Result<(), AssetReferenceError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(AssetReferenceError::ManifestMismatch);
    }
    if snapshot.locale != manifest.locale {
        return Err(AssetReferenceError::LocaleMismatch);
    }
    if snapshot.producer_version != ASSET_REFERENCE_PRODUCER_VERSION {
        return Err(AssetReferenceError::ProducerVersionMismatch);
    }
    if snapshot.content_revision != manifest.content_set_revision {
        return Err(AssetReferenceError::ContentRevisionMismatch);
    }
    if snapshot.generation != manifest.catalog_generation {
        return Err(AssetReferenceError::GenerationMismatch);
    }
    if snapshot.assets.len() > ASSET_MAX_ASSETS {
        return Err(AssetReferenceError::InvalidInput("assets"));
    }
    Ok(())
}

/// Validated entries keyed by handle, with the rendition bytes split away from them.
type CollectedAssets = (
    BTreeMap<String, AssetEntry>,
    BTreeMap<String, AssetRenditionPayload>,
);

/// Validates every entry and splits its rendition bytes away from the metadata catalog.
fn collect_assets(
    inputs: Vec<AssetEntryInput>,
    manifest: &ContentManifest,
    binding: &AssetCatalogBinding,
    limits: &AssetRenditionLimits,
) -> Result<CollectedAssets, AssetReferenceError> {
    let mut assets = BTreeMap::new();
    let mut renditions = BTreeMap::new();
    for input in inputs {
        let (handle, descriptor) = validate_asset(&input, manifest, binding, limits)?;
        let payload = input.rendition.value().cloned();
        let key = handle.as_str().to_owned();
        let entry = AssetEntry::from_input(binding, handle, input, descriptor);
        if assets.insert(key.clone(), entry).is_some() {
            return Err(AssetReferenceError::DuplicateAsset(key));
        }
        if let Some(payload) = payload {
            renditions.insert(key, payload);
        }
    }
    Ok((assets, renditions))
}
