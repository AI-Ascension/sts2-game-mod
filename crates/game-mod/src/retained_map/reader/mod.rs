// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::sync::Arc;

use super::binding::{
    RetainedMapFreshness, RetainedMapLiveBinding, RetainedMapObservationState,
    RetainedMapVisibilityScope,
};
use super::error::RetainedMapError;
use super::model::{RETAINED_MAP_MAX_STALE_CONTINUATIONS, RetainedMapNode, RetainedMapSnapshot};
use super::page::{ContinuationScope, RetainedMapCursorState, RetainedMapNodeSummary};
use super::source::{RetainedMapCapability, RetainedMapSource, map_error};

mod topology;

/// Reader retaining one already-public map snapshot while enforcing live identity and scope.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Reads serve retained knowledge and never open the map surface.
///
/// Generation freshness, reveal-policy withholding, and surface-open state are independent
/// dimensions: withholding is preserved across `reconcile`, and opening the surface never re-arms
/// travel without a fresh observation or reconciliation.
#[derive(Debug)]
pub struct RetainedMapReader {
    pub(super) retained: Option<RetainedMapSnapshot>,
    pub(super) observation: RetainedMapObservationState,
    pub(super) freshness: RetainedMapFreshness,
    pub(super) withheld: bool,
    pub(super) screen_open: bool,
    pub(super) scope: RetainedMapVisibilityScope,
    pub(super) pages: BTreeMap<String, RetainedMapCursorState>,
    pub(super) next_cursor: u64,
    pub(super) continuation_scope: Arc<ContinuationScope>,
}

impl RetainedMapReader {
    /// Creates a reader with no permitted observation yet.
    #[must_use]
    pub fn new(scope: RetainedMapVisibilityScope) -> Self {
        Self {
            retained: None,
            observation: RetainedMapObservationState::Closed,
            freshness: RetainedMapFreshness::NeverObserved,
            withheld: false,
            screen_open: false,
            scope,
            pages: BTreeMap::new(),
            next_cursor: 0,
            continuation_scope: Arc::new(ContinuationScope),
        }
    }

    /// Observes and adopts one permitted snapshot against an exact identity fence.
    pub fn from_source<S: RetainedMapSource>(
        source: &S,
        expected: &RetainedMapLiveBinding,
        scope: RetainedMapVisibilityScope,
    ) -> Result<Self, RetainedMapError> {
        let mut reader = Self::new(scope);
        reader.observe(source, expected)?;
        Ok(reader)
    }

    /// Reobserves the map while permitted, recording only already-public topology.
    ///
    /// A closed, forbidden, unsupported, or unknown surface fails without opening the UI and
    /// revokes current authority. A rejected read that proves a changed live identity or generation
    /// marks the retained knowledge stale so an old reference can never authorize travel. Transient
    /// failures that establish nothing about the live surface leave retained state intact.
    pub fn observe<S: RetainedMapSource>(
        &mut self,
        source: &S,
        expected: &RetainedMapLiveBinding,
    ) -> Result<(), RetainedMapError> {
        if let RetainedMapCapability::Unavailable(reason) = source.capability() {
            return Err(RetainedMapError::Unavailable(reason));
        }
        let observation = source.observation_state();
        if observation != RetainedMapObservationState::Observable {
            self.observation = observation;
            self.screen_open = false;
            if self.freshness == RetainedMapFreshness::Current {
                self.freshness = RetainedMapFreshness::Retained;
            }
            return Err(RetainedMapError::MapNotObservable(observation));
        }
        let input = source
            .read_snapshot(expected, self.scope)
            .map_err(map_error)?;
        if input.binding != *expected {
            self.stale_identity();
            return Err(RetainedMapError::StaleSource);
        }
        let candidate = RetainedMapSnapshot::from_input(input)?;
        if let Some(current) = &self.retained
            && let Err(error) = validate_replacement(current.binding(), candidate.binding())
        {
            self.stale_identity();
            return Err(error);
        }
        self.retained = Some(candidate);
        self.observation = RetainedMapObservationState::Observable;
        self.freshness = RetainedMapFreshness::Current;
        self.screen_open = true;
        Ok(())
    }

    fn stale_identity(&mut self) {
        self.freshness = RetainedMapFreshness::Stale;
        self.screen_open = false;
    }

    /// Marks the map surface closed without discarding retained knowledge.
    ///
    /// Closing only demotes a generation-current observation to `Retained`; it never changes
    /// policy withholding.
    pub fn close_screen(&mut self) {
        self.observation = RetainedMapObservationState::Closed;
        self.screen_open = false;
        if self.freshness == RetainedMapFreshness::Current {
            self.freshness = RetainedMapFreshness::Retained;
        }
    }

    /// Marks the map surface observable without performing a read.
    ///
    /// Merely opening the surface never refreshes or re-arms retained knowledge; only a subsequent
    /// `observe` or `reconcile` can make it current again.
    pub fn open_screen(&mut self) {
        self.observation = RetainedMapObservationState::Observable;
    }

    /// Withholds retained knowledge after a reveal-policy change.
    ///
    /// Withholding is an independent policy dimension that survives `observe`, `reconcile`, and
    /// `replace_snapshot`. The topology stays retained but every read fails closed until an explicit
    /// [`RetainedMapReader::unwithhold`].
    pub fn withhold(&mut self) {
        self.withheld = true;
    }

    /// Clears a reveal-policy withholding.
    ///
    /// This is the only operation that un-withholds retained knowledge; it never promotes generation
    /// freshness on its own.
    pub fn unwithhold(&mut self) {
        self.withheld = false;
    }

    /// Reconciles retained knowledge against the complete current live snapshot fence.
    ///
    /// Any change to the catalog, instance, run, act, mode, map-instance, snapshot, or epoch marks
    /// the retained knowledge stale so it can never be read as current and the surface is treated
    /// as closed. The retained topology stays available for honest disclosure.
    pub fn reconcile(&mut self, live: &RetainedMapLiveBinding) -> RetainedMapFreshness {
        let freshness = match &self.retained {
            None => RetainedMapFreshness::NeverObserved,
            Some(snapshot) => {
                let current = snapshot.binding();
                if !current.same_snapshot_fence(live) {
                    RetainedMapFreshness::Stale
                } else if self.observation == RetainedMapObservationState::Observable {
                    RetainedMapFreshness::Current
                } else {
                    RetainedMapFreshness::Retained
                }
            }
        };
        self.freshness = freshness;
        self.screen_open = matches!(freshness, RetainedMapFreshness::Current);
        self.effective_freshness()
    }

    /// Returns the honest freshness of the retained knowledge.
    ///
    /// A policy withholding takes precedence over the generation freshness and survives
    /// [`RetainedMapReader::reconcile`].
    #[must_use]
    pub const fn freshness(&self) -> RetainedMapFreshness {
        self.effective_freshness()
    }

    /// Returns whether reveal policy currently withholds the retained knowledge.
    #[must_use]
    pub const fn is_withheld(&self) -> bool {
        self.withheld
    }

    const fn effective_freshness(&self) -> RetainedMapFreshness {
        if self.withheld {
            RetainedMapFreshness::Withheld
        } else {
            self.freshness
        }
    }

    /// Returns the last known map surface state.
    #[must_use]
    pub const fn observation(&self) -> RetainedMapObservationState {
        self.observation
    }

    /// Returns the selected visibility scope.
    #[must_use]
    pub const fn scope(&self) -> RetainedMapVisibilityScope {
        self.scope
    }

    /// Returns the retained snapshot fence, if any permitted observation happened.
    #[must_use]
    pub fn binding(&self) -> Option<&RetainedMapLiveBinding> {
        self.retained.as_ref().map(RetainedMapSnapshot::binding)
    }

    /// Replaces the retained snapshot only within the same identity and a newer epoch.
    pub fn replace_snapshot(&mut self, next: RetainedMapSnapshot) -> Result<(), RetainedMapError> {
        if let Some(current) = &self.retained {
            validate_replacement(current.binding(), next.binding())?;
        }
        self.retained = Some(next);
        self.freshness = if self.observation == RetainedMapObservationState::Observable {
            RetainedMapFreshness::Current
        } else {
            RetainedMapFreshness::Retained
        };
        self.screen_open = matches!(self.freshness, RetainedMapFreshness::Current);
        Ok(())
    }

    /// Inserts one bounded cursor registry entry, evicting the oldest when full.
    pub(super) fn insert_page(&mut self, token: String, state: RetainedMapCursorState) {
        if self.pages.len() >= RETAINED_MAP_MAX_STALE_CONTINUATIONS
            && let Some(oldest) = self.pages.keys().next().cloned()
        {
            self.pages.remove(&oldest);
        }
        self.pages.insert(token, state);
    }

    /// Allocates the next deterministic opaque token.
    pub(super) fn make_token(&mut self) -> String {
        let token = format!("retained-map-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

pub(super) fn validate_replacement(
    current: &RetainedMapLiveBinding,
    next: &RetainedMapLiveBinding,
) -> Result<(), RetainedMapError> {
    if next.catalog != current.catalog {
        return Err(RetainedMapError::CatalogMismatch);
    }
    if next.game_instance_id != current.game_instance_id {
        return Err(RetainedMapError::GameInstanceMismatch);
    }
    if next.run_id != current.run_id {
        return Err(RetainedMapError::RunMismatch);
    }
    if next.mode_id != current.mode_id {
        return Err(RetainedMapError::ModeMismatch);
    }
    if next.act_id != current.act_id {
        return Err(RetainedMapError::ActMismatch);
    }
    if next.map_instance_id != current.map_instance_id {
        return Err(RetainedMapError::MapInstanceMismatch);
    }
    if next.epoch <= current.epoch {
        return Err(RetainedMapError::NonMonotonicEpoch {
            current: current.epoch,
            supplied: next.epoch,
        });
    }
    Ok(())
}

fn summary(node: &RetainedMapNode, binding: &RetainedMapLiveBinding) -> RetainedMapNodeSummary {
    RetainedMapNodeSummary {
        reference: super::model::RetainedMapNodeReference {
            binding: binding.clone(),
            node_id: node.node_id.clone(),
        },
        kind: node.kind.clone(),
        visibility: node.visibility,
        label: node.label.status(),
        contents: node.contents,
    }
}
