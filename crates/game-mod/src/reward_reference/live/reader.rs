// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{RewardCatalogReader, RewardLiveError};
use super::identity::{ReaderScope, RewardLiveSnapshotFence, RewardLiveSnapshotReference};
use super::measure::{offer_bytes, snapshot_bytes};
use super::snapshot::{
    REWARD_LIVE_MAX_DETAIL_BYTES, REWARD_LIVE_MAX_OFFERS, REWARD_LIVE_MAX_SNAPSHOT_BYTES,
    RewardLiveSnapshotInput,
};
use super::source::{RewardLiveCaptureRequest, RewardLiveSource};
use super::validation::validate_snapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Explicit invalidation cause for reader-local live handles and continuations.
pub enum RewardLiveInvalidation {
    /// A new source capture began and invalidated the preceding observation.
    CaptureStarted,
    /// A source read or validation failed; no prior observation remains current.
    SourceFailure,
    /// The caller observed a reward claim or selection mutation.
    ClaimOrSelection,
    /// The caller observed a room transition.
    RoomChanged,
    /// The caller observed a run transition.
    RunChanged,
    /// The caller observed a host epoch transition.
    EpochChanged,
    /// The selected profile or game instance changed.
    ProfileChanged,
    /// The static catalog generation changed.
    CatalogChanged,
    /// The active locale changed.
    LocaleChanged,
    /// The source restored a prior saved state.
    Restored,
    /// The source restarted or reinitialized.
    Restarted,
    /// An owner explicitly invalidated the retained observation.
    Manual,
    /// The local generation counter can no longer advance safely.
    GenerationExhausted,
}

pub(super) struct RetainedSnapshot {
    pub(super) input: RewardLiveSnapshotInput,
    pub(super) fence: Arc<RewardLiveSnapshotFence>,
    pub(super) reader_generation: u64,
    pub(super) retained_bytes: usize,
}

/// Reader retaining one bounded immutable live snapshot and one existing static catalog reader.
///
/// Every capture invalidates old live handles before touching the source. Subsequent list, get,
/// detail, and resolution calls use retained owned values only.
pub struct RewardLiveReader {
    pub(super) source: Box<dyn RewardLiveSource>,
    pub(super) catalog: RewardCatalogReader,
    pub(super) instance_id: super::identity::RewardLiveInstanceId,
    pub(super) generation: u64,
    pub(super) current: Option<RetainedSnapshot>,
    pub(super) scope: Arc<ReaderScope>,
    disabled: bool,
    last_invalidation: Option<RewardLiveInvalidation>,
}

impl std::fmt::Debug for RewardLiveReader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RewardLiveReader")
            .field("generation", &self.generation)
            .field("has_current_snapshot", &self.current.is_some())
            .field("last_invalidation", &self.last_invalidation)
            .field("disabled", &self.disabled)
            .finish()
    }
}

impl RewardLiveReader {
    /// Creates an independent reader scope over one object-safe source and static catalog.
    #[must_use]
    pub fn new<S: RewardLiveSource + 'static>(
        source: S,
        catalog: RewardCatalogReader,
        instance_id: super::identity::RewardLiveInstanceId,
    ) -> Self {
        Self::from_box(Box::new(source), catalog, instance_id)
    }

    /// Creates a reader from an already boxed source port.
    #[must_use]
    pub fn from_box(
        source: Box<dyn RewardLiveSource>,
        catalog: RewardCatalogReader,
        instance_id: super::identity::RewardLiveInstanceId,
    ) -> Self {
        Self {
            source,
            catalog,
            instance_id,
            generation: 0,
            current: None,
            scope: Arc::new(ReaderScope { _scope: 0 }),
            disabled: false,
            last_invalidation: None,
        }
    }

    /// Copies, validates, bounds, and retains one complete current observation.
    ///
    /// Any source or validation failure leaves no current snapshot. The source read occurs only
    /// here; retained queries never recapture, claim, or evaluate RNG.
    pub fn capture(&mut self) -> Result<RewardLiveSnapshotReference, RewardLiveError> {
        self.invalidate(RewardLiveInvalidation::CaptureStarted)?;
        let expected_catalog = self.catalog.catalog().binding().clone();
        let request = RewardLiveCaptureRequest {
            catalog: &expected_catalog,
            instance_id: &self.instance_id,
        };
        let input = match self.source.capture(&request) {
            Ok(input) => input,
            Err(error) => {
                self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
                return Err(RewardLiveError::Source(error));
            }
        };
        if let Err(error) = preflight(&input) {
            self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
            return Err(error);
        }
        let retained_bytes = match snapshot_bytes(&input) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
                return Err(error);
            }
        };
        if retained_bytes > REWARD_LIVE_MAX_SNAPSHOT_BYTES {
            self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
            return Err(RewardLiveError::SnapshotTooLarge {
                limit: REWARD_LIVE_MAX_SNAPSHOT_BYTES,
                actual: retained_bytes,
            });
        }
        for offer in &input.offers.entries {
            let detail_bytes = match offer_bytes(offer) {
                Ok(bytes) => bytes,
                Err(error) => {
                    self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
                    return Err(error);
                }
            };
            if detail_bytes > REWARD_LIVE_MAX_DETAIL_BYTES {
                self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
                return Err(RewardLiveError::DetailTooLarge {
                    limit: REWARD_LIVE_MAX_DETAIL_BYTES,
                    actual: detail_bytes,
                });
            }
        }
        if let Err(error) = validate_snapshot(
            &input,
            &expected_catalog,
            self.instance_id.as_str(),
            &self.catalog,
        ) {
            self.last_invalidation = Some(RewardLiveInvalidation::SourceFailure);
            return Err(error);
        }
        let fence = Arc::new(RewardLiveSnapshotFence {
            catalog: input.catalog.clone(),
            instance_id: input.instance_id.clone(),
            run_id: input.run_id.clone(),
            room_id: input.room_id.clone(),
            epoch: input.epoch,
            state_generation: input.state_generation,
            snapshot_id: input.snapshot_id.clone(),
            source_revision: input.source_revision.clone(),
        });
        let snapshot = self.snapshot_reference(Arc::clone(&fence), self.generation);
        self.current = Some(RetainedSnapshot {
            input,
            fence,
            reader_generation: self.generation,
            retained_bytes,
        });
        self.last_invalidation = None;
        Ok(snapshot)
    }

    /// Explicitly invalidates every handle and continuation held by this reader.
    pub fn invalidate(&mut self, reason: RewardLiveInvalidation) -> Result<(), RewardLiveError> {
        self.current = None;
        self.last_invalidation = Some(reason);
        if self.disabled {
            return Err(RewardLiveError::GenerationExhausted);
        }
        match self.generation.checked_add(1) {
            Some(next) => {
                self.generation = next;
                Ok(())
            }
            None => {
                self.disabled = true;
                self.last_invalidation = Some(RewardLiveInvalidation::GenerationExhausted);
                Err(RewardLiveError::GenerationExhausted)
            }
        }
    }

    /// Rebinds the reader to a selected instance after invalidating its current handles.
    pub fn rebind_instance(
        &mut self,
        instance_id: super::identity::RewardLiveInstanceId,
    ) -> Result<(), RewardLiveError> {
        self.invalidate(RewardLiveInvalidation::ProfileChanged)?;
        self.instance_id = instance_id;
        Ok(())
    }

    /// Replaces the retained static catalog after invalidating all live references.
    pub fn replace_catalog(
        &mut self,
        catalog: RewardCatalogReader,
        reason: RewardLiveInvalidation,
    ) -> Result<(), RewardLiveError> {
        if !matches!(
            reason,
            RewardLiveInvalidation::CatalogChanged | RewardLiveInvalidation::LocaleChanged
        ) {
            return Err(RewardLiveError::InvalidInput("catalog_invalidation_reason"));
        }
        self.invalidate(reason)?;
        self.catalog = catalog;
        Ok(())
    }

    /// Returns the most recent invalidation cause, including source failure.
    #[must_use]
    pub const fn last_invalidation(&self) -> Option<RewardLiveInvalidation> {
        self.last_invalidation
    }

    /// Returns the current retained source-byte estimate; it is not a wire-size estimate.
    pub fn retained_source_bytes(&self) -> Result<usize, RewardLiveError> {
        self.current
            .as_ref()
            .map(|snapshot| snapshot.retained_bytes)
            .ok_or(RewardLiveError::NoCurrentSnapshot)
    }
}

fn preflight(input: &RewardLiveSnapshotInput) -> Result<(), RewardLiveError> {
    if input.offers.entries.len() > REWARD_LIVE_MAX_OFFERS {
        return Err(RewardLiveError::InvalidInput("offer_count"));
    }
    for offer in &input.offers.entries {
        if offer.groups.entries.len() > super::snapshot::REWARD_LIVE_MAX_GROUPS_PER_OFFER
            || offer.items.entries.len() > super::snapshot::REWARD_LIVE_MAX_ITEMS_PER_OFFER
        {
            return Err(RewardLiveError::InvalidInput("offer_member_count"));
        }
        for group in &offer.groups.entries {
            if group.actions.entries.len() > super::snapshot::REWARD_LIVE_MAX_ACTIONS_PER_GROUP {
                return Err(RewardLiveError::InvalidInput("action_count"));
            }
            if group.items.entries.len() > super::snapshot::REWARD_LIVE_MAX_ITEMS_PER_OFFER {
                return Err(RewardLiveError::InvalidInput("group_item_count"));
            }
            for action in &group.actions.entries {
                if action.targets.entries.len()
                    > super::snapshot::REWARD_LIVE_MAX_TARGETS_PER_ACTION
                {
                    return Err(RewardLiveError::InvalidInput("action_target_count"));
                }
            }
        }
        for item in &offer.items.entries {
            for quantity in [&item.quantity.base_amount, &item.quantity.visible_amount] {
                if let super::super::RewardNumericValue::Formula(formula) = quantity
                    && formula.unresolved_inputs.len() > super::super::REWARD_MAX_FORMULA_INPUTS
                {
                    return Err(RewardLiveError::InvalidInput("formula_input_count"));
                }
            }
        }
    }
    Ok(())
}
