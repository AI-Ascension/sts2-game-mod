// SPDX-License-Identifier: MIT

use super::super::{
    RewardCatalogBinding, RewardDefinitionReference, RewardEvidence, RewardField,
    RewardFieldStatus, RewardItem, RewardItemReference, RewardKind, RewardOfferState,
    RewardQuantity, RewardSelection, RewardSemanticReference, RewardText, RewardVisibility,
};
use super::identity::{
    RewardLiveActionId, RewardLiveGroupId, RewardLiveInstanceId, RewardLiveItemId,
    RewardLiveItemInstanceId, RewardLiveItemReference, RewardLiveOfferId, RewardLiveRoomId,
    RewardLiveRunId, RewardLiveSnapshotId,
};

/// Maximum live offers accepted in one copied observation.
pub const REWARD_LIVE_MAX_OFFERS: usize = 64;
/// Maximum selection groups retained for one live offer.
pub const REWARD_LIVE_MAX_GROUPS_PER_OFFER: usize = 16;
/// Maximum live items retained for one offer.
pub const REWARD_LIVE_MAX_ITEMS_PER_OFFER: usize = 64;
/// Maximum live actions retained for one selection group.
pub const REWARD_LIVE_MAX_ACTIONS_PER_GROUP: usize = 16;
/// Maximum distinct item targets copied for one host-observed action.
pub const REWARD_LIVE_MAX_TARGETS_PER_ACTION: usize = 64;
/// Maximum live offers returned in one page.
pub const REWARD_LIVE_MAX_PAGE_ITEMS: usize = 64;
/// Maximum accounted retained bytes for one live offer and its nested records.
pub const REWARD_LIVE_MAX_DETAIL_BYTES: usize = 128 * 1024;
/// Maximum accounted retained bytes for one complete live snapshot.
pub const REWARD_LIVE_MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;

/// Collection with source completeness preserved independently from its visible entries.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLiveCollection<T> {
    /// Availability and completeness of the source collection.
    pub status: RewardFieldStatus,
    /// Copied records; empty is valid when status is Available.
    pub entries: Vec<T>,
}

impl<T> RewardLiveCollection<T> {
    /// Creates a collection while retaining the source's explicit status.
    #[must_use]
    pub const fn new(status: RewardFieldStatus, entries: Vec<T>) -> Self {
        Self { status, entries }
    }

    /// Creates an observed collection, including an observed empty collection.
    #[must_use]
    pub const fn available(entries: Vec<T>) -> Self {
        Self::new(RewardFieldStatus::Available, entries)
    }
}

/// Owned source capture bound to the exact static catalog and selected instance.
///
/// Offer IDs are unique within this snapshot. Group IDs and item IDs are each unique within
/// their owning offer; action IDs are unique within their owning group. Repeated nested raw IDs
/// under different parents are valid and remain distinct through parent-qualified references.
#[derive(Clone, Debug)]
pub struct RewardLiveSnapshotInput {
    /// Manifest, locale, and producer binding used for every static reference.
    pub catalog: RewardCatalogBinding,
    /// Selected game instance.
    pub instance_id: RewardLiveInstanceId,
    /// Current run identity.
    pub run_id: RewardLiveRunId,
    /// Current room identity.
    pub room_id: RewardLiveRoomId,
    /// Source epoch that changes on a host lifecycle boundary.
    pub epoch: u64,
    /// Source state generation that changes when the visible reward state changes.
    pub state_generation: u64,
    /// Source-owned snapshot identity, separate from every offer and content identity.
    pub snapshot_id: RewardLiveSnapshotId,
    /// Optional source revision, preserving unavailable/withheld states.
    pub source_revision: RewardField<String>,
    /// All copied offers, including explicit empty, partial, or unavailable state.
    pub offers: RewardLiveCollection<RewardLiveOffer>,
}

/// One visible live offer, separate from a static reward definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLiveOffer {
    /// Live offer identity within the copied snapshot.
    pub id: RewardLiveOfferId,
    /// Static reward definition, when the source can bind one.
    pub definition: RewardField<RewardDefinitionReference>,
    /// Observed reward category, including custom, unsupported, and unknown categories.
    pub kind: RewardKind,
    /// Exact observed state; unknown remains explicit.
    pub state: RewardField<RewardOfferState>,
    /// Public, owner-only, hidden, or unknown visibility.
    pub visibility: RewardVisibility,
    /// Current groups; an empty available collection is meaningful.
    pub groups: RewardLiveCollection<RewardLiveGroup>,
    /// Current visible item instances or definitions.
    pub items: RewardLiveCollection<RewardLiveItem>,
}

/// One current live selection group owned by a live offer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLiveGroup {
    /// Live group identity, distinct from the static selection identity.
    pub id: RewardLiveGroupId,
    /// Optional static selection definition reference.
    pub selection: RewardField<RewardLiveSelectionReference>,
    /// Source-observed label.
    pub label: RewardText,
    /// Observed selection cardinality; zero is preserved.
    pub choose_min: RewardField<u32>,
    /// Observed maximum cardinality; zero is preserved.
    pub choose_max: RewardField<u32>,
    /// Whether this live group can be skipped, not inferred from static policy.
    pub optional_skip: RewardField<bool>,
    /// Item identities that this group currently offers.
    pub items: RewardLiveCollection<RewardLiveItemId>,
    /// Host-observed actions only; this reader synthesizes none.
    pub actions: RewardLiveCollection<RewardLiveAction>,
    /// Visibility of this group and its nested actions.
    pub visibility: RewardVisibility,
}

/// One host-observed action identity in one current group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLiveAction {
    /// Live action identity, distinct from static action vocabulary.
    pub id: RewardLiveActionId,
    /// Host-observed action category.
    pub kind: super::super::RewardActionKind,
    /// Source-owned label or explicit unavailable state.
    pub label: RewardText,
    /// Exact target items; an observed empty list stays empty.
    pub targets: RewardLiveCollection<RewardLiveItemId>,
    /// Visibility of this action.
    pub visibility: RewardVisibility,
}

/// One currently visible item in a live offer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLiveItem {
    /// Live item identity scoped by its group and offer references.
    pub id: RewardLiveItemId,
    /// Existing instance identity when the item already has one.
    pub instance_id: RewardField<RewardLiveItemInstanceId>,
    /// Exact static item definition, available before acquisition when supplied.
    pub definition: RewardField<RewardItemReference>,
    /// Typed content identity when exposed by the source.
    pub content: RewardField<RewardSemanticReference>,
    /// Source-owned label or explicit unavailable state.
    pub label: RewardText,
    /// Exact base and visible quantities with units and modification state.
    pub quantity: RewardQuantity,
    /// Evidence label copied from the source.
    pub evidence: RewardEvidence,
    /// Visibility of the item.
    pub visibility: RewardVisibility,
}

/// Static selection identity retained independently from the live group identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardLiveSelectionReference {
    /// Static reward definition owning the selection.
    pub reward: RewardDefinitionReference,
    /// Stable selection group ID from the static catalog.
    pub group_id: String,
}

/// Bounded public detail for one retained live offer.
#[derive(Clone, Debug)]
pub struct RewardLiveOfferDetail {
    /// Reader-scoped offer handle with its full snapshot fence.
    pub reference: super::identity::RewardLiveOfferReference,
    /// Static definition resolution or explicit unavailable reason.
    pub definition: RewardField<super::super::RewardOfferDefinition>,
    /// Current source-observed category.
    pub kind: RewardKind,
    /// Exact source-observed state.
    pub state: RewardField<RewardOfferState>,
    /// Exact source visibility label for this offer.
    pub visibility: RewardVisibility,
    /// Current visible groups and completeness.
    pub groups: RewardLiveCollection<RewardLiveGroupDetail>,
    /// Current visible items and completeness.
    pub items: RewardLiveCollection<RewardLiveItemDetail>,
    /// Source projection of action availability, not an independent host eligibility proof.
    pub disposition: RewardLiveOfferDisposition,
}

/// Current item and action availability classification exposed alongside raw source state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardLiveOfferDisposition {
    /// The host source explicitly reported a capacity-blocked offer.
    BlockedCapacity,
    /// A current selectable action was present in the visible source snapshot.
    ObservedSelectableAction,
    /// No current selectable action was observed for this offer.
    NoObservedSelectableAction,
    /// The host source returned an unknown state.
    UnknownState,
    /// The state itself was explicitly withheld or unavailable.
    StateUnavailable,
}

/// One live group with nested live references.
#[derive(Clone, Debug)]
pub struct RewardLiveGroupDetail {
    /// Reader-scoped group reference.
    pub reference: super::identity::RewardLiveGroupReference,
    /// Resolved static selection identity when available.
    pub selection: RewardField<RewardLiveSelectionDetail>,
    /// Source-observed label and constraints.
    pub label: RewardText,
    /// Exact minimum selection count.
    pub choose_min: RewardField<u32>,
    /// Exact maximum selection count.
    pub choose_max: RewardField<u32>,
    /// Exact optional skip observation.
    pub optional_skip: RewardField<bool>,
    /// Exact source visibility label for this group.
    pub visibility: RewardVisibility,
    /// Group item membership after visibility filtering.
    pub items: RewardLiveCollection<RewardLiveItemReference>,
    /// Host-observed actions after visibility filtering.
    pub actions: RewardLiveCollection<RewardLiveActionDetail>,
}

/// Static selection definition resolved through the retained catalog.
#[derive(Clone, Debug)]
pub struct RewardLiveSelectionDetail {
    /// Exact static selection identity.
    pub reference: RewardLiveSelectionReference,
    /// Static definition remains separate from current live actions.
    pub definition: RewardSelection,
}

/// One live item with its static item definition resolved before acquisition.
#[derive(Clone, Debug)]
pub struct RewardLiveItemDetail {
    /// Reader-scoped item reference.
    pub reference: RewardLiveItemReference,
    /// Optional live instance reference.
    pub instance: RewardField<super::identity::RewardLiveItemInstanceReference>,
    /// Static item resolution or source-supplied unavailable state.
    pub definition: RewardField<RewardItem>,
    /// Typed content reference resolved through a static item when supplied; otherwise unavailable.
    pub content: RewardField<RewardSemanticReference>,
    /// Source-owned presentation and quantity fields.
    pub label: RewardText,
    /// Exact modified quantity state.
    pub quantity: RewardQuantity,
    /// Evidence label copied from source.
    pub evidence: RewardEvidence,
    /// Exact source visibility label for this item.
    pub visibility: RewardVisibility,
}

/// One live action with its exact group and offer ownership path.
#[derive(Clone, Debug)]
pub struct RewardLiveActionDetail {
    /// Reader-scoped action reference.
    pub reference: super::identity::RewardLiveActionReference,
    /// Host-observed action kind and label.
    pub kind: super::super::RewardActionKind,
    /// Exact targets after scope filtering.
    pub targets: RewardLiveCollection<RewardLiveItemReference>,
    /// Action visibility.
    pub visibility: RewardVisibility,
}
