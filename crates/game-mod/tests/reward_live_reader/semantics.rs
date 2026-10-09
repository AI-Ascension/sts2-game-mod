// SPDX-License-Identifier: MIT

use std::sync::atomic::Ordering;

use super::support::*;
use sts2_game_mod::{
    RewardField, RewardFieldStatus, RewardLiveOfferDisposition, RewardOfferState,
    RewardUnavailableReason, RewardVisibilityScope,
};

#[test]
fn capture_retains_multiple_groups_offers_and_pre_acquisition_static_references() {
    let catalog = test_catalog();
    let (mut reader, counters) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let snapshot = reader.capture().expect("capture");
    assert_eq!(snapshot.catalog_binding(), catalog.binding());
    assert_eq!(snapshot.instance_id(), "game:main");
    assert_eq!(snapshot.run_id(), "run:one");
    assert_eq!(snapshot.room_id(), "room:one");
    assert_eq!(snapshot.generations(), (1, 1));
    assert!(reader.retained_source_bytes().expect("retained bytes") > 0);

    let page = reader
        .list_offers(offer_query(snapshot.clone(), 64))
        .expect("retained offer listing");
    assert_eq!(page.total, 6);
    assert_eq!(page.status, RewardFieldStatus::Available);
    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);

    let card_ref = page
        .entries
        .iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
        .expect("card offer")
        .reference
        .clone();
    let card = reader
        .get_offer(&card_ref, RewardVisibilityScope::Public)
        .expect("card detail");
    assert_eq!(card.groups.entries.len(), 2);
    assert_eq!(card.groups.status, RewardFieldStatus::Available);
    assert!(matches!(card.definition, RewardField::Available(_)));
    assert!(matches!(
        reader
            .resolve_definition(&card_ref, RewardVisibilityScope::Public)
            .expect("static reward definition"),
        RewardField::Available(_)
    ));
    let group = &card.groups.entries[0];
    let selection = group.selection.value().expect("observed selection");
    assert_eq!(selection.reference.group_id, "group:pick");
    assert_eq!(selection.definition.group_id, "group:pick");
    assert_eq!(group.choose_min, RewardField::Available(0));
    assert_eq!(group.optional_skip, RewardField::Available(false));
    assert!(
        !group
            .actions
            .entries
            .iter()
            .any(|action| action.kind == sts2_game_mod::RewardActionKind::Skip)
    );
    assert!(
        card.groups.entries[1]
            .actions
            .entries
            .iter()
            .any(|action| action.kind == sts2_game_mod::RewardActionKind::Skip)
    );

    let item = card
        .items
        .entries
        .iter()
        .find(|item| item.reference.item_id().as_str() == "live:item-strike")
        .expect("offered card item");
    assert!(matches!(
        &item.instance,
        RewardField::Unavailable(RewardUnavailableReason::NotObserved)
    ));
    assert!(matches!(&item.definition, RewardField::Available(_)));
    assert!(matches!(
        reader
            .resolve_item_definition(&item.reference, RewardVisibilityScope::Public)
            .expect("pre-acquisition definition"),
        RewardField::Available(_)
    ));
    let action_reference = group.actions.entries[0].reference.clone();
    assert_eq!(
        reader
            .get_action(&action_reference, RewardVisibilityScope::Public)
            .expect("observed choose action")
            .kind,
        sts2_game_mod::RewardActionKind::Choose
    );

    let gold_offers = page
        .entries
        .iter()
        .filter(|entry| entry.kind == sts2_game_mod::RewardKind::Currency)
        .collect::<Vec<_>>();
    assert_eq!(gold_offers.len(), 2);
    assert_ne!(
        gold_offers[0].reference.offer_id(),
        gold_offers[1].reference.offer_id()
    );
    let gold_detail = reader
        .get_offer(&gold_offers[0].reference, RewardVisibilityScope::Public)
        .expect("first currency offer");
    let currency = &gold_detail.items.entries[0];
    assert_eq!(currency.quantity.unit_value(), Some("currency:gold"));
    assert_eq!(
        currency.quantity.base_amount,
        sts2_game_mod::RewardNumericValue::Fixed(25)
    );
    assert_eq!(
        currency.quantity.visible_amount,
        sts2_game_mod::RewardNumericValue::Fixed(40)
    );
    assert_eq!(currency.quantity.modified, RewardField::Available(true));

    let potion = page
        .entries
        .iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-potion-blocked")
        .expect("full potion offer");
    assert_eq!(
        potion.disposition,
        RewardLiveOfferDisposition::BlockedCapacity
    );
    assert_eq!(
        potion.state,
        RewardField::Available(RewardOfferState::BlockedCapacity)
    );
    let potion_detail = reader
        .get_offer(&potion.reference, RewardVisibilityScope::Public)
        .expect("blocked potion detail");
    assert_eq!(
        potion_detail.groups.entries[0].actions.status,
        RewardFieldStatus::Available
    );
    assert!(potion_detail.groups.entries[0].actions.entries.is_empty());
    let special = page
        .entries
        .iter()
        .find(|entry| entry.kind == sts2_game_mod::RewardKind::SpecialGrant)
        .expect("special grant");
    let special_detail = reader
        .get_offer(&special.reference, RewardVisibilityScope::Public)
        .expect("special detail");
    assert_eq!(special_detail.kind, sts2_game_mod::RewardKind::SpecialGrant);
    assert_eq!(
        special_detail.items.entries[0].content,
        RewardField::Unavailable(RewardUnavailableReason::Unsupported),
        "an unbound special content identity is not exposed as a catalog resolution"
    );
    assert!(
        page.entries
            .iter()
            .any(|entry| entry.kind == sts2_game_mod::RewardKind::Relic)
    );

    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);
    assert_eq!(counters.claims.load(Ordering::SeqCst), 0);
    assert_eq!(counters.rng_evaluations.load(Ordering::SeqCst), 0);
}

#[test]
fn empty_unavailable_zero_false_and_withheld_collection_states_stay_distinct() {
    let catalog = test_catalog();
    let mut input = base_snapshot(&catalog);
    let card = &mut input.offers.entries[0];
    card.groups.entries[1].actions =
        sts2_game_mod::RewardLiveCollection::new(RewardFieldStatus::NotObserved, Vec::new());
    card.groups.entries[1].choose_min = RewardField::Available(0);
    card.groups.entries[1].optional_skip = RewardField::Available(false);
    card.items.entries[1].visibility = sts2_game_mod::RewardVisibility::Hidden;
    let gold = &mut input.offers.entries[1];
    gold.state = RewardField::Unavailable(RewardUnavailableReason::Denied);

    let (mut reader, _) = reader_for(&catalog, [Ok(input)]);
    let snapshot = reader.capture().expect("capture");
    let page = reader.list_offers(offer_query(snapshot, 64)).expect("list");
    let card_ref = page
        .entries
        .iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
        .expect("card")
        .reference
        .clone();
    let card = reader
        .get_offer(&card_ref, RewardVisibilityScope::Public)
        .expect("card detail");
    assert_eq!(card.items.status, RewardFieldStatus::Partial);
    assert_eq!(card.items.entries.len(), 1);
    assert_eq!(card.groups.entries.len(), 2);
    assert_eq!(
        card.groups.entries[1].actions.status,
        RewardFieldStatus::NotObserved
    );
    assert!(card.groups.entries[1].actions.entries.is_empty());
    assert_eq!(
        card.groups.entries[1].items.status,
        RewardFieldStatus::Denied
    );
    assert_eq!(card.groups.entries[1].choose_min, RewardField::Available(0));
    assert_eq!(
        card.groups.entries[1].optional_skip,
        RewardField::Available(false)
    );

    let gold_ref = page
        .entries
        .iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-gold-one")
        .expect("currency offer")
        .reference
        .clone();
    let gold = reader
        .get_offer(&gold_ref, RewardVisibilityScope::Public)
        .expect("currency detail");
    assert_eq!(
        gold.disposition,
        RewardLiveOfferDisposition::StateUnavailable
    );
    assert_eq!(
        gold.state,
        RewardField::Unavailable(RewardUnavailableReason::Denied)
    );
    assert_eq!(
        card.groups.entries[0].actions.status,
        RewardFieldStatus::Available
    );
    assert_eq!(
        card.groups.entries[0].actions.entries.len(),
        1,
        "an observed non-empty action list stays available"
    );
}

#[test]
fn owner_only_entries_are_partial_for_public_scope_and_complete_for_owner_scope() {
    let catalog = test_catalog();
    let mut input = base_snapshot(&catalog);
    input.offers.entries[0].items.entries[1].visibility =
        sts2_game_mod::RewardVisibility::OwnerOnly;
    let (mut reader, counters) = reader_for(&catalog, [Ok(input)]);
    let snapshot = reader.capture().expect("capture");
    let mut wrong_locale = offer_query(snapshot.clone(), 64);
    wrong_locale.locale = "fr-FR".to_owned();
    assert!(matches!(
        reader.list_offers(wrong_locale),
        Err(sts2_game_mod::RewardLiveError::Catalog(
            sts2_game_mod::RewardCatalogError::LocaleMismatch
        ))
    ));
    let card = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("public list")
        .entries
        .into_iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
        .expect("card offer");
    let public = reader
        .get_offer(&card.reference, RewardVisibilityScope::Public)
        .expect("public detail");
    assert_eq!(public.items.status, RewardFieldStatus::Partial);
    assert_eq!(public.items.entries.len(), 1);
    let owner = reader
        .get_offer(&card.reference, RewardVisibilityScope::Owner)
        .expect("owner detail");
    assert_eq!(owner.items.status, RewardFieldStatus::Available);
    assert_eq!(owner.items.entries.len(), 2);
    assert_eq!(
        owner.items.entries[1].visibility,
        sts2_game_mod::RewardVisibility::OwnerOnly
    );
    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn custom_unsupported_and_unknown_categories_remain_distinct() {
    let catalog = test_catalog();
    let mut input = base_snapshot(&catalog);
    for (id, kind) in [
        (
            "live:custom",
            sts2_game_mod::RewardKind::Custom("kind:custom".to_owned()),
        ),
        (
            "live:unsupported",
            sts2_game_mod::RewardKind::Unsupported("kind:unsupported".to_owned()),
        ),
        ("live:unknown", sts2_game_mod::RewardKind::Unknown),
    ] {
        input.offers.entries.push(offer(
            id,
            RewardField::Unavailable(RewardUnavailableReason::NotObserved),
            kind,
            RewardOfferState::Unknown,
            Vec::new(),
            Vec::new(),
        ));
    }
    let (mut reader, counters) = reader_for(&catalog, [Ok(input)]);
    let snapshot = reader.capture().expect("capture");
    let page = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("category list");
    let categories = page
        .entries
        .iter()
        .filter(|entry| {
            matches!(
                &entry.kind,
                sts2_game_mod::RewardKind::Custom(_)
                    | sts2_game_mod::RewardKind::Unsupported(_)
                    | sts2_game_mod::RewardKind::Unknown
            )
        })
        .map(|entry| entry.kind.clone())
        .collect::<Vec<_>>();
    assert!(categories.contains(&sts2_game_mod::RewardKind::Custom("kind:custom".to_owned())));
    assert!(categories.contains(&sts2_game_mod::RewardKind::Unsupported(
        "kind:unsupported".to_owned()
    )));
    assert!(categories.contains(&sts2_game_mod::RewardKind::Unknown));
    assert!(
        page.entries
            .iter()
            .filter(|entry| {
                matches!(
                    &entry.kind,
                    sts2_game_mod::RewardKind::Unknown
                        | sts2_game_mod::RewardKind::Custom(_)
                        | sts2_game_mod::RewardKind::Unsupported(_)
                )
            })
            .all(|entry| entry.disposition == RewardLiveOfferDisposition::UnknownState)
    );
    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);
}
