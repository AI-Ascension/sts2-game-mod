// SPDX-License-Identifier: MIT

//! The synthetic read port every asset fixture produces a catalog through.

use std::cell::Cell;

use sts2_game_mod::{
    AssetCatalog, AssetCatalogProducer, AssetCatalogSnapshot, AssetEntryInput, AssetEntryReference,
    AssetHandle, AssetListQuery, AssetReadAvailability, AssetReadPort, AssetReferenceError,
    AssetRenditionLimits, AssetSourceError, AssetVisibilityScope, ContentManifest,
};

use crate::manifest_fixture::fixture_manifest;
use crate::support::{fixture_assets, snapshot};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixturePort {
    pub outcome: Result<AssetCatalogSnapshot, AssetSourceError>,
    reads: Cell<usize>,
}

impl FixturePort {
    pub fn answering(snapshot: AssetCatalogSnapshot) -> Self {
        Self {
            outcome: Ok(snapshot),
            reads: Cell::new(0),
        }
    }

    pub fn failing(error: AssetSourceError) -> Self {
        Self {
            outcome: Err(error),
            reads: Cell::new(0),
        }
    }

    pub fn reads(&self) -> usize {
        self.reads.get()
    }
}

impl AssetReadPort for FixturePort {
    fn read_availability(&self) -> AssetReadAvailability {
        AssetReadAvailability::SyntheticFixtureOnly
    }

    fn read_assets(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<AssetCatalogSnapshot, AssetSourceError> {
        self.reads.set(self.reads.get() + 1);
        self.outcome.clone()
    }
}

pub fn limits() -> AssetRenditionLimits {
    AssetRenditionLimits::default()
}

pub fn produce_with(
    manifest: &ContentManifest,
    port: &FixturePort,
) -> Result<AssetCatalog, AssetReferenceError> {
    AssetCatalogProducer::new().produce(manifest, port, &limits())
}

pub fn produce(manifest: &ContentManifest, assets: Vec<AssetEntryInput>) -> AssetCatalog {
    produce_with(
        manifest,
        &FixturePort::answering(snapshot(manifest, assets)),
    )
    .expect("catalog")
}

/// The shared catalog: eight assets over the fixture manifest.
pub fn fixture_catalog() -> (ContentManifest, AssetCatalog) {
    let manifest = fixture_manifest();
    let catalog = produce(&manifest, fixture_assets(&manifest));
    (manifest, catalog)
}

pub fn handle(name: &str) -> AssetHandle {
    AssetHandle::new(name).expect("handle")
}

pub fn entry_reference(catalog: &AssetCatalog, name: &str) -> AssetEntryReference {
    AssetEntryReference {
        catalog: catalog.binding().clone(),
        handle: handle(name),
    }
}

pub fn query(locale: &str, scope: AssetVisibilityScope, limit: usize) -> AssetListQuery {
    AssetListQuery {
        locale: locale.to_owned(),
        scope,
        revision: None,
        media_kind: None,
        retrieval_state: None,
        limit,
        continuation: None,
    }
}
