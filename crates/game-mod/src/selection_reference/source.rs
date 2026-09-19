// SPDX-License-Identifier: MIT

use std::cell::Cell;

use crate::ContentManifest;

use super::{
    catalog::{SelectionCatalogSnapshot, SelectionCatalogSource},
    error::SelectionSourceError,
};

/// Deterministic owner-local source fixture used by owner-side tests and probes.
///
/// The fixture counts served reads so a test can prove that producing a catalog, paging selectors
/// and candidates, and reconciling picks never mutates the source or performs a hidden extra read.
#[derive(Debug)]
pub struct FixtureSelectionSource {
    snapshot: SelectionCatalogSnapshot,
    reads: Cell<usize>,
}

impl FixtureSelectionSource {
    /// Creates a deterministic source that copies one owned snapshot.
    #[must_use]
    pub fn new(snapshot: SelectionCatalogSnapshot) -> Self {
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
    pub const fn snapshot(&self) -> &SelectionCatalogSnapshot {
        &self.snapshot
    }
}

impl SelectionCatalogSource for FixtureSelectionSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<SelectionCatalogSnapshot, SelectionSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

/// One sanitized failure the failing fixture source can report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureSelectionFailure {
    /// No supported selector registry is active.
    NoActiveSource,
    /// The source denied the read.
    AccessDenied,
    /// The source returned malformed data.
    Malformed,
}

/// Failing source that exposes only a sanitized failure and no host details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailingSelectionSource(pub FixtureSelectionFailure);

impl SelectionCatalogSource for FailingSelectionSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<SelectionCatalogSnapshot, SelectionSourceError> {
        Err(match self.0 {
            FixtureSelectionFailure::NoActiveSource => SelectionSourceError::NoActiveSource,
            FixtureSelectionFailure::AccessDenied => SelectionSourceError::AccessDenied,
            FixtureSelectionFailure::Malformed => SelectionSourceError::Malformed,
        })
    }
}
