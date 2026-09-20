// SPDX-License-Identifier: MIT

use super::super::{
    CoopCatalogBinding, CoopError, CoopFamilyCoverage, CoopLiveFence, CoopPartyRecord,
    CoopPartyView, CoopPeerListQuery, CoopPeerPage, CoopPeerReference, CoopPeerView,
    CoopReadAuthority, CoopReadScope,
};
use super::reader::CoopCatalogReader;

/// Immutable co-op party bound to one content manifest, locale and producer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopCatalog {
    pub(super) binding: CoopCatalogBinding,
    pub(super) family: CoopFamilyCoverage,
    pub(super) party: Option<CoopPartyRecord>,
}

impl CoopCatalog {
    pub(crate) fn from_parts(
        binding: CoopCatalogBinding,
        family: CoopFamilyCoverage,
        party: Option<CoopPartyRecord>,
    ) -> Self {
        Self {
            binding,
            family,
            party,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &CoopCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the party family.
    #[must_use]
    pub fn family(&self) -> &CoopFamilyCoverage {
        &self.family
    }

    /// Returns the capability every published read withholds.
    ///
    /// Observing a party is not authority to act in it, to select the single-player save profile or
    /// to load a save, so the boundary states the capability it never grants.
    #[must_use]
    pub const fn authority(&self) -> CoopReadAuthority {
        CoopReadAuthority::NotGranted
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> CoopCatalogReader {
        CoopCatalogReader::new(self.clone())
    }

    /// Reads the retained party record.
    pub fn party(&self) -> Result<CoopPartyRecord, CoopError> {
        self.reader().party()
    }

    /// Reads the fenced live party.
    pub fn current(
        &self,
        fence: &CoopLiveFence,
        scope: CoopReadScope,
    ) -> Result<CoopPartyView, CoopError> {
        self.reader().current(fence, scope)
    }

    /// Projects one member under an explicit read scope.
    pub fn peer(
        &self,
        reference: &CoopPeerReference,
        scope: CoopReadScope,
    ) -> Result<CoopPeerView, CoopError> {
        self.reader().peer(reference, scope)
    }

    /// Lists the members one read scope may observe, one bounded page at a time.
    pub fn list_peers(&self, query: &CoopPeerListQuery) -> Result<CoopPeerPage, CoopError> {
        self.reader().list_peers(query)
    }
}
