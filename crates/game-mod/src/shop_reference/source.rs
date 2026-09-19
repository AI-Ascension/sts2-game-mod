// SPDX-License-Identifier: MIT

use std::cell::Cell;

use crate::ContentManifest;

use super::{
    catalog::{ShopCatalogSnapshot, ShopCatalogSource},
    error::ShopSourceError,
};

/// Deterministic owner-local source fixture used by owner-side tests and probes.
///
/// The fixture counts served reads so a test can prove that producing a catalog, listing shops and
/// entries, and looking up definitions never mutates the source or performs a hidden extra read.
#[derive(Debug)]
pub struct FixtureShopSource {
    snapshot: ShopCatalogSnapshot,
    reads: Cell<usize>,
}

impl FixtureShopSource {
    /// Creates a deterministic source that copies one owned snapshot.
    #[must_use]
    pub fn new(snapshot: ShopCatalogSnapshot) -> Self {
        Self {
            snapshot,
            reads: Cell::new(0),
        }
    }

    /// Returns the number of catalog reads this fixture served.
    #[must_use]
    pub fn reads(&self) -> usize {
        self.reads.get()
    }

    /// Returns the retained snapshot without consuming it or counting a read.
    #[must_use]
    pub fn snapshot(&self) -> &ShopCatalogSnapshot {
        &self.snapshot
    }
}

impl ShopCatalogSource for FixtureShopSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ShopCatalogSnapshot, ShopSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

/// One sanitized failure the failing fixture source can report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureShopFailure {
    /// No supported shop registry is active.
    NoActiveSource,
    /// The source denied the read.
    AccessDenied,
    /// The source returned malformed data.
    Malformed,
}

/// Failing source that exposes only a sanitized failure and no host details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailingShopSource(pub FixtureShopFailure);

impl ShopCatalogSource for FailingShopSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ShopCatalogSnapshot, ShopSourceError> {
        Err(match self.0 {
            FixtureShopFailure::NoActiveSource => ShopSourceError::NoActiveSource,
            FixtureShopFailure::AccessDenied => ShopSourceError::AccessDenied,
            FixtureShopFailure::Malformed => ShopSourceError::Malformed,
        })
    }
}
