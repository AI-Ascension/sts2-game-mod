// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    EnemyCatalogBinding, EnemyCatalogError, EnemyDefinition, EnemyDefinitionReference,
    EnemyFamilyCoverage, EnemyMoveDefinition, EnemyMoveReference, EnemyVisibilityScope,
};
use super::reader::EnemyCatalogReader;

/// Immutable enemy definitions keyed by namespaced enemy identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyCatalog {
    pub(super) binding: EnemyCatalogBinding,
    pub(super) family: EnemyFamilyCoverage,
    pub(super) definitions: BTreeMap<String, EnemyDefinition>,
}

impl EnemyCatalog {
    pub(crate) fn from_parts(
        binding: EnemyCatalogBinding,
        family: EnemyFamilyCoverage,
        definitions: BTreeMap<String, EnemyDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &EnemyCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the enemy family.
    #[must_use]
    pub fn family(&self) -> &EnemyFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> EnemyCatalogReader {
        EnemyCatalogReader::new(self.clone())
    }

    /// Performs one exact enemy lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &EnemyDefinitionReference,
        scope: EnemyVisibilityScope,
    ) -> Result<EnemyDefinition, EnemyCatalogError> {
        self.reader().get(reference, scope)
    }

    /// Performs one exact move lookup under an explicit visibility scope.
    pub fn get_move(
        &self,
        reference: &EnemyMoveReference,
        scope: EnemyVisibilityScope,
    ) -> Result<EnemyMoveDefinition, EnemyCatalogError> {
        self.reader().get_move(reference, scope)
    }
}
