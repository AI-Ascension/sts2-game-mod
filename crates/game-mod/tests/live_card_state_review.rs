// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use sts2_game_mod::{
    CardCost, CardCostSemantics, CardDefinitionReference, CardEffectValue, CardFlags,
    CardInstanceReference, CardLocation, CardModifierScope, CardOwner, CardOwnerKind, CardPile,
    CardPosition, CardUpgrade, LIVE_CARD_MAX_COST_CONTRIBUTORS, LIVE_CARD_MAX_EFFECT_LIST_ITEMS,
    LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES, LIVE_CARD_MAX_FLAGS, LiveCardCollection,
    LiveCardCollectionStatus, LiveCardError, LiveCardFixture, LiveCardProjection, LiveCardQuery,
    LiveCardReadReference, LiveCardSnapshot, LiveCardStore,
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
        "text".to_owned(),
        CardEffectValue::Text("x".repeat(LIVE_CARD_MAX_EFFECT_OVERRIDE_BYTES)),
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
}
