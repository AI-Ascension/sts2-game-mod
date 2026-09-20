// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    AssetCatalogBinding, AssetClassCoverage, AssetDetailQuery, AssetEntry, AssetEntryReference,
    AssetMediaClass, AssetReferenceError, AssetRenditionPayload, AssetRevision,
    AssetVisibilityScope,
};
use super::page::{AssetListQuery, AssetPage};
use super::reader::AssetCatalogReader;

/// Immutable asset entries keyed by opaque handle, with their declared class coverage.
///
/// The retained catalog holds rendition bytes in a private side table, so metadata lookups and
/// listings can never return a blob and only [`AssetCatalogReader::retrieve`] can.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetCatalog {
    pub(super) binding: AssetCatalogBinding,
    pub(super) classes: Vec<AssetClassCoverage>,
    pub(super) assets: BTreeMap<String, AssetEntry>,
    pub(super) renditions: BTreeMap<String, AssetRenditionPayload>,
}

impl AssetCatalog {
    pub(crate) fn from_parts(
        binding: AssetCatalogBinding,
        classes: Vec<AssetClassCoverage>,
        assets: BTreeMap<String, AssetEntry>,
        renditions: BTreeMap<String, AssetRenditionPayload>,
    ) -> Self {
        Self {
            binding,
            classes,
            assets,
            renditions,
        }
    }

    /// Returns the manifest, locale, content revision, generation and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &AssetCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns the documented freshness of this read.
    #[must_use]
    pub fn revision(&self) -> AssetRevision {
        self.binding.revision()
    }

    /// Returns per-class support state, including unsupported and unavailable classes.
    #[must_use]
    pub fn classes(&self) -> &[AssetClassCoverage] {
        &self.classes
    }

    /// Returns the declared state of one media class.
    #[must_use]
    pub fn class_state(&self, class: AssetMediaClass) -> Option<&AssetClassCoverage> {
        self.classes.iter().find(|row| row.class == class)
    }

    /// Returns an immutable catalog reader with an independent bounded cursor registry.
    #[must_use]
    pub fn reader(&self) -> AssetCatalogReader {
        AssetCatalogReader::new(self.clone())
    }

    /// Performs one exact metadata lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &AssetEntryReference,
        scope: AssetVisibilityScope,
        revision: Option<&AssetRevision>,
    ) -> Result<AssetEntry, AssetReferenceError> {
        self.reader().get(&AssetDetailQuery {
            entry: reference.clone(),
            scope,
            revision: revision.cloned(),
        })
    }

    /// Lists one complete page of visible assets from a fresh reader.
    ///
    /// This convenience holds no cursor registry, so it cannot serve a page the matching assets do
    /// not fit into: a partial result is refused rather than returned with a continuation no reader
    /// can consume.  Hold the reader from [`AssetCatalog::reader`] to walk every page.
    pub fn list(&self, query: &AssetListQuery) -> Result<AssetPage, AssetReferenceError> {
        let page = self.reader().list(query)?;
        if page.is_partial() {
            return Err(AssetReferenceError::PartialPageRequiresReader);
        }
        Ok(page)
    }
}
