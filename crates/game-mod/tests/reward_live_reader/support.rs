// SPDX-License-Identifier: MIT

use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[path = "../support/reward_reference.rs"]
mod static_fixtures;
pub use static_fixtures::*;

use sts2_game_mod::{
    RewardActionKind, RewardCatalog, RewardDefinitionReference, RewardEvidence, RewardField,
    RewardItemReference, RewardKind, RewardLiveAction, RewardLiveActionId,
    RewardLiveCaptureRequest, RewardLiveCollection, RewardLiveGroup, RewardLiveGroupId,
    RewardLiveInstanceId, RewardLiveItem, RewardLiveItemId, RewardLiveOffer, RewardLiveOfferId,
    RewardLiveSelectionReference, RewardLiveSnapshotInput, RewardLiveSource, RewardOfferState,
    RewardQuantity, RewardSemanticReferenceKind, RewardSourceError, RewardUnavailableReason,
    RewardVisibility,
};

#[path = "support/live_snapshot.rs"]
mod live_snapshot;
pub use live_snapshot::base_snapshot;

#[derive(Default)]
pub struct SourceCounters {
    pub reads: AtomicUsize,
    pub claims: AtomicUsize,
    pub rng_evaluations: AtomicUsize,
}

pub struct ScriptedSource {
    queue: Mutex<VecDeque<Result<RewardLiveSnapshotInput, RewardSourceError>>>,
    pub counters: Arc<SourceCounters>,
}

impl ScriptedSource {
    pub fn new(
        responses: impl IntoIterator<Item = Result<RewardLiveSnapshotInput, RewardSourceError>>,
    ) -> (Self, Arc<SourceCounters>) {
        let counters = Arc::new(SourceCounters::default());
        (
            Self {
                queue: Mutex::new(responses.into_iter().collect()),
                counters: Arc::clone(&counters),
            },
            counters,
        )
    }
}

impl RewardLiveSource for ScriptedSource {
    fn capture(
        &self,
        request: &RewardLiveCaptureRequest<'_>,
    ) -> Result<RewardLiveSnapshotInput, RewardSourceError> {
        let _request_fence = (request.catalog(), request.instance_id());
        self.counters.reads.fetch_add(1, Ordering::SeqCst);
        self.queue
            .lock()
            .expect("source queue")
            .pop_front()
            .unwrap_or(Err(RewardSourceError::NoActiveSource))
    }
}

pub fn test_catalog() -> RewardCatalog {
    rich_catalog(&full_manifest())
}

pub fn reader_for(
    catalog: &RewardCatalog,
    responses: impl IntoIterator<Item = Result<RewardLiveSnapshotInput, RewardSourceError>>,
) -> (sts2_game_mod::RewardLiveReader, Arc<SourceCounters>) {
    let (source, counters) = ScriptedSource::new(responses);
    let reader = sts2_game_mod::RewardLiveReader::new(
        source,
        catalog.reader(),
        RewardLiveInstanceId::new("game:main").expect("instance ID"),
    );
    (reader, counters)
}

pub fn offer_query(
    snapshot: sts2_game_mod::RewardLiveSnapshotReference,
    limit: usize,
) -> sts2_game_mod::RewardLiveOfferListQuery {
    sts2_game_mod::RewardLiveOfferListQuery {
        locale: snapshot.catalog_binding().locale.clone(),
        snapshot,
        scope: sts2_game_mod::RewardVisibilityScope::Public,
        limit,
        continuation: None,
    }
}

pub fn live_item(
    catalog: &RewardCatalog,
    id: &str,
    reward_id: &str,
    item_id: &str,
    kind: RewardSemanticReferenceKind,
    content_id: &str,
    quantity: RewardQuantity,
) -> RewardLiveItem {
    RewardLiveItem {
        id: live_item_id(id),
        instance_id: RewardField::Unavailable(RewardUnavailableReason::NotObserved),
        definition: RewardField::Available(RewardItemReference {
            catalog: catalog.binding().clone(),
            reward_id: reward_id.to_owned(),
            item_id: item_id.to_owned(),
        }),
        content: RewardField::Available(reference(kind, content_id)),
        label: text(item_id),
        quantity,
        evidence: RewardEvidence::Observed,
        visibility: RewardVisibility::Visible,
    }
}

pub fn offer(
    id: &str,
    definition: RewardField<RewardDefinitionReference>,
    kind: RewardKind,
    state: RewardOfferState,
    groups: Vec<RewardLiveGroup>,
    items: Vec<RewardLiveItem>,
) -> RewardLiveOffer {
    RewardLiveOffer {
        id: live_offer_id(id),
        definition,
        kind,
        state: RewardField::Available(state),
        visibility: RewardVisibility::Visible,
        groups: RewardLiveCollection::available(groups),
        items: RewardLiveCollection::available(items),
    }
}

pub fn group(
    id: &str,
    selection: RewardField<RewardLiveSelectionReference>,
    items: Vec<&str>,
    actions: Vec<RewardLiveAction>,
) -> RewardLiveGroup {
    RewardLiveGroup {
        id: live_group_id(id),
        selection,
        label: text(id),
        choose_min: RewardField::Available(0),
        choose_max: RewardField::Available(1),
        optional_skip: RewardField::Available(false),
        items: RewardLiveCollection::available(items.into_iter().map(live_item_id).collect()),
        actions: RewardLiveCollection::available(actions),
        visibility: RewardVisibility::Visible,
    }
}

pub fn action(id: &str, kind: RewardActionKind, targets: &[&str]) -> RewardLiveAction {
    RewardLiveAction {
        id: RewardLiveActionId::new(id).expect("action ID"),
        kind,
        label: text(id),
        targets: RewardLiveCollection::available(
            targets.iter().map(|target| live_item_id(target)).collect(),
        ),
        visibility: RewardVisibility::Visible,
    }
}

pub fn selection_ref(catalog: &RewardCatalog, reward_id: &str) -> RewardLiveSelectionReference {
    RewardLiveSelectionReference {
        reward: RewardDefinitionReference {
            catalog: catalog.binding().clone(),
            reward_id: reward_id.to_owned(),
        },
        group_id: "group:pick".to_owned(),
    }
}

pub fn live_offer_id(id: &str) -> RewardLiveOfferId {
    RewardLiveOfferId::new(id).expect("offer ID")
}

pub fn live_group_id(id: &str) -> RewardLiveGroupId {
    RewardLiveGroupId::new(id).expect("group ID")
}

pub fn live_item_id(id: &str) -> RewardLiveItemId {
    RewardLiveItemId::new(id).expect("item ID")
}
