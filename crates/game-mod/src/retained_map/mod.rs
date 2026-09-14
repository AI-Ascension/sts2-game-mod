// SPDX-License-Identifier: MIT

//! Source-only retained map knowledge observed while the map surface was permitted.
//!
//! This module retains only already-public topology that was copied from a permitted observation
//! and serves it after the map screen closes. It keeps retained knowledge distinct from current
//! navigation authority: freshness/provenance stay explicit, a retained read never claims to be
//! current or complete, hidden future room contents stay withheld, and travel references are
//! actionable only while the owning generation is current and the surface is open. Static
//! identity binds to the existing content-manifest cursor, locale, and an owner-local producer
//! version; live identity binds to instance, run, act, mode, map instance, snapshot, and a
//! monotonic epoch. This is an owner-local contract witness, not a native extractor, transport
//! route, or replacement for the `runtime-map-v1` topology, binding, or generation fences.

mod binding;
mod error;
mod field;
mod measure;
mod model;
mod page;
mod reader;
mod source;
mod validation;

pub use binding::{
    RetainedMapCatalogBinding, RetainedMapFreshness, RetainedMapLiveBinding,
    RetainedMapNodeVisibility, RetainedMapObservationState, RetainedMapProvenance,
    RetainedMapTravelActionability, RetainedMapVisibilityScope,
};
pub use error::{RetainedMapError, RetainedMapSourceError, RetainedMapUnavailableReason};
pub use field::{RetainedMapField, RetainedMapFieldStatus};
pub use model::{
    RETAINED_MAP_MAX_DETAIL_BYTES, RETAINED_MAP_MAX_EDGES, RETAINED_MAP_MAX_IDENTITY_BYTES,
    RETAINED_MAP_MAX_NODES, RETAINED_MAP_MAX_PAGE_ITEMS, RETAINED_MAP_MAX_SNAPSHOT_BYTES,
    RETAINED_MAP_MAX_STALE_CONTINUATIONS, RETAINED_MAP_MAX_TEXT_BYTES,
    RETAINED_MAP_MAX_TRAVEL_BINDINGS, RETAINED_MAP_PRODUCER_VERSION, RetainedMapContents,
    RetainedMapEdge, RetainedMapEdgeInput, RetainedMapNode, RetainedMapNodeInput,
    RetainedMapNodeKind, RetainedMapNodeReference, RetainedMapSnapshot, RetainedMapSnapshotInput,
    RetainedMapTravel, RetainedMapTravelInput, RetainedMapTravelReference,
};
pub use page::{
    RetainedMapContinuation, RetainedMapNodeSummary, RetainedMapTopologyPage,
    RetainedMapTopologyQuery,
};
pub use reader::RetainedMapReader;
pub use source::{
    FixtureRetainedMapSource, RetainedMapCapability, RetainedMapSource,
    UnavailableRetainedMapSource,
};
