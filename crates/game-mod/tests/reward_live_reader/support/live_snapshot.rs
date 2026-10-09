// SPDX-License-Identifier: MIT

use super::*;
use sts2_game_mod::{
    RewardDefinitionReference, RewardKind, RewardLiveRoomId, RewardLiveRunId, RewardLiveSnapshotId,
    RewardOfferState, RewardSemanticReference, RewardSemanticReferenceKind,
};

pub fn base_snapshot(catalog: &RewardCatalog) -> RewardLiveSnapshotInput {
    let card = definition(catalog, "reward:card");
    let gold = definition(catalog, "reward:gold");
    let potion = definition(catalog, "reward:potion");
    let relic = definition(catalog, "reward:relic");
    let card_strike = live_item(
        catalog,
        "live:item-strike",
        "reward:card",
        "item:strike",
        RewardSemanticReferenceKind::Card,
        "card:strike",
        quantity(None, 1),
    );
    let card_defend = live_item(
        catalog,
        "live:item-defend",
        "reward:card",
        "item:defend",
        RewardSemanticReferenceKind::Card,
        "card:defend",
        quantity(None, 1),
    );
    let gold_one = live_item(
        catalog,
        "live:item-gold-one",
        "reward:gold",
        "item:gold",
        RewardSemanticReferenceKind::Currency,
        "currency:gold",
        quantity_pair(Some("currency:gold"), 25, 40),
    );
    let gold_two = live_item(
        catalog,
        "live:item-gold-two",
        "reward:gold",
        "item:gold",
        RewardSemanticReferenceKind::Currency,
        "currency:gold",
        quantity_pair(Some("currency:gold"), 25, 40),
    );
    let potion_item = live_item(
        catalog,
        "live:item-potion",
        "reward:potion",
        "item:potion",
        RewardSemanticReferenceKind::Potion,
        "potion:heal",
        quantity(None, 1),
    );
    let relic_item = live_item(
        catalog,
        "live:item-relic",
        "reward:relic",
        "item:relic",
        RewardSemanticReferenceKind::Relic,
        "relic:shrine",
        quantity(None, 1),
    );
    let special_item = RewardLiveItem {
        id: live_item_id("live:item-special"),
        instance_id: RewardField::Unavailable(RewardUnavailableReason::NotObserved),
        definition: RewardField::Unavailable(RewardUnavailableReason::Unsupported),
        content: RewardField::Available(RewardSemanticReference {
            kind: RewardSemanticReferenceKind::Unknown,
            id: "special:grant-token".to_owned(),
            label: text("Special grant"),
        }),
        label: text("Special grant"),
        quantity: quantity(None, 1),
        evidence: RewardEvidence::Observed,
        visibility: RewardVisibility::Visible,
    };
    RewardLiveSnapshotInput {
        catalog: catalog.binding().clone(),
        instance_id: RewardLiveInstanceId::new("game:main").expect("instance ID"),
        run_id: RewardLiveRunId::new("run:one").expect("run ID"),
        room_id: RewardLiveRoomId::new("room:one").expect("room ID"),
        epoch: 1,
        state_generation: 1,
        snapshot_id: RewardLiveSnapshotId::new("snapshot:repeat").expect("snapshot ID"),
        source_revision: RewardField::Available("source:revision-one".to_owned()),
        offers: RewardLiveCollection::available(vec![
            offer(
                "live:offer-card",
                RewardField::Available(card),
                RewardKind::Card,
                RewardOfferState::Offered,
                vec![
                    group(
                        "live:group-card-a",
                        RewardField::Available(selection_ref(catalog, "reward:card")),
                        vec!["live:item-strike"],
                        vec![action(
                            "live:action-card-a",
                            RewardActionKind::Choose,
                            &["live:item-strike"],
                        )],
                    ),
                    group(
                        "live:group-card-b",
                        RewardField::Unavailable(RewardUnavailableReason::NotObserved),
                        vec!["live:item-defend"],
                        vec![action("live:action-card-skip", RewardActionKind::Skip, &[])],
                    ),
                ],
                vec![card_strike, card_defend],
            ),
            offer(
                "live:offer-gold-one",
                RewardField::Available(gold.clone()),
                RewardKind::Currency,
                RewardOfferState::Offered,
                vec![group(
                    "live:group-gold-one",
                    RewardField::Available(selection_ref(catalog, "reward:gold")),
                    vec!["live:item-gold-one"],
                    vec![action("live:action-gold-one", RewardActionKind::Claim, &[])],
                )],
                vec![gold_one],
            ),
            offer(
                "live:offer-gold-two",
                RewardField::Available(gold),
                RewardKind::Currency,
                RewardOfferState::Offered,
                vec![group(
                    "live:group-gold-two",
                    RewardField::Available(selection_ref(catalog, "reward:gold")),
                    vec!["live:item-gold-two"],
                    vec![action("live:action-gold-two", RewardActionKind::Claim, &[])],
                )],
                vec![gold_two],
            ),
            offer(
                "live:offer-potion-blocked",
                RewardField::Available(potion),
                RewardKind::Potion,
                RewardOfferState::BlockedCapacity,
                vec![group(
                    "live:group-potion",
                    RewardField::Available(selection_ref(catalog, "reward:potion")),
                    vec!["live:item-potion"],
                    Vec::new(),
                )],
                vec![potion_item],
            ),
            offer(
                "live:offer-relic",
                RewardField::Available(relic),
                RewardKind::Relic,
                RewardOfferState::Offered,
                vec![group(
                    "live:group-relic",
                    RewardField::Available(selection_ref(catalog, "reward:relic")),
                    vec!["live:item-relic"],
                    vec![action(
                        "live:action-relic",
                        RewardActionKind::Choose,
                        &["live:item-relic"],
                    )],
                )],
                vec![relic_item],
            ),
            offer(
                "live:offer-special",
                RewardField::Unavailable(RewardUnavailableReason::NotObserved),
                RewardKind::SpecialGrant,
                RewardOfferState::Offered,
                vec![group(
                    "live:group-special",
                    RewardField::Unavailable(RewardUnavailableReason::NotObserved),
                    vec!["live:item-special"],
                    vec![action("live:action-special", RewardActionKind::Claim, &[])],
                )],
                vec![special_item],
            ),
        ]),
    }
}

fn definition(catalog: &RewardCatalog, reward_id: &str) -> RewardDefinitionReference {
    RewardDefinitionReference {
        catalog: catalog.binding().clone(),
        reward_id: reward_id.to_owned(),
    }
}
