// SPDX-License-Identifier: MIT

//! Metadata projections: what a page may state about one asset, and never its bytes.

use super::super::{
    AssetEntry, AssetEntryReference, AssetRenditionAvailability, AssetRetrievalState,
    AssetRevision, AssetSummary,
};
use super::page::AssetListQuery;

/// Maps one declared retrieval state to the availability class a rendition may report.
pub(super) fn availability_of(entry: &AssetEntry) -> AssetRenditionAvailability {
    match entry.retrieval_state {
        AssetRetrievalState::Retrievable => AssetRenditionAvailability::Available,
        AssetRetrievalState::MetadataOnly => AssetRenditionAvailability::MetadataOnly,
        AssetRetrievalState::RetrievalUnavailable => {
            AssetRenditionAvailability::RetrievalUnavailable
        }
        AssetRetrievalState::AssetMissing => AssetRenditionAvailability::AssetMissing,
    }
}

/// Projects one retained entry onto the byte-free metadata surface a page returns.
pub(super) fn summary(entry: &AssetEntry, revision: &AssetRevision) -> AssetSummary {
    AssetSummary {
        reference: AssetEntryReference {
            catalog: entry.binding.clone(),
            handle: entry.handle.clone(),
        },
        revision: revision.clone(),
        definition: entry.definition.clone(),
        media_kind: entry.media_kind,
        retrieval_state: entry.retrieval_state,
        media_properties: entry.media_properties.clone(),
        byte_size: entry.byte_size.clone(),
        visibility: entry.visibility,
    }
}

/// Returns whether one asset is observable under this scope and filter.
pub(super) fn observable(entry: &AssetEntry, query: &AssetListQuery) -> bool {
    query.scope.observes(entry.visibility)
        && query.media_kind.is_none_or(|kind| entry.media_kind == kind)
        && query
            .retrieval_state
            .is_none_or(|state| entry.retrieval_state == state)
}
