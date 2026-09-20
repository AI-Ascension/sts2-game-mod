// SPDX-License-Identifier: MIT

use super::super::{
    SemanticCaptureWindow, SemanticCatalogBinding, SemanticEventKind, SemanticEventRecord,
    SemanticEventScope, SemanticFamilyCoverage, SemanticHistoryAuthority, SemanticHistoryScope,
};

/// One bounded, read-only view of a semantic gameplay history.
///
/// The view keeps a disclosed gap as a record, so a consumer sees that a sequence number was
/// occupied by something the boundary could not observe instead of reading a contiguous history that
/// silently lost it. Every entry carries its own coverage, and the window states the spans that are
/// not fully captured.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticHistoryView {
    /// Catalog witness for this read.
    pub binding: SemanticCatalogBinding,
    /// Capability this read does not grant.
    pub authority: SemanticHistoryAuthority,
    /// Scope the caller observed from.
    pub scope: SemanticHistoryScope,
    /// Explicit support coverage for the history family.
    pub family: SemanticFamilyCoverage,
    /// The run, branch, episode and epoch this history belongs to.
    pub event_scope: SemanticEventScope,
    /// What the boundary observed and which spans it could not fully observe.
    pub window: SemanticCaptureWindow,
    /// Records in ascending sequence order, gaps included.
    pub events: Vec<SemanticEventRecord>,
    /// Number of records this scope observes, gaps included.
    pub total: usize,
}

impl SemanticHistoryView {
    /// Returns the observed events of this view, excluding disclosed gaps.
    #[must_use]
    pub fn observed(&self) -> Vec<&SemanticEventRecord> {
        self.events
            .iter()
            .filter(|event| event.is_observed())
            .collect()
    }

    /// Returns the disclosed gaps of this view.
    #[must_use]
    pub fn gaps(&self) -> Vec<&SemanticEventRecord> {
        self.events
            .iter()
            .filter(|event| !event.is_observed())
            .collect()
    }

    /// Returns every distinct kind this view observed, in the inventory's own order.
    #[must_use]
    pub fn observed_kinds(&self) -> Vec<SemanticEventKind> {
        let present = self
            .observed()
            .into_iter()
            .filter_map(|event| event.event.kind)
            .collect::<std::collections::BTreeSet<_>>();
        SemanticEventKind::ALL
            .into_iter()
            .filter(|kind| present.contains(kind))
            .collect()
    }

    /// Returns whether a parent and a child it causes are both visible here in causal order.
    #[must_use]
    pub fn causal_pair_visible(&self, parent_id: &str, child_id: &str) -> bool {
        let parent = self
            .events
            .iter()
            .find(|event| event.event_id() == parent_id);
        let child = self
            .events
            .iter()
            .find(|event| event.event_id() == child_id);
        match (parent, child) {
            (Some(parent), Some(child)) => {
                child
                    .event
                    .causal_parent
                    .as_ref()
                    .is_some_and(|causal| causal.stated_parent() == Some(parent_id))
                    && parent.sequence().precedes(&child.sequence())
            }
            _ => false,
        }
    }
}
