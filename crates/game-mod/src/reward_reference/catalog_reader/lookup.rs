// SPDX-License-Identifier: MIT

use super::super::{
    RewardCatalogError, RewardDefinitionReference, RewardItem, RewardItemReference,
    RewardOfferDefinition, RewardVisibilityScope,
};
use super::project_definition;
use super::reader::RewardCatalogReader;

impl RewardCatalogReader {
    /// Performs one exact reward definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RewardDefinitionReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardOfferDefinition, RewardCatalogError> {
        self.validate_family()?;
        let definition = self.definition_for_reference(reference, scope)?;
        Ok(project_definition(definition, scope))
    }

    /// Performs one exact offered-item lookup under an explicit visibility scope.
    pub fn get_item(
        &self,
        reference: &RewardItemReference,
        scope: RewardVisibilityScope,
    ) -> Result<RewardItem, RewardCatalogError> {
        self.validate_family()?;
        Ok(self.item_for_reference(reference, scope)?.clone())
    }
}
