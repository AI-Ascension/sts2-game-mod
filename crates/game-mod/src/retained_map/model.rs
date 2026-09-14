// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::binding::{
    RetainedMapLiveBinding, RetainedMapProvenance, RetainedMapTravelActionability,
};
use super::error::{RetainedMapError, RetainedMapUnavailableReason};
use super::field::RetainedMapField;

/// Owner-local producer identity for retained map knowledge.
pub const RETAINED_MAP_PRODUCER_VERSION: &str = "game-retained-map-v1";
/// Maximum bytes accepted for one identity token.
pub const RETAINED_MAP_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const RETAINED_MAP_MAX_TEXT_BYTES: usize = 4 * 1024;
/// Maximum retained topology nodes in one snapshot.
pub const RETAINED_MAP_MAX_NODES: usize = 256;
/// Maximum retained directed edges in one snapshot.
pub const RETAINED_MAP_MAX_EDGES: usize = 1_024;
/// Maximum retained generation-bound travel bindings in one snapshot.
pub const RETAINED_MAP_MAX_TRAVEL_BINDINGS: usize = 256;
/// Maximum entries returned by one bounded topology page.
pub const RETAINED_MAP_MAX_PAGE_ITEMS: usize = 64;
/// Maximum aggregate bytes retained for one node detail.
pub const RETAINED_MAP_MAX_DETAIL_BYTES: usize = 8 * 1024;
/// Maximum aggregate bytes retained by one coherent retained snapshot.
pub const RETAINED_MAP_MAX_SNAPSHOT_BYTES: usize = 256 * 1024;
/// Maximum unconsumed continuation tokens retained at once.
pub const RETAINED_MAP_MAX_STALE_CONTINUATIONS: usize = 128;

/// Already-public topology category of one node.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapNodeKind {
    /// The run-start node.
    Start,
    /// A normal combat node.
    Combat,
    /// An elite combat node.
    Elite,
    /// A boss node.
    Boss,
    /// A rest node.
    Rest,
    /// A shop node.
    Shop,
    /// A treasure node.
    Treasure,
    /// An event node whose outcome stays withheld.
    Event,
    /// The source could not classify the category.
    Unknown,
    /// The source knows a category but must not reveal it.
    Withheld,
    /// A new category is known but unsupported by this producer.
    Unsupported(String),
}

/// The only permitted contents description for a retained node.
///
/// A retained node never carries a hidden future outcome. Withheld, unavailable, and unknown
/// remain explicit so a caller cannot mistake absence for an observed empty room.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RetainedMapContents {
    /// Only the already-public node category is retained.
    PublicCategory,
    /// The node kind is visible but its future contents are withheld by policy.
    Withheld(RetainedMapUnavailableReason),
    /// The source supports contents but did not provide them.
    Unavailable(RetainedMapUnavailableReason),
    /// The source could not classify the contents.
    Unknown,
}

/// Source-owned topology node before validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapNodeInput {
    /// Live topology identity, distinct from a definition or action identity.
    pub node_id: String,
    /// Already-public node category.
    pub kind: RetainedMapNodeKind,
    /// Visibility of this node.
    pub visibility: super::binding::RetainedMapNodeVisibility,
    /// Optional localized label.
    pub label: RetainedMapField<String>,
    /// Explicitly bounded contents description.
    pub contents: RetainedMapContents,
}

/// Live topology node retained in an immutable snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapNode {
    /// Live topology identity, distinct from a definition or action identity.
    pub node_id: String,
    /// Already-public node category.
    pub kind: RetainedMapNodeKind,
    /// Visibility of this node.
    pub visibility: super::binding::RetainedMapNodeVisibility,
    /// Optional localized label.
    pub label: RetainedMapField<String>,
    /// Explicitly bounded contents description.
    pub contents: RetainedMapContents,
}

/// Source-owned directed edge before validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapEdgeInput {
    /// Edge source node.
    pub from_node_id: String,
    /// Edge destination node.
    pub to_node_id: String,
}

/// Validated directed edge between retained nodes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapEdge {
    /// Edge source node.
    pub from_node_id: String,
    /// Edge destination node.
    pub to_node_id: String,
}

/// Source-owned generation-bound travel binding before validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapTravelInput {
    /// Current node the travel starts from.
    pub from_node_id: String,
    /// Destination node.
    pub to_node_id: String,
    /// Generation-bound navigation action identity, distinct from a node identity.
    pub action_id: String,
}

/// Retained generation-bound travel binding without current authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapTravel {
    /// Current node the travel starts from.
    pub from_node_id: String,
    /// Destination node.
    pub to_node_id: String,
    /// Generation-bound navigation action identity, distinct from a node identity.
    pub action_id: String,
}

/// Source-owned retained snapshot before validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapSnapshotInput {
    /// Exact identity fence for this observation.
    pub binding: RetainedMapLiveBinding,
    /// Provenance of the retained knowledge.
    pub provenance: RetainedMapProvenance,
    /// Retained topology nodes.
    pub nodes: Vec<RetainedMapNodeInput>,
    /// Retained directed edges.
    pub edges: Vec<RetainedMapEdgeInput>,
    /// Retained generation-bound travel bindings.
    pub travel: Vec<RetainedMapTravelInput>,
}

/// Immutable validated retained map snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapSnapshot {
    pub(super) binding: RetainedMapLiveBinding,
    pub(super) provenance: RetainedMapProvenance,
    pub(super) nodes: BTreeMap<String, RetainedMapNode>,
    pub(super) edges: Vec<RetainedMapEdge>,
    pub(super) travel: BTreeMap<String, RetainedMapTravel>,
}

impl RetainedMapSnapshot {
    /// Validates and adopts source-owned input without retaining host objects.
    pub fn from_input(input: RetainedMapSnapshotInput) -> Result<Self, RetainedMapError> {
        super::validation::validate_snapshot(input)
    }

    /// Returns the exact snapshot identity fence.
    #[must_use]
    pub fn binding(&self) -> &RetainedMapLiveBinding {
        &self.binding
    }

    /// Returns the provenance of the retained knowledge.
    #[must_use]
    pub const fn provenance(&self) -> RetainedMapProvenance {
        self.provenance
    }

    /// Returns retained nodes by live topology identity.
    #[must_use]
    pub fn nodes(&self) -> &BTreeMap<String, RetainedMapNode> {
        &self.nodes
    }

    /// Returns retained directed edges in source order.
    #[must_use]
    pub fn edges(&self) -> &[RetainedMapEdge] {
        &self.edges
    }

    /// Returns retained travel bindings by action identity.
    #[must_use]
    pub fn travel(&self) -> &BTreeMap<String, RetainedMapTravel> {
        &self.travel
    }

    /// Returns the number of retained nodes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether no node was retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Exact retained topology node reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RetainedMapNodeReference {
    /// Snapshot identity fence.
    pub binding: RetainedMapLiveBinding,
    /// Live topology node identity.
    pub node_id: String,
}

/// Retained travel binding joined to its current actionability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedMapTravelReference {
    /// Snapshot identity fence.
    pub binding: RetainedMapLiveBinding,
    /// Current node the travel starts from.
    pub from_node_id: String,
    /// Destination node.
    pub to_node_id: String,
    /// Generation-bound navigation action identity.
    pub action_id: String,
    /// Whether this reference is currently actionable.
    pub actionability: RetainedMapTravelActionability,
}
