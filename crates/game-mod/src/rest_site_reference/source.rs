// SPDX-License-Identifier: MIT

use std::cell::Cell;

use crate::ContentManifest;

use super::{
    catalog::{RestSiteCatalogSnapshot, RestSiteCatalogSource},
    error::RestSourceError,
};

/// Deterministic owner-local source fixture used by owner-side tests and probes.
///
/// The fixture counts served reads so a test can prove that producing a catalog, listing sites and
/// options, and looking up definitions never mutates the source or performs a hidden extra read.
#[derive(Debug)]
pub struct FixtureRestSiteSource {
    snapshot: RestSiteCatalogSnapshot,
    reads: Cell<usize>,
}

impl FixtureRestSiteSource {
    /// Creates a deterministic source that copies one owned snapshot.
    #[must_use]
    pub fn new(snapshot: RestSiteCatalogSnapshot) -> Self {
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
    pub const fn snapshot(&self) -> &RestSiteCatalogSnapshot {
        &self.snapshot
    }
}

impl RestSiteCatalogSource for FixtureRestSiteSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RestSiteCatalogSnapshot, RestSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

/// One sanitized failure the failing fixture source can report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureRestFailure {
    /// No supported rest-site registry is active.
    NoActiveSource,
    /// The source denied the read.
    AccessDenied,
    /// The source returned malformed data.
    Malformed,
}

/// Failing source that exposes only a sanitized failure and no host details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailingRestSiteSource(pub FixtureRestFailure);

impl RestSiteCatalogSource for FailingRestSiteSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RestSiteCatalogSnapshot, RestSourceError> {
        Err(match self.0 {
            FixtureRestFailure::NoActiveSource => RestSourceError::NoActiveSource,
            FixtureRestFailure::AccessDenied => RestSourceError::AccessDenied,
            FixtureRestFailure::Malformed => RestSourceError::Malformed,
        })
    }
}
