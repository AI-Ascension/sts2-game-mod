// SPDX-License-Identifier: MIT

//! Source-only game-asset metadata and bounded permitted rendition retrieval.
//!
//! This module copies the asset metadata the installed build links to game content into an
//! immutable catalog fenced by the existing content manifest, locale, content-set revision and an
//! owner-local producer version.  An agent can therefore learn which icon, art or audio belongs to
//! a definition and what its media properties are without reaching a filesystem path, an arbitrary
//! URL, or an unrestricted binary blob.
//!
//! The distinctions the rest of the boundary depends on are preserved rather than flattened: an
//! asset the installed build does not contain is not an asset this boundary merely cannot serve,
//! an unknown property is never converted into zero, an empty dimension or duration, a bytes count
//! or an invented description, an owner-only or hidden asset is never returned outside a scope that
//! may observe it, and a partial page is never labelled complete.  Binary bytes never travel with
//! metadata: the retained catalog and every listing or lookup surface carry a descriptor only, and
//! bytes leave this boundary through one explicitly bounded, explicitly permitted rendition call
//! whose media type can never be an executable markup type.  Handles are opaque and are refused
//! when they name a path or a URL, so retrieval cannot become filesystem export.  The slice is
//! read-only and inert by construction: it exposes no install, extraction, execution or game-data
//! mutation entry point.  No native asset read, transport route or host compatibility is claimed.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod field;
mod handle;
mod identity;
mod kind;
mod model;
mod validation;
mod value;

pub use catalog::{
    AssetCatalogProducer, AssetCatalogSnapshot, AssetReadPort, UnavailableAssetHost,
};
pub use catalog_reader::*;
pub use definition::*;
pub use error::{
    AssetReadAvailability, AssetReferenceError, AssetRenditionAuthority, AssetSourceError,
};
pub use field::{AssetField, AssetFieldAvailability, AssetFieldStatus};
pub use handle::{
    AssetHandle, AssetHandleLease, AssetRendition, AssetRenditionAvailability,
    AssetRenditionLimits, AssetRenditionPayload, AssetRenditionRequest,
};
pub use identity::{AssetCatalogBinding, AssetRevision, is_opaque_handle};
pub use kind::{
    AssetClassCoverage, AssetClassState, AssetMediaClass, AssetMediaKind, AssetRetrievalState,
    AssetVisibility, AssetVisibilityScope,
};
pub use model::*;
pub use value::{
    AssetByteSize, AssetFieldValue, AssetMediaProperties, AssetOrigin, AssetRenditionDescriptor,
    is_markup_media_type,
};
