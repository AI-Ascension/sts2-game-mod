// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::binding::{RetainedMapFreshness, RetainedMapLiveBinding, RetainedMapNodeVisibility};
use super::field::RetainedMapFieldStatus;
use super::model::{RetainedMapContents, RetainedMapNodeKind, RetainedMapNodeReference};

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
    /// Whether the page is both fully enumerated and trustworthy as current/retained.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RetainedMapContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct RetainedMapCursorState {
    pub(super) binding: RetainedMapLiveBinding,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
