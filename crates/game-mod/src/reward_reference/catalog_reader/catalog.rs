// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    RewardCatalogBinding, RewardCatalogError, RewardDefinitionReference, RewardFamilyCoverage,
    RewardItem, RewardItemReference, RewardOfferDefinition, RewardVisibilityScope,
};
use super::reader::RewardCatalogReader;

/// Immutable reward offer definitions keyed by namespaced reward identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardCatalog {
    pub(super) binding: RewardCatalogBinding,
    pub(super) family: RewardFamilyCoverage,
    pub(super) definitions: BTreeMap<String, RewardOfferDefinition>,
}

impl RewardCatalog {
    pub(crate) fn from_parts(
        binding: RewardCatalogBinding,
        family: RewardFamilyCoverage,
        definitions: BTreeMap<String, RewardOfferDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &RewardCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the reward family.
    #[must_use]
    pub fn family(&self) -> &RewardFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> RewardCatalogReader {
        RewardCatalogReader::new(self.clone())
    }

    /// Performs one exact reward definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RewardDefinitionReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardOfferDefinition, RewardCatalogError> {
        self.reader().get(reference, scope)
    }

    /// Performs one exact offered-item lookup under an explicit visibility scope.
    pub fn get_item(
        &self,
        reference: &RewardItemReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardItem, RewardCatalogError> {
        self.reader().get_item(reference, scope)
    }
}
