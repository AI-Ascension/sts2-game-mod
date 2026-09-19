// SPDX-License-Identifier: MIT

use std::cell::Cell;

use crate::ContentManifest;

use super::{
    catalog::{RunConfigurationCatalogSnapshot, RunConfigurationCatalogSource},
    error::RunConfigurationSourceError,
};

/// Deterministic owner-local source fixture used by owner-side tests and probes.
///
/// The fixture counts served reads so a test can prove that producing a catalog, listing, and
/// looking up definitions never mutates the source or performs a hidden extra read.
#[derive(Debug)]
pub struct FixtureRunConfigurationSource {
    snapshot: RunConfigurationCatalogSnapshot,
    reads: Cell<usize>,
}

impl FixtureRunConfigurationSource {
    /// Creates a deterministic source that copies one owned snapshot.
    #[must_use]
    pub fn new(snapshot: RunConfigurationCatalogSnapshot) -> Self {
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
    pub fn snapshot(&self) -> &RunConfigurationCatalogSnapshot {
        &self.snapshot
    }
}

impl RunConfigurationCatalogSource for FixtureRunConfigurationSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RunConfigurationCatalogSnapshot, RunConfigurationSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

/// One sanitized failure the failing fixture source can report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FixtureRunConfigurationFailure {
    /// No supported run-configuration registry is active.
    NoActiveSource,
    /// The source denied the read.
    AccessDenied,
    /// The source returned malformed data.
    Malformed,
}

/// Failing source that exposes only a sanitized failure and no host details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailingRunConfigurationSource(pub FixtureRunConfigurationFailure);

impl RunConfigurationCatalogSource for FailingRunConfigurationSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RunConfigurationCatalogSnapshot, RunConfigurationSourceError> {
        Err(match self.0 {
            FixtureRunConfigurationFailure::NoActiveSource => {
                RunConfigurationSourceError::NoActiveSource
            }
            FixtureRunConfigurationFailure::AccessDenied => {
                RunConfigurationSourceError::AccessDenied
            }
            FixtureRunConfigurationFailure::Malformed => RunConfigurationSourceError::Malformed,
        })
    }
}
