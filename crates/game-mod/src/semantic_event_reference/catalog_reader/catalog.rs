// SPDX-License-Identifier: MIT

use super::super::{
    SemanticCatalogBinding, SemanticEventBatch, SemanticEventError, SemanticEventListQuery,
    SemanticEventPage, SemanticEventReader, SemanticEventReference, SemanticFamilyCoverage,
    SemanticHistoryAuthority, SemanticHistoryFence, SemanticHistoryScope, SemanticHistoryView,
};

/// Immutable semantic gameplay history bound to one content manifest and producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticHistoryCatalog {
    pub(super) binding: SemanticCatalogBinding,
    pub(super) family: SemanticFamilyCoverage,
    pub(super) batch: Option<SemanticEventBatch>,
}

impl SemanticHistoryCatalog {
    pub(crate) fn from_parts(
        binding: SemanticCatalogBinding,
        family: SemanticFamilyCoverage,
        batch: Option<SemanticEventBatch>,
    ) -> Self {
        Self {
            binding,
            family,
            batch,
        }
    }

    /// Returns the manifest and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &SemanticCatalogBinding {
        &self.binding
    }

    /// Returns explicit support coverage for the history family.
    #[must_use]
    pub fn family(&self) -> &SemanticFamilyCoverage {
        &self.family
    }

    /// Returns the capability every published read withholds.
    ///
    /// Observing what happened is not authority to replay it, to rewind to it or to act in the run,
    /// so the boundary states the capability it never grants.
    #[must_use]
    pub const fn authority(&self) -> SemanticHistoryAuthority {
        SemanticHistoryAuthority::NotGranted
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> SemanticEventReader {
        SemanticEventReader::new(self.clone())
    }

    /// Reads the retained history at its own scope.
    pub fn history(&self) -> Result<SemanticHistoryView, SemanticEventError> {
        self.reader().history()
    }

    /// Reads the fenced live history.
    pub fn current(
        &self,
        fence: &SemanticHistoryFence,
        scope: SemanticHistoryScope,
    ) -> Result<SemanticHistoryView, SemanticEventError> {
        self.reader().current(fence, scope)
    }

    /// Reads one event by exact reference.
    pub fn event(
        &self,
        reference: &SemanticEventReference,
        scope: SemanticHistoryScope,
    ) -> Result<SemanticHistoryView, SemanticEventError> {
        self.reader().event(reference, scope)
    }

    /// Lists the events one read scope may observe, one bounded page at a time.
    pub fn list_events(
        &self,
        query: &SemanticEventListQuery,
    ) -> Result<SemanticEventPage, SemanticEventError> {
        self.reader().list_events(query)
    }
}
