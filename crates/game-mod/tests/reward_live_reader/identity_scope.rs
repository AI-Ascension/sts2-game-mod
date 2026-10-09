// SPDX-License-Identifier: MIT

use super::support::*;
use sts2_game_mod::{RewardActionKind, RewardNumericValue, RewardVisibilityScope};

#[test]
fn repeated_nested_ids_resolve_through_their_emitted_parent_handles() {
    let catalog = test_catalog();
    let mut input = base_snapshot(&catalog);
    let shared_group = live_group_id("live:group:shared");
    let shared_item = live_item_id("live:item:shared");
    let shared_action =
        sts2_game_mod::RewardLiveActionId::new("live:action:shared").expect("shared action ID");

    let card_input = &mut input.offers.entries[0];
    card_input.groups.entries[0].id = shared_group.clone();
    card_input.groups.entries[0].label = text("card group");
    card_input.items.entries[0].id = shared_item.clone();
    card_input.items.entries[0].label = text("card item");
    card_input.items.entries[0].quantity = quantity(None, 3);
    card_input.groups.entries[0].items.entries[0] = shared_item.clone();
    card_input.groups.entries[0].actions.entries[0].id = shared_action.clone();
    card_input.groups.entries[0].actions.entries[0]
        .targets
        .entries[0] = shared_item.clone();

    card_input.groups.entries[1].label = text("card second group");
    card_input.groups.entries[1].actions.entries[0].id = shared_action.clone();
    card_input.groups.entries[1].actions.entries[0].kind = RewardActionKind::Choose;
    card_input.groups.entries[1].actions.entries[0].targets =
        sts2_game_mod::RewardLiveCollection::available(vec![live_item_id("live:item-defend")]);

    let relic_input = &mut input.offers.entries[4];
    relic_input.groups.entries[0].id = shared_group;
    relic_input.groups.entries[0].label = text("relic group");
    relic_input.items.entries[0].id = shared_item.clone();
    relic_input.items.entries[0].label = text("relic item");
    relic_input.items.entries[0].quantity = quantity(None, 7);
    relic_input.groups.entries[0].items.entries[0] = shared_item;
    relic_input.groups.entries[0].actions.entries[0].id = shared_action;
    relic_input.groups.entries[0].actions.entries[0].kind = RewardActionKind::Claim;
    relic_input.groups.entries[0].actions.entries[0].targets =
        sts2_game_mod::RewardLiveCollection::available(Vec::new());

    let (mut reader, counters) = reader_for(&catalog, [Ok(input)]);
    let snapshot = reader.capture().expect("capture repeated scoped IDs");
    let page = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("list repeated scoped IDs");
    let card_reference = page
        .entries
        .iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
        .expect("card offer row")
        .reference
        .clone();
    let relic_reference = page
        .entries
        .iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-relic")
        .expect("relic offer row")
        .reference
        .clone();

    let card_detail = reader
        .get_offer(&card_reference, RewardVisibilityScope::Public)
        .expect("card detail by emitted offer handle");
    let relic_detail = reader
        .get_offer(&relic_reference, RewardVisibilityScope::Public)
        .expect("relic detail by emitted offer handle");
    assert_eq!(card_detail.kind, sts2_game_mod::RewardKind::Card);
    assert_eq!(relic_detail.kind, sts2_game_mod::RewardKind::Relic);
    assert_eq!(
        card_detail
            .definition
            .value()
            .expect("card static definition")
            .reference
            .reward_id,
        "reward:card"
    );
    assert_eq!(
        relic_detail
            .definition
            .value()
            .expect("relic static definition")
            .reference
            .reward_id,
        "reward:relic"
    );
    assert_eq!(
        reader
            .resolve_definition(&card_reference, RewardVisibilityScope::Public)
            .expect("resolve card definition")
            .value()
            .expect("resolved card definition")
            .reference
            .reward_id,
        "reward:card"
    );
    assert_eq!(
        reader
            .resolve_definition(&relic_reference, RewardVisibilityScope::Public)
            .expect("resolve relic definition")
            .value()
            .expect("resolved relic definition")
            .reference
            .reward_id,
        "reward:relic"
    );

    let card_shared_group = card_detail
        .groups
        .entries
        .iter()
        .find(|group| group.label == text("card group"))
        .expect("card shared group");
    let card_second_group = card_detail
        .groups
        .entries
        .iter()
        .find(|group| group.label == text("card second group"))
        .expect("second card group");
    let relic_shared_group = relic_detail
        .groups
        .entries
        .iter()
        .find(|group| group.label == text("relic group"))
        .expect("relic shared group");
    assert_eq!(
        card_shared_group.reference.group_id().as_str(),
        relic_shared_group.reference.group_id().as_str()
    );
    assert_ne!(
        card_shared_group.reference.offer().offer_id(),
        relic_shared_group.reference.offer().offer_id()
    );

    let card_item_reference = card_shared_group.items.entries[0].clone();
    let relic_item_reference = relic_shared_group.items.entries[0].clone();
    assert_eq!(
        card_item_reference.item_id().as_str(),
        relic_item_reference.item_id().as_str()
    );
    let card_item = reader
        .get_item(&card_item_reference, RewardVisibilityScope::Public)
        .expect("card item by emitted parent handle");
    let relic_item = reader
        .get_item(&relic_item_reference, RewardVisibilityScope::Public)
        .expect("relic item by emitted parent handle");
    assert_eq!(card_item.label, text("card item"));
    assert_eq!(relic_item.label, text("relic item"));
    assert_eq!(
        card_item.quantity.visible_amount,
        RewardNumericValue::Fixed(3)
    );
    assert_eq!(
        relic_item.quantity.visible_amount,
        RewardNumericValue::Fixed(7)
    );
    assert_eq!(
        card_item
            .definition
            .value()
            .expect("card item definition")
            .reference
            .item_id,
        "item:strike"
    );
    assert_eq!(
        relic_item
            .definition
            .value()
            .expect("relic item definition")
            .reference
            .item_id,
        "item:relic"
    );
    assert_eq!(
        reader
            .resolve_item_definition(&card_item_reference, RewardVisibilityScope::Public)
            .expect("resolve card item")
            .value()
            .expect("resolved card item")
            .reference
            .item_id,
        "item:strike"
    );
    assert_eq!(
        reader
            .resolve_item_definition(&relic_item_reference, RewardVisibilityScope::Public)
            .expect("resolve relic item")
            .value()
            .expect("resolved relic item")
            .reference
            .item_id,
        "item:relic"
    );

    let card_action = reader
        .get_action(
            &card_shared_group.actions.entries[0].reference,
            RewardVisibilityScope::Public,
        )
        .expect("card action by emitted parent handle");
    let second_card_action = reader
        .get_action(
            &card_second_group.actions.entries[0].reference,
            RewardVisibilityScope::Public,
        )
        .expect("second card action by emitted parent handle");
    let relic_action = reader
        .get_action(
            &relic_shared_group.actions.entries[0].reference,
            RewardVisibilityScope::Public,
        )
        .expect("relic action by emitted parent handle");
    assert_eq!(
        card_action.reference.action_id().as_str(),
        "live:action:shared"
    );
    assert_eq!(
        second_card_action.reference.action_id().as_str(),
        "live:action:shared"
    );
    assert_eq!(
        relic_action.reference.action_id().as_str(),
        "live:action:shared"
    );
    assert_eq!(card_action.kind, RewardActionKind::Choose);
    assert_eq!(second_card_action.kind, RewardActionKind::Choose);
    assert_eq!(relic_action.kind, RewardActionKind::Claim);
    assert_eq!(card_action.targets.entries.len(), 1);
    assert_eq!(
        card_action.targets.entries[0].item_id().as_str(),
        "live:item:shared"
    );
    assert_eq!(
        card_action.targets.entries[0].offer().offer_id().as_str(),
        "live:offer-card"
    );
    assert_eq!(second_card_action.targets.entries.len(), 1);
    assert_eq!(
        second_card_action.targets.entries[0].item_id().as_str(),
        "live:item-defend"
    );
    assert!(relic_action.targets.entries.is_empty());
    assert_eq!(
        relic_action.reference.group().offer().offer_id().as_str(),
        "live:offer-relic"
    );
    assert_eq!(counters.reads.load(std::sync::atomic::Ordering::SeqCst), 1);
    assert_eq!(counters.claims.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert_eq!(
        counters
            .rng_evaluations
            .load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}
