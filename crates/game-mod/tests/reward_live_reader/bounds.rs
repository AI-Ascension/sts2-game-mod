// SPDX-License-Identifier: MIT

use super::support::*;
use sts2_game_mod::{
    RewardEvidence, RewardField, RewardLiveCollection, RewardLiveError, RewardLiveItem,
    RewardLiveItemId, RewardLiveOffer, RewardLiveOfferId, RewardNumericValue, RewardQuantity,
    RewardText, RewardUnavailableReason, RewardVisibility,
};

#[test]
fn duplicate_identities_and_full_binding_mismatches_refuse_the_capture() {
    let catalog = test_catalog();
    let mut duplicate = base_snapshot(&catalog);
    duplicate.offers.entries[1].id = duplicate.offers.entries[0].id.clone();
    let (mut reader, _) = reader_for(&catalog, [Ok(duplicate)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("duplicate_offer_id"))
    ));
    assert!(matches!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    ));

    let mut wrong_binding = base_snapshot(&catalog);
    wrong_binding.catalog.locale = "fr-FR".to_owned();
    let (mut reader, _) = reader_for(&catalog, [Ok(wrong_binding)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::BindingMismatch("catalog"))
    ));

    let mut wrong_instance = base_snapshot(&catalog);
    wrong_instance.instance_id =
        sts2_game_mod::RewardLiveInstanceId::new("game:other").expect("instance ID");
    let (mut reader, _) = reader_for(&catalog, [Ok(wrong_instance)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::BindingMismatch("instance"))
    ));

    let mut duplicate_group = base_snapshot(&catalog);
    let repeated_group_id = duplicate_group.offers.entries[0].groups.entries[0]
        .id
        .clone();
    duplicate_group.offers.entries[0].groups.entries[1].id = repeated_group_id;
    let (mut reader, _) = reader_for(&catalog, [Ok(duplicate_group)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("duplicate_group_id"))
    ));

    let mut duplicate_item = base_snapshot(&catalog);
    let repeated_item_id = duplicate_item.offers.entries[0].items.entries[0].id.clone();
    duplicate_item.offers.entries[0].items.entries[1].id = repeated_item_id;
    let (mut reader, _) = reader_for(&catalog, [Ok(duplicate_item)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("duplicate_item_id"))
    ));

    let mut wrong_action_target = base_snapshot(&catalog);
    let different_group_item_id = wrong_action_target.offers.entries[0].items.entries[1]
        .id
        .clone();
    wrong_action_target.offers.entries[0].groups.entries[0]
        .actions
        .entries[0]
        .targets
        .entries[0] = different_group_item_id;
    let (mut reader, _) = reader_for(&catalog, [Ok(wrong_action_target)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("action_target_membership"))
    ));

    let mut duplicate_action = base_snapshot(&catalog);
    let repeated_action = duplicate_action.offers.entries[0].groups.entries[0]
        .actions
        .entries[0]
        .clone();
    duplicate_action.offers.entries[0].groups.entries[0]
        .actions
        .entries
        .push(repeated_action);
    let (mut reader, _) = reader_for(&catalog, [Ok(duplicate_action)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("duplicate_action_id"))
    ));

    let mut wrong_static_reference = base_snapshot(&catalog);
    if let RewardField::Available(reference) =
        &mut wrong_static_reference.offers.entries[0].definition
    {
        reference.catalog.locale = "fr-FR".to_owned();
    }
    let (mut reader, _) = reader_for(&catalog, [Ok(wrong_static_reference)]);
    assert!(matches!(reader.capture(), Err(RewardLiveError::Catalog(_))));
}

#[test]
fn count_limits_reject_before_retaining_and_never_truncate() {
    let catalog = test_catalog();
    let mut too_many = base_snapshot(&catalog);
    let prototype = too_many.offers.entries[0].clone();
    too_many.offers.entries = (0..=sts2_game_mod::REWARD_LIVE_MAX_OFFERS)
        .map(|index| {
            let mut offer = prototype.clone();
            offer.id = RewardLiveOfferId::new(format!("live:offer-{index}")).expect("offer ID");
            offer
        })
        .collect();
    let (mut reader, _) = reader_for(&catalog, [Ok(too_many)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("offer_count"))
    ));
    assert!(sts2_game_mod::RewardLiveOfferId::new("x".repeat(257)).is_err());
}

#[test]
fn detail_and_snapshot_byte_bounds_fail_closed_without_retaining_old_data() {
    let catalog = test_catalog();
    let mut detail_overflow = base_snapshot(&catalog);
    detail_overflow.offers.entries = vec![large_offer("detail", 9, 16 * 1024)];
    let (mut reader, _) = reader_for(&catalog, [Ok(detail_overflow)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::DetailTooLarge { .. })
    ));
    assert!(matches!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    ));

    let mut aggregate_overflow = base_snapshot(&catalog);
    aggregate_overflow.offers.entries = (0..11)
        .map(|offer| large_offer(&format!("aggregate-{offer}"), 8, 12 * 1024))
        .collect();
    let (mut reader, _) = reader_for(&catalog, [Ok(aggregate_overflow)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::SnapshotTooLarge { .. })
    ));
    assert!(matches!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    ));
}

#[test]
fn source_text_and_formula_input_limits_refuse_without_retaining() {
    let catalog = test_catalog();
    let mut long_text = base_snapshot(&catalog);
    long_text.offers.entries[0].items.entries[0].label =
        RewardText::Available("x".repeat(16 * 1024 + 1));
    let (mut reader, _) = reader_for(&catalog, [Ok(long_text)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::Catalog(
            sts2_game_mod::RewardCatalogError::InvalidInput("item_label")
        ))
    ));
    assert!(matches!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    ));

    let mut too_many_formula_inputs = base_snapshot(&catalog);
    too_many_formula_inputs.offers.entries[0].items.entries[0]
        .quantity
        .base_amount = RewardNumericValue::Formula(sts2_game_mod::RewardFormula {
        rule_reference: "rule:quantity".to_owned(),
        unresolved_inputs: (0..=sts2_game_mod::REWARD_MAX_FORMULA_INPUTS)
            .map(|index| format!("condition:{index}"))
            .collect(),
    });
    let (mut reader, _) = reader_for(&catalog, [Ok(too_many_formula_inputs)]);
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::InvalidInput("formula_input_count"))
    ));
    assert!(matches!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    ));
}

fn large_offer(prefix: &str, items: usize, text_bytes: usize) -> RewardLiveOffer {
    let entries = (0..items)
        .map(|index| RewardLiveItem {
            id: RewardLiveItemId::new(format!("live:item-{prefix}-{index}")).expect("item ID"),
            instance_id: RewardField::Unavailable(RewardUnavailableReason::NotObserved),
            definition: RewardField::Unavailable(RewardUnavailableReason::Unsupported),
            content: RewardField::Unavailable(RewardUnavailableReason::NotObserved),
            label: RewardText::Available("x".repeat(text_bytes)),
            quantity: RewardQuantity {
                unit: RewardField::Unavailable(RewardUnavailableReason::NotApplicable),
                base_amount: RewardNumericValue::Fixed(1),
                visible_amount: RewardNumericValue::Fixed(1),
                modified: RewardField::Available(false),
            },
            evidence: RewardEvidence::Observed,
            visibility: RewardVisibility::Visible,
        })
        .collect();
    RewardLiveOffer {
        id: RewardLiveOfferId::new(format!("live:offer-{prefix}")).expect("offer ID"),
        definition: RewardField::Unavailable(RewardUnavailableReason::NotObserved),
        kind: sts2_game_mod::RewardKind::Custom("custom:grant".to_owned()),
        state: RewardField::Available(sts2_game_mod::RewardOfferState::Offered),
        visibility: RewardVisibility::Visible,
        groups: RewardLiveCollection::available(Vec::new()),
        items: RewardLiveCollection::available(entries),
    }
}
