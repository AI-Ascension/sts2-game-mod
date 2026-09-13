// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use sts2_game_mod::{
    CardCost, CardCostSemantics, CardDefinitionReference, CardEffectValue, CardExpiration,
    CardFlags, CardInstanceReference, CardLocation, CardModifier, CardModifierScope, CardOwner,
    CardOwnerKind, CardPile, CardPosition, CardUpgrade, LIVE_CARD_MAX_COST_CONTRIBUTORS,
    LIVE_CARD_MAX_EFFECT_LIST_ITEMS, LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES, LIVE_CARD_MAX_FLAGS,
    LiveCardCollection, LiveCardCollectionStatus, LiveCardError, LiveCardFixture,
    LiveCardProjection, LiveCardQuery, LiveCardReadReference, LiveCardSnapshot, LiveCardStore,
};

fn reference(epoch: u64) -> LiveCardReadReference {
    LiveCardReadReference::new(
        "instance-1",
        "run-1",
        "manifest-1",
        format!("snapshot-{epoch}"),
        epoch,
    )
    .expect("reference")
}

fn projection(read: &LiveCardReadReference, instance_id: &str) -> LiveCardProjection {
    LiveCardProjection {
        instance: CardInstanceReference::new(read.clone(), instance_id).expect("instance"),
        definition: CardDefinitionReference::new("manifest-1", "card:fixture").expect("definition"),
        owner: CardOwner::new(CardOwnerKind::Player, Some("player-1")).expect("owner"),
        location: CardLocation::Pile {
            pile: CardPile::Hand,
            position: CardPosition::Known(0),
        },
        upgrade: CardUpgrade::new(0, Some("base"), None::<String>).expect("upgrade"),
        modifiers: Vec::new(),
        flags: CardFlags::none(),
        effect_parameter_overrides: BTreeMap::new(),
        cost: CardCostSemantics {
            base: CardCost::Fixed(1),
            current: CardCost::Fixed(1),
            effective: CardCost::Fixed(1),
            contributors: Vec::new(),
        },
    }
}

fn snapshot(epoch: u64) -> LiveCardSnapshot {
    let read = reference(epoch);
    LiveCardSnapshot::new(
        read.clone(),
        [
            LiveCardFixture::new(projection(&read, "card-a"), 64),
            LiveCardFixture::new(projection(&read, "card-b"), 64),
        ],
        true,
    )
}

#[test]
fn effect_flags_contributors_and_public_identities_are_bounded() {
    let read = reference(1);
    let mut invalid = projection(&read, "card-a");
    invalid
        .effect_parameter_overrides
        .insert("text".to_owned(), CardEffectValue::Text("\0".to_owned()));
    let rejected = LiveCardStore::new(
        LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true),
        1,
        512,
    );
    assert!(matches!(
        rejected,
        Err(LiveCardError::InvalidProjection("effect_parameter_text"))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.effect_parameter_overrides.insert(
        "text-a".to_owned(),
        CardEffectValue::Text("x".repeat(LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES / 2 + 1)),
    );
    invalid.effect_parameter_overrides.insert(
        "text-b".to_owned(),
        CardEffectValue::Text("x".repeat(LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES / 2 + 1)),
    );
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection(
            "effect override bytes exceed local bound"
        ))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.effect_parameter_overrides.insert(
        "values".to_owned(),
        CardEffectValue::IntegerList(vec![0; LIVE_CARD_MAX_EFFECT_LIST_ITEMS + 1]),
    );
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection(
            "effect integer-list count exceeds local bound"
        ))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.flags.other = (0..=LIVE_CARD_MAX_FLAGS)
        .map(|index| format!("flag-{index}"))
        .collect::<BTreeSet<_>>();
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection(
            "flag count exceeds local bound"
        ))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.cost.contributors = vec![
        sts2_game_mod::CardCostContributor {
            source_ref: "effect:test".to_owned(),
            order: 1,
            scope: CardModifierScope::Turn,
            amount: None,
            value: CardCost::Fixed(1),
            expiration: sts2_game_mod::CardExpiration::Permanent,
        };
        LIVE_CARD_MAX_COST_CONTRIBUTORS + 1
    ];
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection(
            "cost contributor count exceeds local bound"
        ))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.definition.definition_id = "bad id".to_owned();
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection("definition_id"))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.owner.owner_id = Some("bad owner".to_owned());
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection("owner_id"))
    ));

    let mut invalid = projection(&read, "card-a");
    invalid.upgrade.path = Some("bad path".to_owned());
    assert!(matches!(
        LiveCardStore::new(
            LiveCardSnapshot::new(read, [LiveCardFixture::new(invalid, 64)], true,),
            1,
            512,
        ),
        Err(LiveCardError::InvalidProjection("upgrade_path"))
    ));
}

#[test]
fn collection_inventory_and_replacement_continuations_are_explicit() {
    let mut store = LiveCardStore::new(snapshot(1), 1, 512).expect("store");
    assert_eq!(
        store.read_page(&LiveCardQuery::new(
            LiveCardCollection::Selector("missing".to_owned()),
            1,
        )),
        Err(LiveCardError::CollectionUnavailable {
            collection: LiveCardCollection::Selector("missing".to_owned()),
            status: LiveCardCollectionStatus::NotObserved,
        })
    );
    assert_eq!(
        store.read_page(&LiveCardQuery::new(
            LiveCardCollection::Pile(CardPile::Other("native-only".to_owned())),
            1,
        )),
        Err(LiveCardError::CollectionUnavailable {
            collection: LiveCardCollection::Pile(CardPile::Other("native-only".to_owned())),
            status: LiveCardCollectionStatus::NotObserved,
        })
    );

    let explicit = snapshot(1).with_collection_status(
        LiveCardCollection::Pile(CardPile::Other("native-only".to_owned())),
        LiveCardCollectionStatus::Unsupported,
    );
    let mut explicit_store = LiveCardStore::new(explicit, 1, 512).expect("explicit store");
    assert_eq!(
        explicit_store.read_page(&LiveCardQuery::new(
            LiveCardCollection::Pile(CardPile::Other("native-only".to_owned())),
            1,
        )),
        Err(LiveCardError::CollectionUnavailable {
            collection: LiveCardCollection::Pile(CardPile::Other("native-only".to_owned())),
            status: LiveCardCollectionStatus::Unsupported,
        })
    );

    let empty = snapshot(1).with_collection_status(
        LiveCardCollection::Pile(CardPile::Draw),
        LiveCardCollectionStatus::Available,
    );
    let mut empty_store = LiveCardStore::new(empty, 1, 512).expect("empty store");
    let page = empty_store
        .read_page(&LiveCardQuery::new(
            LiveCardCollection::Pile(CardPile::Draw),
            1,
        ))
        .expect("known empty page");
    assert!(page.entries.is_empty());

    let mut replacing = LiveCardStore::new(snapshot(1), 1, 512).expect("replacement store");
    let page = replacing
        .read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1))
        .expect("first page");
    let continuation = page.continuation.expect("continuation");
    replacing
        .replace_snapshot(snapshot(2))
        .expect("new snapshot");
    assert_eq!(
        replacing.read_page(&LiveCardQuery {
            collection: LiveCardCollection::AllVisible,
            limit: 1,
            continuation: Some(continuation),
        }),
        Err(LiveCardError::StaleReference)
    );

    let selector = LiveCardCollection::Selector("reward-1".to_owned());
    let mut old_selector = snapshot(1);
    for (index, fixture) in old_selector.cards.iter_mut().enumerate() {
        fixture.projection.location = sts2_game_mod::CardLocation::Selector {
            selector_id: "reward-1".to_owned(),
            position: sts2_game_mod::CardPosition::Known(index),
        };
    }
    let old_selector =
        old_selector.with_collection_status(selector.clone(), LiveCardCollectionStatus::Available);
    let mut selector_store = LiveCardStore::new(old_selector, 1, 512).expect("selector store");
    let selector_page = selector_store
        .read_page(&LiveCardQuery::new(selector.clone(), 1))
        .expect("selector page");
    let selector_continuation = selector_page.continuation.expect("selector continuation");
    selector_store
        .replace_snapshot(snapshot(2))
        .expect("selector replacement");
    assert_eq!(
        selector_store.read_page(&LiveCardQuery {
            collection: selector,
            limit: 1,
            continuation: Some(selector_continuation),
        }),
        Err(LiveCardError::StaleReference)
    );
}

#[test]
fn measured_projection_size_overrides_untrusted_estimate() {
    let read = reference(1);
    let mut large_text = projection(&read, "text-card");
    large_text.effect_parameter_overrides.insert(
        "text".to_owned(),
        CardEffectValue::Text("x".repeat(LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES)),
    );
    let mut text_store = LiveCardStore::new(
        LiveCardSnapshot::new(read.clone(), [LiveCardFixture::new(large_text, 1)], true),
        1,
        512,
    )
    .expect("large text fixture");
    assert!(matches!(
        text_store.read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1)),
        Err(LiveCardError::DetailTooLarge {
            limit: 512,
            actual
        }) if actual > 512
    ));

    let mut many_modifiers = projection(&read, "modifier-card");
    many_modifiers.modifiers = (0..64)
        .map(|index| CardModifier {
            source_ref: format!("modifier-{index}"),
            order: index,
            scope: CardModifierScope::Turn,
            amount: None,
            value: sts2_game_mod::CardModifierValue::Text("temporary".to_owned()),
            expiration: CardExpiration::EndOfTurn,
        })
        .collect();
    let mut modifier_store = LiveCardStore::new(
        LiveCardSnapshot::new(read, [LiveCardFixture::new(many_modifiers, 1)], true),
        1,
        512,
    )
    .expect("modifier fixture");
    assert!(matches!(
        modifier_store.read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1)),
        Err(LiveCardError::DetailTooLarge {
            limit: 512,
            actual
        }) if actual > 512
    ));
}
