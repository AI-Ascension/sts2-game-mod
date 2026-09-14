// SPDX-License-Identifier: MIT

//! Source-only act, encounter, and map-generation reference data.
//!
//! This module copies act structures, room/node categories, normal/elite/boss encounter
//! definitions, eligibility predicates, weighted encounter pools, and map-generation
//! constraint/rule references into an immutable catalog fenced by the existing content manifest
//! and locale.  Static reference possibilities stay distinct from a seed-specific unrevealed
//! assignment, and definition identities stay distinct from live map-node identities.  No live
//! map read, seed assignment, transport route, native extractor, or host compatibility is
//! claimed here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{ActCatalogProducer, ActCatalogSnapshot, ActCatalogSource};
pub use catalog_reader::{
    ActCatalog, ActCatalogReader, ActContinuation, ActDefinitionPage, ActDefinitionSummary,
    ActEncounterContinuation, ActEncounterDefinitionPage, ActEncounterListQuery,
    ActEncounterSummary, ActListQuery,
};
pub use definition::{
    ActDefinition, ActDefinitionInput, ActParameter, EligibilityCondition, EligibilityKind,
    EncounterDefinition, EncounterDefinitionInput, EncounterEnemy, EncounterEnemyGroup,
    EncounterKind, EncounterPool, EncounterPoolEntry, EncounterPoolKind, MapConstraintKind,
    MapGenerationConstraint, RoomCategoryDefinition, RoomCategoryKind,
};
pub use error::{ActReferenceError, ActSourceError};
pub use model::{
    ACT_MAX_CONSTRAINTS, ACT_MAX_DEFINITION_BYTES, ACT_MAX_DEFINITIONS, ACT_MAX_ELIGIBILITY,
    ACT_MAX_ENCOUNTERS, ACT_MAX_ENEMY_GROUPS, ACT_MAX_FORMULA_INPUTS, ACT_MAX_GROUP_ENEMIES,
    ACT_MAX_IDENTITY_BYTES, ACT_MAX_PAGE_ITEMS, ACT_MAX_PARAMETERS, ACT_MAX_POOL_ENTRIES,
    ACT_MAX_POOLS, ACT_MAX_REFERENCES, ACT_MAX_ROOM_CATEGORIES, ACT_MAX_TEXT_BYTES,
    ACT_MAX_VARIANTS, ACT_REFERENCE_ENCOUNTER_KIND, ACT_REFERENCE_ENEMY_KIND,
    ACT_REFERENCE_ENTITY_KIND, ACT_REFERENCE_PRODUCER_VERSION, ActCatalogBinding,
    ActDefinitionReference, ActEncounterReference, ActEvidence, ActFamilyCoverage, ActFamilyState,
    ActField, ActFieldStatus, ActFormula, ActMapNodeReference, ActNumericValue, ActPoolReference,
    ActRoomCategoryReference, ActSemanticReference, ActSemanticReferenceKind, ActText,
    ActUnavailableReason, ActVisibility, ActVisibilityScope, EncounterPossibility,
    GenerationWeight,
};
