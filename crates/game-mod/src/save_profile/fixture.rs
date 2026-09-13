// SPDX-License-Identifier: MIT

use super::error::{ProfileDiscoveryError, ProfileFixtureError, ProfileSelectionRejection};
use super::identity::{BaselineFence, slot_limit_is_valid};
use super::model::{
    ProfileDiscovery, ProfileDiscoveryRequest, ProfileSelectionReceipt, ProfileSelectionRequest,
    SaveProfileStatus, SaveSlotSummary,
};
use super::port::{ProfileReadPort, ProfileSelectionAvailability, ProfileSelectionPort};
use super::{
    InstanceIdentity, ProfileIdentityError, SaveProfileBaseline, SaveSlotId,
    SelectionIdempotencyKey, UserDataIdentity,
};
use std::collections::BTreeMap;

fn fixture_identity_error(error: ProfileIdentityError) -> ProfileFixtureError {
    ProfileFixtureError::InvalidIdentity(error)
}

fn baseline_matches_user_data(
    baseline: &SaveProfileBaseline,
    user_data_id: &UserDataIdentity,
) -> bool {
    baseline.user_data_id() == user_data_id
}

fn context_error(
    expected_instance: &InstanceIdentity,
    expected_user_data: &UserDataIdentity,
    expected_baseline: &SaveProfileBaseline,
    fence: &BaselineFence,
) -> Result<(), (ProfileSelectionRejection, ProfileDiscoveryError)> {
    if fence.instance_id() != expected_instance {
        return Err((
            ProfileSelectionRejection::WrongInstance,
            ProfileDiscoveryError::WrongInstance,
        ));
    }
    if fence.baseline().user_data_id() != expected_user_data {
        return Err((
            ProfileSelectionRejection::WrongUserData,
            ProfileDiscoveryError::WrongUserData,
        ));
    }
    if fence.baseline() != expected_baseline {
        return Err((
            ProfileSelectionRejection::StaleBaseline,
            ProfileDiscoveryError::StaleBaseline,
        ));
    }
    Ok(())
}

/// Deterministic host-independent fixture for discovery, selection fences, and reconciliation.
///
/// Its only mutation is in-memory current-selection state. It never reads a path, creates a
/// directory, copies a profile, or claims that a native host supports the operation.
#[derive(Debug)]
pub struct FakeSaveProfileHost {
    instance_id: InstanceIdentity,
    user_data_id: UserDataIdentity,
    baseline: SaveProfileBaseline,
    slots: BTreeMap<SaveSlotId, SaveSlotSummary>,
    current_selection: Option<SaveSlotId>,
    history: BTreeMap<SelectionIdempotencyKey, (ProfileSelectionRequest, ProfileSelectionReceipt)>,
    selection_busy: bool,
    mutation_count: usize,
}

impl FakeSaveProfileHost {
    /// Creates a synthetic store with an explicit current selection (or no selection).
    pub fn new(
        fence: BaselineFence,
        slots: impl IntoIterator<Item = SaveSlotSummary>,
        current_selection: Option<SaveSlotId>,
    ) -> Result<Self, ProfileFixtureError> {
        let user_data_id = fence.baseline().user_data_id().clone();
        let instance_id = fence.instance_id().clone();
        let baseline = fence.baseline().clone();
        let mut indexed = BTreeMap::new();
        for slot in slots {
            if !slot_limit_is_valid(indexed.len() + 1) {
                return Err(ProfileFixtureError::TooManySlots);
            }
            if !baseline_matches_user_data(slot.baseline(), &user_data_id) {
                return Err(ProfileFixtureError::BaselineMismatch);
            }
            if slot.baseline() != &baseline {
                return Err(ProfileFixtureError::BaselineMismatch);
            }
            if indexed.insert(slot.slot_id().clone(), slot).is_some() {
                return Err(ProfileFixtureError::DuplicateSlot);
            }
        }
        if let Some(selected) = &current_selection
            && !indexed.contains_key(selected)
        {
            return Err(ProfileFixtureError::SelectionNotListed);
        }
        Ok(Self {
            instance_id,
            user_data_id,
            baseline,
            slots: indexed,
            current_selection,
            history: BTreeMap::new(),
            selection_busy: false,
            mutation_count: 0,
        })
    }

    /// Returns the number of synthetic selection mutations, useful for safety assertions.
    #[must_use]
    pub const fn mutation_count(&self) -> usize {
        self.mutation_count
    }

    /// Makes the fixture report a concurrent selection in progress.
    pub const fn set_selection_busy(&mut self, busy: bool) {
        self.selection_busy = busy;
    }

    /// Returns the owner-local baseline fence used by the fixture.
    pub fn fence(&self) -> Result<BaselineFence, ProfileFixtureError> {
        BaselineFence::new(self.instance_id.clone(), self.baseline.clone())
            .map_err(fixture_identity_error)
    }

    fn discovery(&self) -> Result<ProfileDiscovery, ProfileFixtureError> {
        ProfileDiscovery::new(
            self.fence()
                .map_err(|_| ProfileFixtureError::BaselineMismatch)?,
            self.slots.values().cloned().collect(),
            self.current_selection.clone(),
        )
    }

    fn context_matches(&self, fence: &BaselineFence) -> Result<(), ProfileSelectionRejection> {
        context_error(&self.instance_id, &self.user_data_id, &self.baseline, fence)
            .map_err(|(selection_error, _)| selection_error)
    }

    fn status_rejection(status: SaveProfileStatus) -> Option<ProfileSelectionRejection> {
        match status {
            SaveProfileStatus::Available => None,
            SaveProfileStatus::ActiveRun => Some(ProfileSelectionRejection::ActiveRun),
            SaveProfileStatus::InUse => Some(ProfileSelectionRejection::InUse),
            SaveProfileStatus::PendingSave => Some(ProfileSelectionRejection::PendingSave),
            SaveProfileStatus::FailedSave => Some(ProfileSelectionRejection::FailedSave),
            SaveProfileStatus::Unsupported => Some(ProfileSelectionRejection::UnsupportedSlot),
        }
    }
}

impl ProfileReadPort for FakeSaveProfileHost {
    fn discover(
        &self,
        request: &ProfileDiscoveryRequest,
    ) -> Result<ProfileDiscovery, ProfileDiscoveryError> {
        context_error(
            &self.instance_id,
            &self.user_data_id,
            &self.baseline,
            request.fence(),
        )
        .map_err(|(_, discovery_error)| discovery_error)?;
        self.discovery()
            .map_err(|_| ProfileDiscoveryError::StaleBaseline)
    }
}

impl ProfileSelectionPort for FakeSaveProfileHost {
    fn selection_availability(&self) -> ProfileSelectionAvailability {
        ProfileSelectionAvailability::SyntheticFixtureOnly
    }

    fn select(
        &mut self,
        request: ProfileSelectionRequest,
    ) -> Result<ProfileSelectionReceipt, ProfileSelectionRejection> {
        self.context_matches(request.fence())?;
        if let Some((original, receipt)) = self.history.get(request.idempotency_key()) {
            if original == &request {
                return Ok(receipt.clone());
            }
            return Err(ProfileSelectionRejection::IdempotencyConflict);
        }
        if self.selection_busy {
            return Err(ProfileSelectionRejection::ConcurrentSelection);
        }
        let slot = self
            .slots
            .get(request.expected_slot())
            .ok_or(ProfileSelectionRejection::UnknownSlot)?;
        if let Some(rejection) = Self::status_rejection(slot.status()) {
            return Err(rejection);
        }
        self.current_selection = Some(request.expected_slot().clone());
        self.mutation_count = self.mutation_count.saturating_add(1);
        let receipt = ProfileSelectionReceipt::new(
            request.idempotency_key().clone(),
            self.discovery()
                .map_err(|_| ProfileSelectionRejection::StaleBaseline)?,
        );
        self.history.insert(
            request.idempotency_key().clone(),
            (request, receipt.clone()),
        );
        Ok(receipt)
    }

    fn reconcile(
        &self,
        idempotency_key: &SelectionIdempotencyKey,
    ) -> Result<ProfileSelectionReceipt, ProfileSelectionRejection> {
        self.history
            .get(idempotency_key)
            .map(|(_, receipt)| receipt.clone())
            .ok_or(ProfileSelectionRejection::UnknownOperation)
    }
}
