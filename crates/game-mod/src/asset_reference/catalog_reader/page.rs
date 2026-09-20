// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    AssetByteSize, AssetCatalogBinding, AssetDefinitionReference, AssetEntryReference,
    AssetFieldValue, AssetMediaKind, AssetMediaProperties, AssetRenditionAuthority,
    AssetRetrievalState, AssetRevision, AssetVisibility, AssetVisibilityScope,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use asset continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl AssetContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded, filtered, revision-fenced asset list request.
///
/// The request names the content revision the caller believes it is reading: a snapshot taken at
/// another revision is refused rather than answered from stale data.
#[derive(Debug, Eq, PartialEq)]
pub struct AssetListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Requested visibility scope.
    pub scope: AssetVisibilityScope,
    /// Content revision the caller believes it is reading, when it states one.
    pub revision: Option<AssetRevision>,
    /// Media-kind filter; absent lists every readable kind.
    pub media_kind: Option<AssetMediaKind>,
    /// Retrieval-state filter; absent lists every state the source reports.
    pub retrieval_state: Option<AssetRetrievalState>,
    /// Maximum assets in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<AssetContinuation>,
}

/// Typed metadata returned by one bounded page.
///
/// This type has no byte field at all, so an ordinary search or log surface cannot carry a blob.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetSummary {
    /// Exact asset reference, carrying its catalog witness.
    pub reference: AssetEntryReference,
    /// Documented freshness of the read this asset came from.
    pub revision: AssetRevision,
    /// Content definition the asset is linked to.
    pub definition: AssetDefinitionReference,
    /// Media kind the source declares.
    pub media_kind: AssetMediaKind,
    /// Whether the asset is retrievable, metadata-only, unavailable or missing.
    pub retrieval_state: AssetRetrievalState,
    /// Media type, dimensions and duration, or their stated absence.
    pub media_properties: AssetFieldValue<AssetMediaProperties>,
    /// Stored and decoded byte sizes, or their stated absence.
    pub byte_size: AssetFieldValue<AssetByteSize>,
    /// Owner-defined visibility.
    pub visibility: AssetVisibility,
}

impl AssetSummary {
    /// Returns whether this metadata surface carries bytes; it never does.
    #[must_use]
    pub const fn carries_binary(&self) -> bool {
        false
    }
}

/// Complete or explicitly partial asset page.
#[derive(Debug, Eq, PartialEq)]
pub struct AssetPage {
    /// Catalog witness for every asset.
    pub binding: AssetCatalogBinding,
    /// Documented freshness of this read.
    pub revision: AssetRevision,
    /// Capability this read does not grant.
    pub authority: AssetRenditionAuthority,
    /// Deterministically ordered assets.
    pub assets: Vec<AssetSummary>,
    /// Number of visible assets matching the filter.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<AssetContinuation>,
}

impl AssetPage {
    /// Returns whether this page is partial and must be continued to be complete.
    #[must_use]
    pub const fn is_partial(&self) -> bool {
        !self.complete
    }
}
