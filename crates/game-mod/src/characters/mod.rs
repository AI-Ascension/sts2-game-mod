// SPDX-License-Identifier: MIT

//! Source-only playable-character reference data.
//!
//! This module copies character definitions, starting configurations, pool references, and
//! unlock requirements into an immutable catalog fenced by the existing content manifest and
//! locale. It does not construct a run, select a character, mutate profile progress, define a
//! transport schema, or claim native-host compatibility.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{CharacterCatalogProducer, CharacterCatalogSnapshot, CharacterCatalogSource};
pub use catalog_reader::{
    CharacterCatalog, CharacterCatalogReader, CharacterContinuation, CharacterDefinitionPage,
    CharacterDefinitionSummary, CharacterListQuery,
};
pub use definition::{
    CharacterDefinition, CharacterDefinitionInput, CharacterFamilyCoverage, CharacterFamilyState,
    CharacterLoadout, CharacterLoadoutAvailability, CharacterLoadoutInput,
    CharacterLoadoutRequirement, CharacterMechanicReference, CharacterPoolReference,
    CharacterResource, CharacterStartingConfiguration, CharacterUnlock, CharacterUnlockRequirement,
};
pub use error::{CharacterCatalogError, CharacterSourceError};
pub use model::{
    CHARACTER_ENTITY_KIND, CHARACTER_MAX_DEFINITION_BYTES, CHARACTER_MAX_DEFINITIONS,
    CHARACTER_MAX_FORMULA_INPUTS, CHARACTER_MAX_IDENTITY_BYTES, CHARACTER_MAX_LOADOUTS,
    CHARACTER_MAX_MECHANIC_REFERENCES, CHARACTER_MAX_PAGE_ITEMS, CHARACTER_MAX_POOLS,
    CHARACTER_MAX_REFERENCES, CHARACTER_MAX_REQUIREMENTS, CHARACTER_MAX_RESOURCES,
    CHARACTER_MAX_TEXT_BYTES, CHARACTER_PRODUCER_VERSION, CharacterCatalogBinding,
    CharacterContentReference, CharacterDefinitionReference, CharacterField, CharacterFieldStatus,
    CharacterFormula, CharacterNumericValue, CharacterOrigin, CharacterText,
    CharacterUnavailableReason, CharacterVisibilityScope,
};
