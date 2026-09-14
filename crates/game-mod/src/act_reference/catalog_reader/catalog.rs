// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    ActCatalogBinding, ActDefinition, ActDefinitionReference, ActEncounterReference,
    ActFamilyCoverage, ActPoolReference, ActReferenceError, ActRoomCategoryReference,
    ActVisibilityScope, EncounterDefinition, EncounterPool, EncounterPossibility,
    MapGenerationConstraint, RoomCategoryDefinition,
};
use super::reader::ActCatalogReader;

/// Immutable act definitions keyed by namespaced act identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActCatalog {
    pub(super) binding: ActCatalogBinding,
    pub(super) family: ActFamilyCoverage,
    pub(super) definitions: BTreeMap<String, ActDefinition>,
}

impl ActCatalog {
    pub(crate) fn from_parts(
        binding: ActCatalogBinding,
        family: ActFamilyCoverage,
        definitions: BTreeMap<String, ActDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &ActCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the act family.
    #[must_use]
    pub fn family(&self) -> &ActFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> ActCatalogReader {
        ActCatalogReader::new(self.clone())
    }

    /// Performs one exact act lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &ActDefinitionReference,
        scope: ActVisibilityScope,
    ) -> Result<ActDefinition, ActReferenceError> {
        self.reader().get(reference, scope)
    }

    /// Performs one exact encounter lookup under an explicit visibility scope.
    pub fn get_encounter(
        &self,
        reference: &ActEncounterReference,
        scope: ActVisibilityScope,
    ) -> Result<EncounterDefinition, ActReferenceError> {
        self.reader().get_encounter(reference, scope)
    }

    /// Performs one exact room/node category lookup.
    pub fn get_room_category(
        &self,
        reference: &ActRoomCategoryReference,
        scope: ActVisibilityScope,
    ) -> Result<RoomCategoryDefinition, ActReferenceError> {
        self.reader().get_room_category(reference, scope)
    }

    /// Performs one exact encounter-pool lookup.
    pub fn get_pool(
        &self,
        reference: &ActPoolReference,
        scope: ActVisibilityScope,
    ) -> Result<EncounterPool, ActReferenceError> {
        self.reader().get_pool(reference, scope)
    }

    /// Performs one exact map-generation constraint lookup.
    pub fn get_constraint(
        &self,
        act: &ActDefinitionReference,
        constraint_id: &str,
        scope: ActVisibilityScope,
    ) -> Result<MapGenerationConstraint, ActReferenceError> {
        self.reader().get_constraint(act, constraint_id, scope)
    }

    /// Resolves reference possibilities for one room/node category.
    pub fn possibility_for_category(
        &self,
        act: &ActDefinitionReference,
        category_id: &str,
        scope: ActVisibilityScope,
    ) -> Result<EncounterPossibility, ActReferenceError> {
        self.reader()
            .possibility_for_category(act, category_id, scope)
    }
}
