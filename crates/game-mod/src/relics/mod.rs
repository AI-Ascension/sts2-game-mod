// SPDX-License-Identifier: MIT

//! Owner-local relic definitions and live-instance semantics.
//!
//! This module is deliberately source-only. It binds static relic metadata to the existing
//! content manifest and keeps live instance values behind an independent coherent snapshot
//! witness. The types are not a wire contract and do not claim native-host coverage.

mod catalog;
mod catalog_reader;
mod definition;
mod definition_validation;
mod error;
mod live;
mod live_model;
mod live_validation;
mod model;

pub use catalog::{RelicCatalogProducer, RelicCatalogSnapshot, RelicCatalogSource};
pub use catalog_reader::{
    RelicCatalog, RelicCatalogReader, RelicContinuation, RelicDefinitionPage,
    RelicDefinitionSummary, RelicListQuery,
};
pub use definition::{
    RelicAcquisition, RelicAcquisitionRule, RelicActivationDefinition, RelicActivationKind,
    RelicCondition, RelicCounterDefinition, RelicCounterState, RelicDefinition,
    RelicDefinitionInput, RelicFamilyCoverage, RelicFamilyState, RelicParameterDefinition,
    RelicParameterValue, RelicResolvedParameter, RelicSemanticReference,
    RelicSemanticReferenceKind, RelicTriggerDefinition, RelicUnlock, RelicVariant,
    RelicVisibilityScope,
};
pub use error::{RelicCatalogError, RelicLiveError, RelicSourceError};
pub use live::{
    RelicInstance, RelicInstanceReference, RelicLiveReader, RelicLiveSnapshot, RelicLiveSource,
};
pub use live_model::{
    RelicAccumulatedValue, RelicActivationState, RelicInstanceInput, RelicLiveSnapshotInput,
    RelicPendingTrigger,
};
pub use model::{
    RELIC_ENTITY_KIND, RELIC_MAX_ACQUISITION_RULES, RELIC_MAX_COUNTERS, RELIC_MAX_DEFINITION_BYTES,
    RELIC_MAX_IDENTITY_BYTES, RELIC_MAX_INSTANCES, RELIC_MAX_LIVE_DETAIL_BYTES,
    RELIC_MAX_PAGE_ITEMS, RELIC_MAX_PARAMETERS, RELIC_MAX_PENDING_TRIGGERS, RELIC_MAX_REFERENCES,
    RELIC_MAX_TEXT_BYTES, RELIC_MAX_VARIANTS, RELIC_PRODUCER_VERSION, RelicCatalogBinding,
    RelicCounterReset, RelicDefinitionReference, RelicField, RelicFieldStatus, RelicLiveBinding,
    RelicOrigin, RelicOwnerId, RelicPool, RelicRarity, RelicTier, RelicUnit, RelicVisibility,
};
