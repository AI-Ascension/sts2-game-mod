// SPDX-License-Identifier: MIT

//! Source-only character resources and controlled secondary entities.
//!
//! This owner-local projection keeps static mechanic definitions separate from coherent live
//! values.  Every live record is fenced by content manifest, game instance, run, mode, snapshot,
//! epoch, and visibility scope.  It does not define a transport route, native ABI, or exact-host
//! extractor.

mod catalog;
mod catalog_validation;
mod definition;
mod error;
mod live;
mod live_detail;
mod live_measure;
mod live_model;
mod live_shape;
mod live_snapshot;
mod live_validation;
mod model;
mod reader;
mod sizes;
mod validation;

pub use catalog::{
    CharacterStateCatalog, CharacterStateCatalogProducer, CharacterStateCatalogSnapshot,
    CharacterStateCatalogSource,
};
pub use definition::{
    CharacterResourceDefinition, CharacterResourceDefinitionInput,
    CharacterResourceDefinitionReference, CharacterResourceKind, CharacterResourceValue,
    CharacterResourceValueDefinition, SecondaryEntityDefinition, SecondaryEntityDefinitionInput,
    SecondaryEntityDefinitionReference, SecondaryEntityKind,
};
pub use error::{CharacterStateCatalogError, CharacterStateLiveError, CharacterStateSourceError};
pub use live::CharacterStateLiveReader;
pub use live_model::{
    CharacterResource, CharacterResourceInput, CharacterResourceSlot, CharacterResourceSlotContent,
    CharacterSecondaryEntity, SecondaryEntityControllerReference, SecondaryEntityInput,
    SecondaryEntityIntent, SecondaryEntityIntentKind, SecondaryEntityStatus, SecondaryEntityTarget,
};
pub use live_snapshot::{
    CharacterStateLiveSnapshot, CharacterStateLiveSnapshotInput, CharacterStateLiveSource,
};
pub use model::{
    CHARACTER_STATE_ENTITY_KIND, CHARACTER_STATE_MAX_COVERAGE,
    CHARACTER_STATE_MAX_DEFINITION_BYTES, CHARACTER_STATE_MAX_DEFINITIONS,
    CHARACTER_STATE_MAX_DETAIL_BYTES, CHARACTER_STATE_MAX_IDENTITY_BYTES,
    CHARACTER_STATE_MAX_INTENTS, CHARACTER_STATE_MAX_LIVE_RESOURCES,
    CHARACTER_STATE_MAX_LIVE_SECONDARY_ENTITIES, CHARACTER_STATE_MAX_PAGE_ITEMS,
    CHARACTER_STATE_MAX_SLOTS, CHARACTER_STATE_MAX_STATUSES, CHARACTER_STATE_MAX_TEXT_BYTES,
    CHARACTER_STATE_PRODUCER_VERSION, CharacterMechanicCoverage, CharacterMechanicState,
    CharacterResourceReference, CharacterSecondaryEntityReference, CharacterStateCatalogBinding,
    CharacterStateField, CharacterStateFieldStatus, CharacterStateLiveBinding, CharacterStateOwner,
    CharacterStateOwnerId, CharacterStateOwnerKind, CharacterStateUnit, CharacterStateVisibility,
    CharacterStateVisibilityScope,
};
pub use reader::{
    CharacterStateCatalogReader, CharacterStateContinuation, CharacterStateDefinitionKind,
    CharacterStateDefinitionPage, CharacterStateDefinitionSummary, CharacterStateListQuery,
};
