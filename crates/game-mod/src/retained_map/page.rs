// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::binding::{RetainedMapFreshness, RetainedMapLiveBinding, RetainedMapNodeVisibility};
use super::field::RetainedMapFieldStatus;
use super::model::{
    RetainedMapContents, RetainedMapEdgeReference, RetainedMapNodeKind, RetainedMapNodeReference,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use retained topology continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RetainedMapContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RetainedMapContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded retained topology page request.
#[derive(Debug, Eq, PartialEq)]
pub struct RetainedMapTopologyQuery {
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RetainedMapContinuation>,
}

/// Explicit retained node summary returned by one bounded page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapNodeSummary {
    /// Exact retained node reference.
    pub reference: RetainedMapNodeReference,
    /// Already-public node category.
    pub kind: RetainedMapNodeKind,
    /// Visibility of the node.
    pub visibility: RetainedMapNodeVisibility,
    /// Availability of the localized label.
    pub label: RetainedMapFieldStatus,
    /// Explicitly bounded contents description.
    pub contents: RetainedMapContents,
}

/// Complete or partial retained topology page.
///
/// A page carries a bounded window of node summaries and a bounded window of directed edges. Both
/// windows are visibility-filtered and share the request limit. The page is `complete` only when
/// node and edge enumeration are both exhausted and the retained knowledge is trustworthy.
#[derive(Debug, Eq, PartialEq)]
pub struct RetainedMapTopologyPage {
    /// Retained snapshot fence, or `None` before any permitted observation.
    pub binding: Option<RetainedMapLiveBinding>,
    /// Honest freshness of the retained knowledge backing this page.
    pub freshness: RetainedMapFreshness,
    /// Deterministically ordered node summaries.
    pub entries: Vec<RetainedMapNodeSummary>,
    /// Number of visible retained nodes.
    pub total: usize,
    /// Deterministically ordered retained directed edges with both endpoints visible.
    pub edges: Vec<RetainedMapEdgeReference>,
    /// Number of visible retained edges.
    pub total_edges: usize,
    /// Whether nodes and edges are fully enumerated and the knowledge is trustworthy.
    pub complete: bool,
    /// Present only when either window is partial.
    pub continuation: Option<RetainedMapContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct RetainedMapCursorState {
    pub(super) binding: RetainedMapLiveBinding,
    pub(super) limit: usize,
    pub(super) offset: usize,
    pub(super) edge_offset: usize,
}
