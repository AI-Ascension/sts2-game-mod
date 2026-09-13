// SPDX-License-Identifier: MIT

//! Source-only power and status definitions plus coherent live instances.
//!
//! The module keeps localized static rules separate from mutable instances.  A live instance is
//! always bound to one catalog, run, snapshot, and monotonic epoch.  These are owner-local Rust
//! values only: they are not a native extractor, transport schema, gateway capability, or wire
//! contract.

mod catalog;
mod catalog_reader;
mod definition;
mod definition_validation;
mod error;
mod live;
mod live_model;
mod live_validation;
mod model;

pub use catalog::{
    PowerStatusCatalogProducer, PowerStatusCatalogSnapshot, PowerStatusCatalogSource,
};
pub use catalog_reader::{
    PowerStatusCatalog, PowerStatusCatalogReader, PowerStatusContinuation,
    PowerStatusDefinitionPage, PowerStatusDefinitionSummary, PowerStatusListQuery,
};
pub use definition::{
    PowerStatusAmountDefinition, PowerStatusCap, PowerStatusCounterDefinition,
    PowerStatusDecayDefinition, PowerStatusDecayRule, PowerStatusDefinition,
    PowerStatusDefinitionInput, PowerStatusDurationDefinition, PowerStatusDurationRule,
    PowerStatusFamilyCoverage, PowerStatusFamilyState, PowerStatusReferenceKind,
    PowerStatusSemanticReference, PowerStatusStackingDefinition, PowerStatusStackingPolicy,
};
pub use error::{PowerStatusCatalogError, PowerStatusLiveError, PowerStatusSourceError};
pub use live::{
    PowerStatusInstance, PowerStatusInstanceReference, PowerStatusLiveReader,
    PowerStatusLiveSnapshot, PowerStatusLiveSource,
};
pub use live_model::{
    PowerStatusAmount, PowerStatusCounterValue, PowerStatusDurationState, PowerStatusInstanceInput,
    PowerStatusLiveSnapshotInput, PowerStatusOwner, PowerStatusOwnerKind, PowerStatusPendingExpiry,
    PowerStatusSourceKind, PowerStatusSourceReference,
};
pub use model::{
    POWER_STATUS_ENTITY_KIND, POWER_STATUS_MAX_CAP_REFERENCES, POWER_STATUS_MAX_COUNTERS,
    POWER_STATUS_MAX_DEFINITION_BYTES, POWER_STATUS_MAX_DEFINITIONS,
    POWER_STATUS_MAX_IDENTITY_BYTES, POWER_STATUS_MAX_INSTANCES,
    POWER_STATUS_MAX_LIVE_DETAIL_BYTES, POWER_STATUS_MAX_PAGE_ITEMS,
    POWER_STATUS_MAX_REFERENCE_BYTES, POWER_STATUS_MAX_REFERENCES, POWER_STATUS_MAX_TEXT_BYTES,
    POWER_STATUS_PRODUCER_VERSION, PowerStatusCatalogBinding, PowerStatusCategory,
    PowerStatusDefinitionReference, PowerStatusField, PowerStatusFieldStatus, PowerStatusKind,
    PowerStatusLiveBinding, PowerStatusOrigin, PowerStatusOwnerId, PowerStatusReset,
    PowerStatusUnit, PowerStatusVisibility, PowerStatusVisibilityScope,
};
