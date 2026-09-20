// SPDX-License-Identifier: MIT

use std::cell::Cell;

use crate::ContentManifest;

use super::{
    catalog::{ActionCatalogSnapshot, ActionCatalogSource},
    error::ActionSourceError,
};

/// Deterministic owner-local source fixture used by owner-side tests and probes.
///
/// The fixture counts served reads so a test can prove that producing a catalog, paging actions and
/// targets, explaining availability, and previewing a target never mutates the source, never
/// consumes the RNG, and never performs a hidden extra read. Because the retained snapshot is
/// immutable and each read returns a clone of it, repeated previews of one frame are byte-identical
/// by construction: a caller cannot reconstruct a withheld future outcome by asking again.
#[derive(Debug)]
pub struct FixtureActionSource {
    snapshot: ActionCatalogSnapshot,
    reads: Cell<usize>,
}

impl FixtureActionSource {
    /// Creates a deterministic source that copies one owned snapshot.
    #[must_use]
    pub fn new(snapshot: ActionCatalogSnapshot) -> Self {
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
    pub const fn snapshot(&self) -> &ActionCatalogSnapshot {
        &self.snapshot
    }
}

impl ActionCatalogSource for FixtureActionSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ActionCatalogSnapshot, ActionSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

/// One sanitized failure the failing fixture source can report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureActionFailure {
    /// No supported legal-action registry is active.
    NoActiveSource,
    /// The source denied the read.
    AccessDenied,
    /// The source returned malformed data.
    Malformed,
}

/// Failing source that exposes only a sanitized failure and no host details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailingActionSource(pub FixtureActionFailure);

impl ActionCatalogSource for FailingActionSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ActionCatalogSnapshot, ActionSourceError> {
        Err(match self.0 {
            FixtureActionFailure::NoActiveSource => ActionSourceError::NoActiveSource,
            FixtureActionFailure::AccessDenied => ActionSourceError::AccessDenied,
            FixtureActionFailure::Malformed => ActionSourceError::Malformed,
        })
    }
}
