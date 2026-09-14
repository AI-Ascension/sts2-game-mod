// SPDX-License-Identifier: MIT

use super::super::{
    ActDefinition, ActEncounterReference, ActField, ActPoolReference, ActReferenceError,
    ActRoomCategoryReference, ActUnavailableReason, ActVisibilityScope, EncounterDefinition,
    EncounterPool, EncounterPossibility, MapGenerationConstraint, RoomCategoryDefinition,
};
use super::reader::ActCatalogReader;
use super::{visible_encounter, visible_pool, visible_room_category};

impl ActCatalogReader {
    /// Performs one exact act lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &super::super::ActDefinitionReference,
        scope: ActVisibilityScope,
    ) -> Result<ActDefinition, ActReferenceError> {
        self.validate_family()?;
        self.definition_for_reference(reference, scope).cloned()
    }

    /// Performs one exact encounter lookup under an explicit visibility scope.
    pub fn get_encounter(
        &self,
        reference: &ActEncounterReference,
        scope: ActVisibilityScope,
    ) -> Result<EncounterDefinition, ActReferenceError> {
        self.validate_family()?;
        let definition =
            self.definition_for_identity(&reference.catalog, &reference.act_id, scope)?;
        find_encounter(definition, &reference.encounter_id, scope)
    }

    /// Performs one exact room/node category lookup.
    pub fn get_room_category(
        &self,
        reference: &ActRoomCategoryReference,
        scope: ActVisibilityScope,
    ) -> Result<RoomCategoryDefinition, ActReferenceError> {
        self.validate_family()?;
        let definition =
            self.definition_for_identity(&reference.catalog, &reference.act_id, scope)?;
        if let Some(category) = definition
            .room_categories
            .iter()
            .find(|category| category.category_id == reference.category_id)
        {
            if visible_room_category(category, scope) {
                return Ok(category.clone());
            }
            return Err(ActReferenceError::ExcludedByScope);
        }
        Err(ActReferenceError::NotFound)
    }

    /// Performs one exact encounter-pool lookup.
    pub fn get_pool(
        &self,
        reference: &ActPoolReference,
        scope: ActVisibilityScope,
    ) -> Result<EncounterPool, ActReferenceError> {
        self.validate_family()?;
        let definition =
            self.definition_for_identity(&reference.catalog, &reference.act_id, scope)?;
        match &definition.pools {
            ActField::Available(pools) => {
                if let Some(pool) = pools.iter().find(|pool| pool.pool_id == reference.pool_id) {
                    if visible_pool(pool, scope) {
                        return Ok(pool.clone());
                    }
                    return Err(ActReferenceError::ExcludedByScope);
                }
                Err(ActReferenceError::NotFound)
            }
            ActField::Unavailable(reason) => Err(ActReferenceError::UnavailableField(*reason)),
        }
    }

    /// Performs one exact map-generation constraint lookup.
    pub fn get_constraint(
        &self,
        act: &super::super::ActDefinitionReference,
        constraint_id: &str,
        scope: ActVisibilityScope,
    ) -> Result<MapGenerationConstraint, ActReferenceError> {
        self.validate_family()?;
        let definition = self.definition_for_reference(act, scope)?;
        match &definition.constraints {
            ActField::Available(constraints) => {
                if let Some(constraint) = constraints
                    .iter()
                    .find(|constraint| constraint.constraint_id == constraint_id)
                {
                    if super::visible_constraint(constraint, scope) {
                        return Ok(constraint.clone());
                    }
                    return Err(ActReferenceError::ExcludedByScope);
                }
                Err(ActReferenceError::NotFound)
            }
            ActField::Unavailable(reason) => Err(ActReferenceError::UnavailableField(*reason)),
        }
    }

    /// Resolves reference possibilities for one room/node category.
    ///
    /// This never returns a seed-specific assignment; an assignment that cannot be safely
    /// revealed is withheld with an explicit reason.
    pub fn possibility_for_category(
        &self,
        act: &super::super::ActDefinitionReference,
        category_id: &str,
        scope: ActVisibilityScope,
    ) -> Result<EncounterPossibility, ActReferenceError> {
        self.validate_family()?;
        let definition = self.definition_for_reference(act, scope)?;
        match &definition.pools {
            ActField::Available(pools) => {
                let matching = pools
                    .iter()
                    .filter(|pool| visible_pool(pool, scope))
                    .find(|pool| pool.room_category_id.as_deref() == Some(category_id));
                if let Some(pool) = matching {
                    return Ok(EncounterPossibility::Pool(pool_reference(definition, pool)));
                }
                Ok(EncounterPossibility::Unavailable(
                    ActUnavailableReason::NotApplicable,
                ))
            }
            ActField::Unavailable(reason) => Ok(match reason {
                ActUnavailableReason::Denied
                | ActUnavailableReason::Unsupported
                | ActUnavailableReason::Unknown => EncounterPossibility::Withheld(*reason),
                _ => EncounterPossibility::Unavailable(*reason),
            }),
        }
    }
}

fn find_encounter(
    definition: &ActDefinition,
    encounter_id: &str,
    scope: ActVisibilityScope,
) -> Result<EncounterDefinition, ActReferenceError> {
    if let Some(encounter) = definition
        .encounters
        .iter()
        .find(|encounter| encounter.reference.encounter_id == encounter_id)
    {
        if visible_encounter(encounter, scope) {
            return Ok(encounter.clone());
        }
        return Err(ActReferenceError::ExcludedByScope);
    }
    Err(ActReferenceError::NotFound)
}

fn pool_reference(definition: &ActDefinition, pool: &EncounterPool) -> ActPoolReference {
    ActPoolReference {
        catalog: definition.reference.catalog.clone(),
        act_id: definition.reference.act_id.clone(),
        pool_id: pool.pool_id.clone(),
    }
}
