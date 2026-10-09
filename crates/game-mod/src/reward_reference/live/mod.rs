// SPDX-License-Identifier: MIT

//! Source-only retained reads over one copied live reward observation.
//!
//! The source port owns host reads and must return detached values. Reader queries use only the
//! retained snapshot and existing static catalog; they never recapture, claim, or evaluate RNG.

mod detail;
mod error;
mod identity;
mod measure;
mod page;
mod projection;
mod query;
mod reader;
mod snapshot;
mod source;
mod validation;

pub use error::RewardLiveError;
pub use identity::{
    RewardLiveActionId, RewardLiveActionReference, RewardLiveGroupId, RewardLiveGroupReference,
    RewardLiveInstanceId, RewardLiveItemId, RewardLiveItemInstanceId,
    RewardLiveItemInstanceReference, RewardLiveItemReference, RewardLiveOfferId,
    RewardLiveOfferReference, RewardLiveRoomId, RewardLiveRunId, RewardLiveSnapshotId,
    RewardLiveSnapshotReference,
};
pub use page::{
    RewardLiveContinuation, RewardLiveOfferEntry, RewardLiveOfferListQuery, RewardLiveOfferPage,
};
pub use reader::{RewardLiveInvalidation, RewardLiveReader};
pub use snapshot::{
    REWARD_LIVE_MAX_ACTIONS_PER_GROUP, REWARD_LIVE_MAX_DETAIL_BYTES,
    REWARD_LIVE_MAX_GROUPS_PER_OFFER, REWARD_LIVE_MAX_ITEMS_PER_OFFER, REWARD_LIVE_MAX_OFFERS,
    REWARD_LIVE_MAX_PAGE_ITEMS, REWARD_LIVE_MAX_SNAPSHOT_BYTES, REWARD_LIVE_MAX_TARGETS_PER_ACTION,
    RewardLiveAction, RewardLiveActionDetail, RewardLiveCollection, RewardLiveGroup,
    RewardLiveGroupDetail, RewardLiveItem, RewardLiveItemDetail, RewardLiveOffer,
    RewardLiveOfferDetail, RewardLiveOfferDisposition, RewardLiveSelectionDetail,
    RewardLiveSelectionReference, RewardLiveSnapshotInput,
};
pub use source::{RewardLiveCaptureRequest, RewardLiveSource};
