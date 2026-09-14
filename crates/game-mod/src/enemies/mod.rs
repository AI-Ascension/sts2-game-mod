// SPDX-License-Identifier: MIT

//! Source-only enemy, boss, elite, and minion reference data.
//!
//! This module copies enemy definitions and independently described behavior rules into an
//! immutable catalog fenced by the existing content manifest and locale. Move possibilities,
//! conditions, phases, cooldowns, repetition restrictions, and probabilities remain reference
//! data; no live chosen move, RNG cursor, combat instance, transport schema, or native-host
//! compatibility is claimed here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{EnemyCatalogProducer, EnemyCatalogSnapshot, EnemyCatalogSource};
pub use catalog_reader::{
    EnemyCatalog, EnemyCatalogReader, EnemyContinuation, EnemyDefinitionPage,
    EnemyDefinitionSummary, EnemyListQuery, EnemyMoveContinuation, EnemyMoveDefinitionPage,
    EnemyMoveListQuery, EnemyMoveSummary,
};
pub use definition::{
    EnemyBehaviorTransition, EnemyConditionReference, EnemyCooldownRule, EnemyDefinition,
    EnemyDefinitionInput, EnemyEncounterReference, EnemyFamilyCoverage, EnemyFamilyState,
    EnemyMoveDefinition, EnemyMoveDefinitionInput, EnemyMoveEffect, EnemyMoveEffectKind,
    EnemyOriginVariant, EnemyParameter, EnemyPhaseDefinition, EnemyRepetitionRule,
    EnemySemanticReference, EnemySemanticReferenceKind, EnemyStat, EnemyStatProfile, EnemyStats,
    EnemyTag,
};
pub use error::{EnemyCatalogError, EnemySourceError};
pub use model::{
    ENEMY_ENTITY_KIND, ENEMY_MAX_DEFINITION_BYTES, ENEMY_MAX_DEFINITIONS, ENEMY_MAX_EFFECTS,
    ENEMY_MAX_FORMULA_INPUTS, ENEMY_MAX_IDENTITY_BYTES, ENEMY_MAX_MOVES, ENEMY_MAX_ORIGIN_VARIANTS,
    ENEMY_MAX_PAGE_ITEMS, ENEMY_MAX_PARAMETERS, ENEMY_MAX_PHASES, ENEMY_MAX_REFERENCES,
    ENEMY_MAX_STAT_PROFILES, ENEMY_MAX_STATS, ENEMY_MAX_TAGS, ENEMY_MAX_TEXT_BYTES,
    ENEMY_PRODUCER_VERSION, EnemyCatalogBinding, EnemyDefinitionReference, EnemyEvidence,
    EnemyField, EnemyFieldStatus, EnemyFormula, EnemyKind, EnemyMoveReference, EnemyNumericValue,
    EnemyOrigin, EnemyProbability, EnemyResetBoundary, EnemyTargetDomain, EnemyTargeting,
    EnemyText, EnemyUnavailableReason, EnemyVisibility, EnemyVisibilityScope,
};
