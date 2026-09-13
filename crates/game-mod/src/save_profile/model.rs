// SPDX-License-Identifier: MIT

use super::error::ProfileFixtureError;
use super::{
    BaselineFence, HostCompatibility, SAVE_PROFILE_MAX_SLOTS, SaveProfileBaseline, SaveSlotId,
    SelectionAuthority, SelectionIdempotencyKey,
};
use std::collections::BTreeSet;

/// Current owner-local status of a save slot, with no host error or payload detail.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveProfileStatus {
    /// The slot can be selected by an admitted owner operation.
    Available,
    /// The slot has an active run and cannot be selected.
    ActiveRun,
    /// Another owner operation currently uses the slot.
    InUse,
    /// A save is pending and selection is not safe.
    PendingSave,
    /// The owner reported a failed save; no retry is inferred.
    FailedSave,
    /// The host does not support this slot for the selected compatibility.
    Unsupported,
}

/// Read-only summary with payload, path, account, and raw-error details omitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveSlotSummary {
    slot_id: SaveSlotId,
    host_compatibility: HostCompatibility,
    status: SaveProfileStatus,
    baseline: SaveProfileBaseline,
}

impl SaveSlotSummary {
    /// Creates one bounded redacted summary.
    #[must_use]
    pub const fn new(
        slot_id: SaveSlotId,
        host_compatibility: HostCompatibility,
        status: SaveProfileStatus,
        baseline: SaveProfileBaseline,
    ) -> Self {
        Self {
            slot_id,
            host_compatibility,
            status,
            baseline,
        }
    }

    /// Returns the opaque slot identity.
    #[must_use]
    pub const fn slot_id(&self) -> &SaveSlotId {
        &self.slot_id
    }

    /// Returns the bounded compatibility witness.
    #[must_use]
    pub const fn host_compatibility(&self) -> &HostCompatibility {
        &self.host_compatibility
    }

    /// Returns the explicit selection status.
    #[must_use]
    pub const fn status(&self) -> SaveProfileStatus {
        self.status
    }

    /// Returns the freshness/baseline witness without exposing save contents.
    #[must_use]
    pub const fn baseline(&self) -> &SaveProfileBaseline {
        &self.baseline
    }
}

/// Explicit context for an owner-local discovery read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileDiscoveryRequest {
    fence: BaselineFence,
}

impl ProfileDiscoveryRequest {
    /// Creates a discovery request with explicit instance and baseline identity.
    #[must_use]
    pub const fn new(fence: BaselineFence) -> Self {
        Self { fence }
    }

    /// Returns the requested identity fence.
    #[must_use]
    pub const fn fence(&self) -> &BaselineFence {
        &self.fence
    }
}

/// Bounded current-selection read and slot-summary result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileDiscovery {
    fence: BaselineFence,
    slots: Vec<SaveSlotSummary>,
    current_selection: Option<SaveSlotId>,
}

impl ProfileDiscovery {
    /// Creates a deterministic discovery result after validating fixture invariants.
    pub fn new(
        fence: BaselineFence,
        mut slots: Vec<SaveSlotSummary>,
        current_selection: Option<SaveSlotId>,
    ) -> Result<Self, ProfileFixtureError> {
        if slots.len() > SAVE_PROFILE_MAX_SLOTS {
            return Err(ProfileFixtureError::TooManySlots);
        }
        let mut identities = BTreeSet::new();
        for slot in &slots {
            if slot.baseline() != fence.baseline() {
                return Err(ProfileFixtureError::BaselineMismatch);
            }
            if !identities.insert(slot.slot_id().clone()) {
                return Err(ProfileFixtureError::DuplicateSlot);
            }
        }
        slots.sort_by(|left, right| left.slot_id().cmp(right.slot_id()));
        if let Some(selected) = &current_selection
            && !identities.contains(selected)
        {
            return Err(ProfileFixtureError::SelectionNotListed);
        }
        Ok(Self {
            fence,
            slots,
            current_selection,
        })
    }

    /// Returns the identity/freshness fence used for this read.
    #[must_use]
    pub const fn fence(&self) -> &BaselineFence {
        &self.fence
    }

    /// Returns redacted summaries in deterministic slot identity order.
    #[must_use]
    pub fn slots(&self) -> &[SaveSlotSummary] {
        &self.slots
    }

    /// Returns the authoritative current selection, if the host supplied one.
    #[must_use]
    pub const fn current_selection(&self) -> Option<&SaveSlotId> {
        self.current_selection.as_ref()
    }
}

/// Explicit request for one host-thread selection operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSelectionRequest {
    fence: BaselineFence,
    authority: SelectionAuthority,
    expected_slot: SaveSlotId,
    idempotency_key: SelectionIdempotencyKey,
}

impl ProfileSelectionRequest {
    /// Creates a request that cannot select an implicit default slot.
    #[must_use]
    pub const fn new(
        fence: BaselineFence,
        authority: SelectionAuthority,
        expected_slot: SaveSlotId,
        idempotency_key: SelectionIdempotencyKey,
    ) -> Self {
        Self {
            fence,
            authority,
            expected_slot,
            idempotency_key,
        }
    }

    /// Returns the request's instance and baseline fence.
    #[must_use]
    pub const fn fence(&self) -> &BaselineFence {
        &self.fence
    }

    /// Returns the explicit selection authority witness.
    #[must_use]
    pub const fn authority(&self) -> &SelectionAuthority {
        &self.authority
    }

    /// Returns the only slot this request may select.
    #[must_use]
    pub const fn expected_slot(&self) -> &SaveSlotId {
        &self.expected_slot
    }

    /// Returns the durable operation key used for reconciliation.
    #[must_use]
    pub const fn idempotency_key(&self) -> &SelectionIdempotencyKey {
        &self.idempotency_key
    }
}

/// Authoritative post-operation readback for one selection operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSelectionReceipt {
    idempotency_key: SelectionIdempotencyKey,
    readback: ProfileDiscovery,
}

impl ProfileSelectionReceipt {
    pub(super) const fn new(
        idempotency_key: SelectionIdempotencyKey,
        readback: ProfileDiscovery,
    ) -> Self {
        Self {
            idempotency_key,
            readback,
        }
    }

    /// Returns the operation key retained for lost-response reconciliation.
    #[must_use]
    pub const fn idempotency_key(&self) -> &SelectionIdempotencyKey {
        &self.idempotency_key
    }

    /// Returns the authoritative current-selection readback.
    #[must_use]
    pub const fn readback(&self) -> &ProfileDiscovery {
        &self.readback
    }
}
