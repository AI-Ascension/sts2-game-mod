// SPDX-License-Identifier: MIT

//! Owner-local potion definitions and live-instance semantics.
//!
//! This module is deliberately source-only. It binds static potion metadata to an immutable
//! content manifest and keeps live instance values behind an independent coherent snapshot.
//! The types are not a wire contract and do not claim native-host coverage. In particular,
//! effect alternatives are retained as structure; this module never samples game randomness or
//! predicts target-specific outcomes.

mod catalog;
mod catalog_reader;
mod definition;
mod definition_validation;
mod error;
mod live;
mod live_model;
mod live_sizes;
mod live_validation;
mod model;

pub use catalog::{PotionCatalogProducer, PotionCatalogSnapshot, PotionCatalogSource};
pub use catalog_reader::{
    PotionCatalog, PotionCatalogReader, PotionContinuation, PotionDefinitionPage,
    PotionDefinitionSummary, PotionListQuery,
};
pub use definition::{
    PotionAcquisition, PotionAcquisitionRule, PotionCondition, PotionDefinition,
    PotionDefinitionInput, PotionEffect, PotionEffectAlternative, PotionEffectKind,
    PotionEffectMagnitude, PotionFamilyCoverage, PotionFamilyState, PotionParameterDefinition,
    PotionParameterValue, PotionResolvedParameter, PotionSemanticReference,
    PotionSemanticReferenceKind, PotionUnlock,
};
pub use error::{PotionCatalogError, PotionLiveError, PotionSourceError};
pub use live::{
    PotionInstance, PotionInstanceReference, PotionLiveReader, PotionLiveSnapshot,
    PotionLiveSource, PotionOffer,
};
pub use live_model::{
    PotionExpiration, PotionInstanceInput, PotionInventory, PotionInventoryInput,
    PotionLiveSnapshotInput, PotionModifier, PotionModifierScope, PotionModifierValue,
    PotionOfferInput, PotionOfferKind, PotionPrice, PotionSlotState, PotionTargetKind,
    PotionTargetReference, PotionUsabilityReason, PotionUsabilityResult, PotionUseState,
};
pub use model::{
    POTION_ENTITY_KIND, POTION_MAX_ACQUISITION_RULES, POTION_MAX_ALTERNATIVES,
    POTION_MAX_DEFINITION_BYTES, POTION_MAX_EFFECTS, POTION_MAX_IDENTITY_BYTES,
    POTION_MAX_INSTANCES, POTION_MAX_LIVE_DETAIL_BYTES, POTION_MAX_MODIFIERS, POTION_MAX_OFFERS,
    POTION_MAX_PAGE_ITEMS, POTION_MAX_PARAMETERS, POTION_MAX_REFERENCES, POTION_MAX_SLOTS,
    POTION_MAX_TARGETS, POTION_MAX_TEXT_BYTES, POTION_PRODUCER_VERSION, PotionCatalogBinding,
    PotionCollectionKind, PotionDefinitionReference, PotionField, PotionFieldStatus,
    PotionLiveBinding, PotionOrigin, PotionOwnerId, PotionPool, PotionRarity, PotionSlotReference,
    PotionTargetMode, PotionUnit, PotionUseRule, PotionVisibility, PotionVisibilityScope,
};
