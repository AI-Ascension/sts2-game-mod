// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::collections::{BTreeMap, BTreeSet};

use sts2_game_mod::live_card_state::{CardCost, CardEffectValue};
use sts2_game_mod::{
    CardCostAmount, CardCostContributor, CardCostSemantics, CardCostUnknownReason,
    CardDefinitionReference, CardExpiration, CardFlags, CardInstanceReference, CardLocation,
    CardModifier, CardModifierScope, CardModifierValue, CardOwner, CardOwnerKind, CardPile,
    CardPosition, CardUpgrade, FixtureLiveCardSource, LiveCardCapability, LiveCardCollection,
    LiveCardError, LiveCardField, LiveCardPageCompleteness, LiveCardProjection, LiveCardQuery,
    LiveCardReadReference, LiveCardSnapshot, LiveCardSource, LiveCardStore,
    UnavailableLiveCardSource,
};

fn reference(epoch: u64) -> LiveCardReadReference {
    LiveCardReadReference::new(
        "instance-1",
        "run-1",
        "manifest-1",
        format!("snapshot-{epoch}"),
        epoch,
    )
    .expect("fixture identity")
}

fn projection(
    read: &LiveCardReadReference,
    instance_id: &str,
    location: CardLocation,
    upgrade: CardUpgrade,
    cost: CardCostSemantics,
) -> LiveCardProjection {
    LiveCardProjection {
        instance: CardInstanceReference::new(read.clone(), instance_id).expect("instance identity"),
        definition: CardDefinitionReference::new("manifest-1", "base:ironclad:strike")
            .expect("definition identity"),
        owner: CardOwner::new(CardOwnerKind::Player, Some("player-1")).expect("owner"),
        location,
        upgrade,
        modifiers: vec![CardModifier {
            source_ref: "effect:temporary-cost".to_owned(),
            order: 1,
            scope: CardModifierScope::Turn,
            amount: Some(-1),
            value: CardModifierValue::Integer(-1),
            expiration: CardExpiration::EndOfTurn,
        }],
        flags: CardFlags {
            retained: false,
            exhaust: false,
            ethereal: false,
            other: BTreeSet::new(),
        },
        effect_parameter_overrides: BTreeMap::from([(
            "damage".to_owned(),
            CardEffectValue::Integer(6),
        )]),
        cost,
    }
}

fn fixed_cost() -> CardCostSemantics {
    CardCostSemantics {
        base: CardCost::Fixed(1),
        current: CardCost::Fixed(0),
        effective: CardCost::Fixed(0),
        contributors: vec![CardCostContributor {
            source_ref: "effect:temporary-cost".to_owned(),
            order: 1,
            scope: CardModifierScope::Turn,
            amount: Some(-1),
            value: CardCost::Fixed(0),
            expiration: CardExpiration::EndOfTurn,
        }],
    }
}

fn alternate_cost() -> CardCostSemantics {
    CardCostSemantics {
        base: CardCost::Fixed(1),
        current: CardCost::Fixed(1),
        effective: CardCost::AlternateResource {
            resource: "blood".to_owned(),
            amount: CardCostAmount::Fixed(2),
        },
        contributors: vec![CardCostContributor {
            source_ref: "relic:alternate-payment".to_owned(),
            order: 1,
            scope: CardModifierScope::Combat,
            amount: None,
            value: CardCost::AlternateResource {
                resource: "blood".to_owned(),
                amount: CardCostAmount::Fixed(2),
            },
            expiration: CardExpiration::EndOfCombat,
        }],
    }
}

fn snapshot(epoch: u64) -> LiveCardSnapshot {
    let read = reference(epoch);
    let first = projection(
        &read,
        "card-instance-a",
        CardLocation::Pile {
            pile: CardPile::Hand,
            position: CardPosition::Known(0),
        },
        CardUpgrade::new(0, Some("base"), None::<String>).expect("base upgrade"),
        fixed_cost(),
    );
    let mut second = projection(
        &read,
        "card-instance-b",
        CardLocation::Selector {
            selector_id: "reward-1".to_owned(),
            position: CardPosition::Known(1),
        },
        CardUpgrade::new(1, Some("plus"), Some("path-a")).expect("upgraded card"),
        alternate_cost(),
    );
    second.owner = CardOwner::new(CardOwnerKind::Selector, Some("reward-1")).expect("selector");
    second.flags.retained = true;
    second.modifiers = vec![CardModifier {
        source_ref: "enchant:retain".to_owned(),
        order: 1,
        scope: CardModifierScope::Combat,
        amount: None,
        value: CardModifierValue::Marker,
        expiration: CardExpiration::EndOfCombat,
    }];
    LiveCardSnapshot::new(
        read,
        [
            sts2_game_mod::LiveCardFixture::new(first, 128),
            sts2_game_mod::LiveCardFixture::new(second, 160),
        ],
        true,
    )
}

fn store() -> LiveCardStore {
    LiveCardStore::new(snapshot(7), 1, 512).expect("fixture store")
}

#[test]
fn same_definition_instances_keep_distinct_upgrade_modifier_and_cost_state() {
    let mut store = store();
    let first = store
        .read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1))
        .expect("first page");
    assert_eq!(first.completeness, LiveCardPageCompleteness::Partial);
    assert_eq!(first.entries[0].instance_id(), "card-instance-a");
    assert_eq!(
        first.entries[0].definition_ref(),
        &CardDefinitionReference::new("manifest-1", "base:ironclad:strike").expect("definition")
    );
    let continuation = first.continuation.clone().expect("continuation");
    let second = store
        .read_page(&LiveCardQuery {
            collection: LiveCardCollection::AllVisible,
            limit: 1,
            continuation: Some(continuation),
        })
        .expect("second page");
    assert_eq!(second.completeness, LiveCardPageCompleteness::Complete);
    assert_eq!(second.entries[0].instance_id(), "card-instance-b");
    assert_ne!(first.entries[0].upgrade, second.entries[0].upgrade);
    assert_ne!(first.entries[0].cost, second.entries[0].cost);
    assert_ne!(first.entries[0].modifiers, second.entries[0].modifiers);
}

#[test]
fn selector_pages_and_stable_instance_detail_cover_cards_outside_hand() {
    let mut store = store();
    let page = store
        .read_page(&LiveCardQuery::new(
            LiveCardCollection::Selector("reward-1".to_owned()),
            1,
        ))
        .expect("selector page");
    assert_eq!(page.entries.len(), 1);
    let instance = page.entries[0].instance.clone();
    let detail = store
        .read_detail(
            &instance,
            &[
                LiveCardField::Location,
                LiveCardField::Upgrade,
                LiveCardField::Cost,
            ],
        )
        .expect("selector detail");
    assert_eq!(detail.fields.len(), 3);
    assert_eq!(detail.reference, reference(7));
    assert_eq!(
        store
            .read_card(&instance)
            .expect("complete detail")
            .instance,
        instance
    );
}

#[test]
fn temporary_cost_contributor_can_expire_without_coercing_special_costs() {
    let mut store = store();
    let hand = store
        .read_page(&LiveCardQuery::new(
            LiveCardCollection::Pile(CardPile::Hand),
            1,
        ))
        .expect("hand page");
    assert_eq!(hand.entries[0].cost.current, CardCost::Fixed(0));
    assert!(matches!(
        hand.entries[0].cost.contributors[0].expiration,
        CardExpiration::EndOfTurn
    ));

    let mut next = snapshot(8);
    next.cards[0].projection.instance.read = reference(8);
    next.cards[0].projection.modifiers.clear();
    next.cards[0].projection.cost = CardCostSemantics {
        base: CardCost::Fixed(1),
        current: CardCost::Fixed(1),
        effective: CardCost::Fixed(1),
        contributors: Vec::new(),
    };
    store.replace_snapshot(next).expect("new turn");
    let current = store
        .read_page(&LiveCardQuery::new(
            LiveCardCollection::Pile(CardPile::Hand),
            1,
        ))
        .expect("new hand page");
    assert_eq!(current.entries[0].cost.current, CardCost::Fixed(1));
    assert_eq!(current.entries[0].cost.effective, CardCost::Fixed(1));
}

#[test]
fn special_and_unresolved_cost_shapes_survive_projection_validation() {
    let read = reference(7);
    let cost = CardCostSemantics {
        base: CardCost::X,
        current: CardCost::Free,
        effective: CardCost::Unplayable,
        contributors: vec![CardCostContributor {
            source_ref: "rule:dynamic-cost".to_owned(),
            order: 1,
            scope: CardModifierScope::Combat,
            amount: None,
            value: CardCost::AlternateResource {
                resource: "charge".to_owned(),
                amount: CardCostAmount::Unknown {
                    observed: Some(-1),
                    reason: CardCostUnknownReason::NegativeOrSentinel,
                },
            },
            expiration: CardExpiration::Unknown,
        }],
    };
    let projection = projection(
        &read,
        "card-special",
        CardLocation::Pile {
            pile: CardPile::Hand,
            position: CardPosition::Known(0),
        },
        CardUpgrade::new(0, Some("base"), None::<String>).expect("upgrade"),
        cost,
    );
    let snapshot = LiveCardSnapshot::new(
        read,
        [sts2_game_mod::LiveCardFixture::new(projection, 128)],
        true,
    );
    let mut store = LiveCardStore::new(snapshot, 1, 512).expect("special-cost store");
    let page = store
        .read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1))
        .expect("special-cost page");
    assert_eq!(page.entries[0].cost.base, CardCost::X);
    assert_eq!(page.entries[0].cost.current, CardCost::Free);
    assert_eq!(page.entries[0].cost.effective, CardCost::Unplayable);
    assert!(matches!(
        page.entries[0].cost.contributors[0].value,
        CardCost::AlternateResource {
            amount: CardCostAmount::Unknown {
                observed: Some(-1),
                reason: CardCostUnknownReason::NegativeOrSentinel,
            },
            ..
        }
    ));
}

#[test]
fn stale_epoch_rejects_old_instance_after_move_or_upgrade() {
    let mut store = store();
    let page = store
        .read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1))
        .expect("page");
    let old = page.entries[0].instance.clone();
    let mut next = snapshot(8);
    next.cards[0].projection.instance.read = reference(8);
    next.cards[0].projection.location = CardLocation::Pile {
        pile: CardPile::Discard,
        position: CardPosition::Known(0),
    };
    next.cards[0].projection.upgrade.count = 1;
    store.replace_snapshot(next).expect("replacement");
    assert_eq!(store.read_card(&old), Err(LiveCardError::StaleReference));
}

#[test]
fn unsupported_field_and_oversize_fail_closed_before_partial_success() {
    let mut fixture = snapshot(7);
    fixture.cards[1] = fixture.cards[1]
        .clone()
        .with_unsupported_fields([LiveCardField::EffectParameters]);
    let mut store = LiveCardStore::new(fixture.clone(), 2, 512).expect("store");
    let page = store
        .read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1))
        .expect("first card");
    let continuation = page.continuation.expect("continuation");
    let second = store.read_page(&LiveCardQuery {
        collection: LiveCardCollection::AllVisible,
        limit: 1,
        continuation: Some(continuation),
    });
    assert_eq!(
        second,
        Err(LiveCardError::UnsupportedField(
            LiveCardField::EffectParameters
        ))
    );
    let subset = fixture.cards[1].projection.instance.clone();
    let subset_store = LiveCardStore::new(fixture, 2, 512).expect("subset store");
    assert_eq!(
        subset_store
            .read_detail(&subset, &[LiveCardField::Location])
            .expect("supported subset")
            .fields
            .len(),
        1
    );

    let oversized = LiveCardStore::new(snapshot(7), 2, 64).expect("bounded store");
    let mut oversized = oversized;
    assert!(matches!(
        oversized.read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1)),
        Err(LiveCardError::DetailTooLarge { limit: 64, actual }) if actual >= 128
    ));
}

#[test]
fn invalid_collection_continuation_and_non_monotonic_epoch_are_typed() {
    let mut store = store();
    assert_eq!(
        store.read_page(&LiveCardQuery::new(
            LiveCardCollection::Selector(String::new()),
            1
        )),
        Err(LiveCardError::UnknownCollection)
    );
    let first = store
        .read_page(&LiveCardQuery::new(LiveCardCollection::AllVisible, 1))
        .expect("page");
    let continuation = first.continuation.expect("continuation");
    assert_eq!(
        store.read_page(&LiveCardQuery {
            collection: LiveCardCollection::Pile(CardPile::Hand),
            limit: 1,
            continuation: Some(continuation),
        }),
        Err(LiveCardError::StaleReference)
    );
    assert_eq!(
        store.replace_snapshot(snapshot(7)),
        Err(LiveCardError::NonMonotonicEpoch {
            current: 7,
            supplied: 7
        })
    );
}

#[test]
fn unavailable_source_is_explicit_and_fixture_source_is_read_only() {
    let unavailable = UnavailableLiveCardSource;
    assert_eq!(
        unavailable.capability(),
        LiveCardCapability::Unavailable(
            sts2_game_mod::LiveCardUnavailableReason::ExactHostEvidenceRequired
        )
    );
    assert_eq!(
        unavailable.read_snapshot(),
        Err(LiveCardError::Unavailable(
            sts2_game_mod::LiveCardUnavailableReason::ExactHostEvidenceRequired
        ))
    );
    assert!(matches!(
        LiveCardStore::from_source(&unavailable, 1, 512),
        Err(LiveCardError::Unavailable(
            sts2_game_mod::LiveCardUnavailableReason::ExactHostEvidenceRequired
        ))
    ));

    let source = FixtureLiveCardSource::new(snapshot(7));
    assert!(source.capability().is_available());
    let copy = source.read_snapshot().expect("owned snapshot");
    assert_eq!(copy.reference, reference(7));
    assert_eq!(copy.cards.len(), 2);
}
