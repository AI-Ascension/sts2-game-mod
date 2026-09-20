// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    RunResultCatalogBinding, RunResultError, RunResultFamilyCoverage, RunResultLiveFence,
    RunResultReadAuthority, RunResultRecord, RunResultReference, RunResultVisibilityScope,
    RunSummaryRecord, RunSummaryReference,
};
use super::reader::RunResultCatalogReader;

/// Immutable completed-run results and prior summaries bound to one content manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunResultCatalog {
    pub(super) binding: RunResultCatalogBinding,
    pub(super) family: RunResultFamilyCoverage,
    pub(super) results: BTreeMap<String, RunResultRecord>,
    pub(super) summaries: BTreeMap<String, RunSummaryRecord>,
}

impl RunResultCatalog {
    pub(crate) fn from_parts(
        binding: RunResultCatalogBinding,
        family: RunResultFamilyCoverage,
        results: BTreeMap<String, RunResultRecord>,
        summaries: BTreeMap<String, RunSummaryRecord>,
    ) -> Self {
        Self {
            binding,
            family,
            results,
            summaries,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &RunResultCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the result family.
    #[must_use]
    pub fn family(&self) -> &RunResultFamilyCoverage {
        &self.family
    }

    /// Returns the capability every published read withholds.
    ///
    /// Reading a completed run is not authority to select a profile, load a save or start a run,
    /// so the boundary states the capability it never grants.
    #[must_use]
    pub const fn authority(&self) -> RunResultReadAuthority {
        RunResultReadAuthority::NotGranted
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> RunResultCatalogReader {
        RunResultCatalogReader::new(self.clone())
    }

    /// Reads the fenced current terminal result.
    pub fn current(
        &self,
        fence: &RunResultLiveFence,
        scope: RunResultVisibilityScope,
    ) -> Result<RunResultRecord, RunResultError> {
        self.reader().current(fence, scope)
    }

    /// Performs one exact retained-result lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RunResultReference,
        scope: RunResultVisibilityScope,
    ) -> Result<RunResultRecord, RunResultError> {
        self.reader().get(reference, scope)
    }

    /// Performs one exact prior-summary lookup under an explicit visibility scope.
    pub fn get_summary(
        &self,
        reference: &RunSummaryReference,
        scope: RunResultVisibilityScope,
    ) -> Result<RunSummaryRecord, RunResultError> {
        self.reader().get_summary(reference, scope)
    }
}
