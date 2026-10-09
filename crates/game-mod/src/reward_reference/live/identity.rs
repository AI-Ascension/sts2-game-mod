// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::RewardCatalogBinding;
use super::super::model::{REWARD_MAX_IDENTITY_BYTES, validate_identity};
use super::error::RewardLiveError;

#[derive(Debug)]
pub(super) struct ReaderScope {
    // A non-zero-sized allocation gives each live reader a distinct Arc allocation identity.
    pub(super) _scope: u8,
}

macro_rules! live_id {
    ($name:ident, $field:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            /// Validates one opaque live identity in its own namespace.
            pub fn new(value: impl Into<String>) -> Result<Self, RewardLiveError> {
                let value = value.into();
                validate_identity(&value, $field).map_err(RewardLiveError::Catalog)?;
                if value.capacity() > REWARD_MAX_IDENTITY_BYTES {
                    return Err(RewardLiveError::InvalidInput($field));
                }
                Ok(Self(value))
            }

            /// Returns the source identity without changing its namespace.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub(super) fn capacity(&self) -> usize {
                self.0.capacity()
            }
        }
    };
}

live_id!(RewardLiveSnapshotId, "live_snapshot_id");
live_id!(RewardLiveInstanceId, "live_instance_id");
live_id!(RewardLiveRunId, "live_run_id");
live_id!(RewardLiveRoomId, "live_room_id");
live_id!(RewardLiveOfferId, "live_offer_id");
live_id!(RewardLiveGroupId, "live_group_id");
live_id!(RewardLiveItemId, "live_item_id");
live_id!(RewardLiveActionId, "live_action_id");
live_id!(RewardLiveItemInstanceId, "live_item_instance_id");

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RewardLiveSnapshotFence {
    pub(super) catalog: RewardCatalogBinding,
    pub(super) instance_id: RewardLiveInstanceId,
    pub(super) run_id: RewardLiveRunId,
    pub(super) room_id: RewardLiveRoomId,
    pub(super) epoch: u64,
    pub(super) state_generation: u64,
    pub(super) snapshot_id: RewardLiveSnapshotId,
    pub(super) source_revision: super::super::RewardField<String>,
}

/// Reader-scoped reference to one exact copied live snapshot.
#[derive(Clone, Debug)]
pub struct RewardLiveSnapshotReference {
    pub(super) fence: Arc<RewardLiveSnapshotFence>,
    pub(super) scope: Arc<ReaderScope>,
    pub(super) reader_generation: u64,
}

impl RewardLiveSnapshotReference {
    pub(super) fn new(
        fence: Arc<RewardLiveSnapshotFence>,
        scope: Arc<ReaderScope>,
        reader_generation: u64,
    ) -> Self {
        Self {
            fence,
            scope,
            reader_generation,
        }
    }

    /// Returns the exact static catalog generation and locale used by the capture.
    #[must_use]
    pub fn catalog_binding(&self) -> &RewardCatalogBinding {
        &self.fence.catalog
    }

    /// Returns the selected game instance identity.
    #[must_use]
    pub fn instance_id(&self) -> &str {
        self.fence.instance_id.as_str()
    }

    /// Returns the observed run identity.
    #[must_use]
    pub fn run_id(&self) -> &str {
        self.fence.run_id.as_str()
    }

    /// Returns the observed room identity.
    #[must_use]
    pub fn room_id(&self) -> &str {
        self.fence.room_id.as_str()
    }

    /// Returns the observed source epoch and state generation.
    #[must_use]
    pub fn generations(&self) -> (u64, u64) {
        (self.fence.epoch, self.fence.state_generation)
    }

    /// Returns the source snapshot identity.
    #[must_use]
    pub fn snapshot_id(&self) -> &RewardLiveSnapshotId {
        &self.fence.snapshot_id
    }

    /// Returns the source revision value or its explicit unavailable reason.
    #[must_use]
    pub fn source_revision(&self) -> &super::super::RewardField<String> {
        &self.fence.source_revision
    }
}

/// Live offer identity scoped to a snapshot and one reader instance.
#[derive(Clone, Debug)]
pub struct RewardLiveOfferReference {
    pub(super) snapshot: RewardLiveSnapshotReference,
    pub(super) id: RewardLiveOfferId,
}

impl RewardLiveOfferReference {
    pub(super) fn new(snapshot: RewardLiveSnapshotReference, id: RewardLiveOfferId) -> Self {
        Self { snapshot, id }
    }

    /// Returns the owning live snapshot fence.
    #[must_use]
    pub fn snapshot(&self) -> &RewardLiveSnapshotReference {
        &self.snapshot
    }

    /// Returns the offer identity in its live namespace.
    #[must_use]
    pub fn offer_id(&self) -> &RewardLiveOfferId {
        &self.id
    }
}

/// Live group identity scoped to its exact offer.
#[derive(Clone, Debug)]
pub struct RewardLiveGroupReference {
    pub(super) offer: RewardLiveOfferReference,
    pub(super) id: RewardLiveGroupId,
}

impl RewardLiveGroupReference {
    pub(super) fn new(offer: RewardLiveOfferReference, id: RewardLiveGroupId) -> Self {
        Self { offer, id }
    }

    /// Returns the owning live offer.
    #[must_use]
    pub fn offer(&self) -> &RewardLiveOfferReference {
        &self.offer
    }

    /// Returns the live group identity.
    #[must_use]
    pub fn group_id(&self) -> &RewardLiveGroupId {
        &self.id
    }
}

/// Live item identity scoped to its offer and optional group association.
#[derive(Clone, Debug)]
pub struct RewardLiveItemReference {
    pub(super) offer: RewardLiveOfferReference,
    pub(super) group: Option<RewardLiveGroupReference>,
    pub(super) id: RewardLiveItemId,
}

impl RewardLiveItemReference {
    pub(super) fn new(
        offer: RewardLiveOfferReference,
        group: Option<RewardLiveGroupReference>,
        id: RewardLiveItemId,
    ) -> Self {
        Self { offer, group, id }
    }

    /// Returns the owning live offer.
    #[must_use]
    pub fn offer(&self) -> &RewardLiveOfferReference {
        &self.offer
    }

    /// Returns the optional group association; unconditional items have no group.
    #[must_use]
    pub fn group(&self) -> Option<&RewardLiveGroupReference> {
        self.group.as_ref()
    }

    /// Returns the live item identity.
    #[must_use]
    pub fn item_id(&self) -> &RewardLiveItemId {
        &self.id
    }
}

/// Live action identity scoped to its exact offer and group.
#[derive(Clone, Debug)]
pub struct RewardLiveActionReference {
    pub(super) group: RewardLiveGroupReference,
    pub(super) id: RewardLiveActionId,
}

impl RewardLiveActionReference {
    pub(super) fn new(group: RewardLiveGroupReference, id: RewardLiveActionId) -> Self {
        Self { group, id }
    }

    /// Returns the owning live group.
    #[must_use]
    pub fn group(&self) -> &RewardLiveGroupReference {
        &self.group
    }

    /// Returns the live action identity.
    #[must_use]
    pub fn action_id(&self) -> &RewardLiveActionId {
        &self.id
    }
}

/// Live identity of an item instance contained in one offer.
#[derive(Clone, Debug)]
pub struct RewardLiveItemInstanceReference {
    /// Owning offer and full snapshot fence.
    pub offer: RewardLiveOfferReference,
    /// Instance identity, distinct from a static content definition.
    pub item_instance_id: RewardLiveItemInstanceId,
}
