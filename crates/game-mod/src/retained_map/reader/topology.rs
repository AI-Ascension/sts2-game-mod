// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::binding::{
    RetainedMapFreshness, RetainedMapLiveBinding, RetainedMapTravelActionability, visible,
};
use super::super::error::{RetainedMapError, RetainedMapUnavailableReason};
use super::super::model::{
    RETAINED_MAP_MAX_PAGE_ITEMS, RetainedMapEdge, RetainedMapEdgeReference, RetainedMapNode,
    RetainedMapNodeReference, RetainedMapSnapshot, RetainedMapTravelReference,
};
use super::super::page::{
    RetainedMapContinuation, RetainedMapCursorState, RetainedMapTopologyPage,
    RetainedMapTopologyQuery,
};
use super::{RetainedMapReader, summary};

impl RetainedMapReader {
    /// Lists visible retained nodes and edges in stable identity order while the map may be closed.
    ///
    /// Both windows are bounded by the requested limit and visibility-filtered: an edge is disclosed
    /// only when both endpoint nodes exist and are visible under the reader's scope. A page is
    /// `complete` only when node and edge enumeration are both exhausted.
    pub fn topology(
        &mut self,
        query: &RetainedMapTopologyQuery,
    ) -> Result<RetainedMapTopologyPage, RetainedMapError> {
        if query.limit == 0 || query.limit > RETAINED_MAP_MAX_PAGE_ITEMS {
            return Err(RetainedMapError::InvalidPageSize);
        }
        if self.withheld {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::PolicyWithheld,
            ));
        }
        let Some(binding) = self
            .retained
            .as_ref()
            .map(|snapshot| snapshot.binding().clone())
        else {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::NeverObserved,
            ));
        };
        let (node_start, edge_start) = self.cursor_start(query, &binding)?;
        let Some(snapshot) = self.retained.as_ref() else {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::NeverObserved,
            ));
        };
        let entries = snapshot
            .nodes()
            .values()
            .filter(|node| visible(node.visibility, self.scope))
            .map(|node| summary(node, &binding))
            .collect::<Vec<_>>();
        let total = entries.len();
        let node_start = node_start.min(total);
        let node_end = node_start.saturating_add(query.limit).min(total);
        let page_entries = entries[node_start..node_end].to_vec();

        let mut edges = snapshot
            .edges()
            .iter()
            .filter(|edge| self.edge_is_visible(snapshot, edge))
            .map(|edge| RetainedMapEdgeReference {
                binding: binding.clone(),
                from_node_id: edge.from_node_id.clone(),
                to_node_id: edge.to_node_id.clone(),
            })
            .collect::<Vec<_>>();
        edges.sort_by(|left, right| {
            (&left.from_node_id, &left.to_node_id).cmp(&(&right.from_node_id, &right.to_node_id))
        });
        let total_edges = edges.len();
        let edge_start = edge_start.min(total_edges);
        let edge_end = edge_start.saturating_add(query.limit).min(total_edges);
        let page_edges = edges[edge_start..edge_end].to_vec();

        let continuation = if node_end < total || edge_end < total_edges {
            let token = self.make_token();
            self.insert_page(
                token.clone(),
                RetainedMapCursorState {
                    binding: binding.clone(),
                    limit: query.limit,
                    offset: node_end,
                    edge_offset: edge_end,
                },
            );
            Some(RetainedMapContinuation::new(
                token,
                Arc::clone(&self.continuation_scope),
            ))
        } else {
            None
        };
        let complete = continuation.is_none() && self.freshness.trusts_topology();
        Ok(RetainedMapTopologyPage {
            binding: Some(binding),
            freshness: self.freshness,
            entries: page_entries,
            total,
            edges: page_edges,
            total_edges,
            complete,
            continuation,
        })
    }

    /// Reads one retained node by its exact snapshot-bound identity.
    pub fn node(
        &self,
        reference: &RetainedMapNodeReference,
    ) -> Result<RetainedMapNode, RetainedMapError> {
        if self.withheld {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::PolicyWithheld,
            ));
        }
        let Some(snapshot) = &self.retained else {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::NeverObserved,
            ));
        };
        if &reference.binding != snapshot.binding() {
            return Err(RetainedMapError::StaleReference);
        }
        let node = snapshot
            .nodes()
            .get(&reference.node_id)
            .ok_or(RetainedMapError::NodeNotFound)?;
        if !visible(node.visibility, self.scope) {
            return Err(RetainedMapError::ScopeDenied("node"));
        }
        Ok(node.clone())
    }

    /// Returns retained travel bindings joined to their current actionability.
    ///
    /// Withheld knowledge and pre-observation reads fail closed instead of returning an empty
    /// success.
    pub fn travel_references(&self) -> Result<Vec<RetainedMapTravelReference>, RetainedMapError> {
        if self.withheld {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::PolicyWithheld,
            ));
        }
        let Some(snapshot) = &self.retained else {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::NeverObserved,
            ));
        };
        let actionability = self.actionability();
        Ok(snapshot
            .travel()
            .values()
            .filter(|travel| {
                self.endpoint_is_visible(snapshot, &travel.from_node_id)
                    && self.endpoint_is_visible(snapshot, &travel.to_node_id)
            })
            .map(|travel| RetainedMapTravelReference {
                binding: snapshot.binding().clone(),
                from_node_id: travel.from_node_id.clone(),
                to_node_id: travel.to_node_id.clone(),
                action_id: travel.action_id.clone(),
                actionability,
            })
            .collect())
    }

    /// Returns whether travel may currently be authorized.
    #[must_use]
    pub fn travel_actionability(&self) -> RetainedMapTravelActionability {
        self.actionability()
    }

    /// Authorizes navigation only for a current, open, retained binding.
    pub fn authorize_travel(
        &self,
        reference: &RetainedMapTravelReference,
    ) -> Result<(), RetainedMapError> {
        if self.withheld {
            return Err(RetainedMapError::TravelNotActionable(
                RetainedMapTravelActionability::Withheld,
            ));
        }
        let Some(snapshot) = &self.retained else {
            return Err(RetainedMapError::Unavailable(
                RetainedMapUnavailableReason::NeverObserved,
            ));
        };
        if &reference.binding != snapshot.binding() {
            return Err(RetainedMapError::StaleReference);
        }
        if !self.endpoint_is_visible(snapshot, &reference.from_node_id)
            || !self.endpoint_is_visible(snapshot, &reference.to_node_id)
        {
            return Err(RetainedMapError::ScopeDenied("travel"));
        }
        let expected = self.actionability();
        if !expected.is_actionable() || !reference.actionability.is_actionable() {
            return Err(RetainedMapError::TravelNotActionable(expected));
        }
        if !snapshot.travel().values().any(|travel| {
            travel.from_node_id == reference.from_node_id
                && travel.to_node_id == reference.to_node_id
                && travel.action_id == reference.action_id
        }) {
            return Err(RetainedMapError::StaleReference);
        }
        Ok(())
    }

    fn actionability(&self) -> RetainedMapTravelActionability {
        if self.withheld {
            return RetainedMapTravelActionability::Withheld;
        }
        match self.freshness {
            RetainedMapFreshness::Current if self.screen_open => {
                RetainedMapTravelActionability::Current
            }
            RetainedMapFreshness::Current | RetainedMapFreshness::Retained => {
                RetainedMapTravelActionability::Retained
            }
            RetainedMapFreshness::Stale => RetainedMapTravelActionability::Stale,
            RetainedMapFreshness::Withheld => RetainedMapTravelActionability::Withheld,
            RetainedMapFreshness::Unavailable | RetainedMapFreshness::NeverObserved => {
                RetainedMapTravelActionability::Unavailable
            }
            RetainedMapFreshness::Unknown => RetainedMapTravelActionability::Unknown,
        }
    }

    /// Returns whether one retained node identity is visible under the reader's scope.
    fn endpoint_is_visible(&self, snapshot: &RetainedMapSnapshot, node_id: &str) -> bool {
        snapshot
            .nodes()
            .get(node_id)
            .is_some_and(|node| visible(node.visibility, self.scope))
    }

    /// Returns whether both retained edge endpoints exist and are visible under the reader's scope.
    fn edge_is_visible(&self, snapshot: &RetainedMapSnapshot, edge: &RetainedMapEdge) -> bool {
        self.endpoint_is_visible(snapshot, &edge.from_node_id)
            && self.endpoint_is_visible(snapshot, &edge.to_node_id)
    }

    fn cursor_start(
        &mut self,
        query: &RetainedMapTopologyQuery,
        binding: &RetainedMapLiveBinding,
    ) -> Result<(usize, usize), RetainedMapError> {
        let Some(continuation) = &query.continuation else {
            return Ok((0, 0));
        };
        if !Arc::ptr_eq(&continuation.scope, &self.continuation_scope) {
            return Err(RetainedMapError::InvalidContinuation);
        }
        let state = self
            .pages
            .remove(continuation.token())
            .ok_or(RetainedMapError::InvalidContinuation)?;
        if &state.binding != binding || state.limit != query.limit {
            return Err(RetainedMapError::InvalidContinuation);
        }
        Ok((state.offset, state.edge_offset))
    }
}
