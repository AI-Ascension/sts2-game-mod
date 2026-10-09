// SPDX-License-Identifier: MIT

use super::support::*;
use sts2_game_mod::{RewardField, RewardLiveError, RewardVisibility, RewardVisibilityScope};

#[test]
fn owner_scope_item_handle_cannot_read_through_public_offer_scope() {
    let catalog = test_catalog();
    let mut input = base_snapshot(&catalog);
    input.offers.entries[0].visibility = RewardVisibility::OwnerOnly;
    let (mut reader, counters) = reader_for(&catalog, [Ok(input)]);
    let snapshot = reader.capture().expect("capture");
    let mut owner_query = offer_query(snapshot, 64);
    owner_query.scope = RewardVisibilityScope::Owner;
    let card_reference = reader
        .list_offers(owner_query)
        .expect("owner listing")
        .entries
        .into_iter()
        .find(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
        .expect("owner-only card offer")
        .reference;
    let owner_detail = reader
        .get_offer(&card_reference, RewardVisibilityScope::Owner)
        .expect("owner offer detail");
    let item_reference = owner_detail.items.entries[0].reference.clone();

    assert!(matches!(
        reader.get_offer(&card_reference, RewardVisibilityScope::Public),
        Err(RewardLiveError::NotFound)
    ));
    assert!(matches!(
        reader.get_item(&item_reference, RewardVisibilityScope::Public),
        Err(RewardLiveError::NotFound)
    ));
    assert!(matches!(
        reader.resolve_item_definition(&item_reference, RewardVisibilityScope::Public),
        Err(RewardLiveError::NotFound)
    ));

    let owner_item = reader
        .get_item(&item_reference, RewardVisibilityScope::Owner)
        .expect("owner item detail");
    assert!(matches!(owner_item.definition, RewardField::Available(_)));
    assert!(matches!(
        reader
            .resolve_item_definition(&item_reference, RewardVisibilityScope::Owner)
            .expect("owner static item definition"),
        RewardField::Available(_)
    ));
    assert_eq!(counters.reads.load(std::sync::atomic::Ordering::SeqCst), 1);
}

#[test]
fn old_item_handles_are_stale_after_hidden_or_unknown_parent_replacement() {
    for visibility in [RewardVisibility::Hidden, RewardVisibility::Unknown] {
        let catalog = test_catalog();
        let first = base_snapshot(&catalog);
        let mut replacement = base_snapshot(&catalog);
        replacement.state_generation = 2;
        replacement.source_revision =
            RewardField::Available("source:visibility-replaced".to_owned());
        replacement.offers.entries[0].visibility = visibility;
        let (mut reader, counters) = reader_for(&catalog, [Ok(first), Ok(replacement)]);
        let first_snapshot = reader.capture().expect("initial capture");
        let first_page = reader
            .list_offers(offer_query(first_snapshot, 64))
            .expect("initial listing");
        let card_reference = first_page
            .entries
            .into_iter()
            .find(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
            .expect("initially visible card offer")
            .reference;
        let card = reader
            .get_offer(&card_reference, RewardVisibilityScope::Public)
            .expect("initial public card detail");
        let item_reference = card.items.entries[0].reference.clone();

        let replacement_snapshot = reader.capture().expect("replacement capture");
        let replacement_page = reader
            .list_offers(offer_query(replacement_snapshot, 64))
            .expect("replacement listing");
        assert!(
            !replacement_page
                .entries
                .iter()
                .any(|entry| entry.reference.offer_id().as_str() == "live:offer-card")
        );
        assert!(matches!(
            reader.get_item(&item_reference, RewardVisibilityScope::Public),
            Err(RewardLiveError::StaleReference)
        ));
        assert!(matches!(
            reader.resolve_item_definition(&item_reference, RewardVisibilityScope::Public),
            Err(RewardLiveError::StaleReference)
        ));
        assert_eq!(counters.reads.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
