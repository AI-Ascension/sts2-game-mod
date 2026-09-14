// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, REWARD_MAX_IDENTITY_BYTES, REWARD_MAX_ITEMS, REWARD_MAX_REQUIREMENTS,
    REWARD_MAX_RULES, RewardCatalog, RewardCatalogError, RewardCatalogProducer, RewardField,
    RewardFieldStatus, RewardItemInstanceReference, RewardKind, RewardOfferDefinitionInput,
    RewardOfferReference, RewardRequirementKind, RewardSemanticReference,
    RewardSemanticReferenceKind, RewardSnapshotReference, RewardVisibility, RewardVisibilityScope,
};

fn produce(
    content: &sts2_game_mod::ContentManifest,
    definitions: Vec<RewardOfferDefinitionInput>,
) -> Result<RewardCatalog, RewardCatalogError> {
    RewardCatalogProducer::new().produce(
        content,
        &RewardSource {
            snapshot: Ok(snapshot(content, definitions)),
        },
    )
}

// --- Finding 4: nested identities are validated and counted. ---

fn instance_reference() -> RewardItemInstanceReference {
    RewardItemInstanceReference {
        offer: RewardOfferReference {
            snapshot: RewardSnapshotReference {
                run_id: "run:one".to_owned(),
                room_id: "room:1".to_owned(),
                snapshot_id: "snapshot:1".to_owned(),
            },
            offer_id: "offer:1".to_owned(),
        },
        item_instance_id: "instance:1".to_owned(),
    }
}

#[test]
fn malformed_nested_instance_identities_are_rejected() {
    let content = manifest(&[("reward", "reward:one"), ("card", "card:strike")]);
    let mut oversized_run = instance_reference();
    oversized_run.offer.snapshot.run_id = "x".repeat(REWARD_MAX_IDENTITY_BYTES + 1);
    let mut empty_room = instance_reference();
    empty_room.offer.snapshot.room_id = String::new();
    let mut newline_offer = instance_reference();
    newline_offer.offer.offer_id = "offer\none".to_owned();
    let mut nul_snapshot = instance_reference();
    nul_snapshot.offer.snapshot.snapshot_id = "snapshot\0one".to_owned();

    let cases = [
        ("instance_run", oversized_run),
        ("instance_room", empty_room),
        ("instance_offer", newline_offer),
        ("instance_snapshot", nul_snapshot),
    ];
    for (field, instance) in cases {
        let mut definition = simple_reward(
            "reward:one",
            "item:one",
            RewardSemanticReferenceKind::Card,
            "card:strike",
        );
        definition.items[0].instance = RewardField::Available(instance);
        assert_eq!(
            produce(&content, vec![definition]),
            Err(RewardCatalogError::InvalidInput(field)),
            "nested {field} must be validated"
        );
    }
}

fn nested_instance_bulk(run_len: usize) -> RewardOfferDefinitionInput {
    let items = (0..REWARD_MAX_ITEMS)
        .map(|index| {
            let mut offered = item(
                &format!("item:{index}"),
                RewardSemanticReferenceKind::Card,
                "card:strike",
                1,
                RewardVisibility::Visible,
            );
            offered.instance = RewardField::Available(RewardItemInstanceReference {
                offer: RewardOfferReference {
                    snapshot: RewardSnapshotReference {
                        run_id: "r".repeat(run_len),
                        room_id: "room:1".to_owned(),
                        snapshot_id: "snap:1".to_owned(),
                    },
                    offer_id: "offer:1".to_owned(),
                },
                item_instance_id: format!("instance:{index}"),
            });
            offered
        })
        .collect::<Vec<_>>();
    let rules = (0..REWARD_MAX_RULES)
        .map(|index| {
            let item_id = format!("item:{index}");
            let mut generation = rule(&format!("rule:{index}"), &[item_id.as_str()]);
            generation.eligibility = (0..REWARD_MAX_REQUIREMENTS)
                .map(|requirement_index| {
                    let mut req = requirement(
                        &format!("req:{index}:{requirement_index}"),
                        RewardRequirementKind::Custom("k".repeat(REWARD_MAX_IDENTITY_BYTES)),
                    );
                    req.parameters.clear();
                    req
                })
                .collect();
            generation
        })
        .collect::<Vec<_>>();
    RewardOfferDefinitionInput {
        reward_id: "reward:one".to_owned(),
        label: text("Bulk"),
        kind: RewardKind::Card,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: RewardVisibility::Visible,
        selection: selection("group:bulk", 1, 1, true),
        items,
        generation: rules,
        state_policy: state_policy(),
        references: Vec::new(),
    }
}

fn nested_instance_actual(run_len: usize) -> usize {
    let content = manifest(&[("reward", "reward:one"), ("card", "card:strike")]);
    match produce(&content, vec![nested_instance_bulk(run_len)]) {
        Err(RewardCatalogError::DefinitionTooLarge { actual, .. }) => actual,
        other => {
            assert!(matches!(
                other,
                Err(RewardCatalogError::DefinitionTooLarge { .. })
            ));
            0
        }
    }
}

#[test]
fn nested_instance_strings_count_toward_definition_bound() {
    let long = nested_instance_actual(REWARD_MAX_IDENTITY_BYTES);
    let short = nested_instance_actual(REWARD_MAX_IDENTITY_BYTES - 4);
    assert_eq!(
        long - short,
        REWARD_MAX_ITEMS * 4,
        "every retained nested instance byte must count toward the definition bound"
    );
}

// --- Finding 5: hidden and empty item pages are distinguishable. ---

fn item_page_catalog() -> RewardCatalog {
    let content = manifest(&[
        ("reward", "reward:hidden"),
        ("reward", "reward:empty"),
        ("card", "card:strike"),
    ]);
    let mut hidden = simple_reward(
        "reward:hidden",
        "item:hid",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    hidden.items[0].visibility = RewardVisibility::Hidden;
    hidden.generation[0].visibility = RewardVisibility::Hidden;
    let mut empty = reward_definition_input("reward:empty", RewardKind::Card, Vec::new());
    empty.generation.clear();
    produce(&content, vec![hidden, empty]).expect("catalog")
}

#[test]
fn hidden_and_empty_item_pages_are_distinguishable() {
    let catalog = item_page_catalog();
    let mut reader = catalog.reader();

    let hidden = reader
        .list_items(&item_list_query(
            reward_definition(&catalog, "reward:hidden"),
            RewardVisibilityScope::Owner,
            8,
        ))
        .expect("hidden item page");
    assert!(hidden.entries.is_empty());
    assert_eq!(hidden.total, 0);
    assert!(hidden.complete);
    assert_eq!(
        hidden.items_status,
        RewardFieldStatus::Denied,
        "a withheld item collection must not read as an observed-empty one"
    );

    let empty = reader
        .list_items(&item_list_query(
            reward_definition(&catalog, "reward:empty"),
            RewardVisibilityScope::Owner,
            8,
        ))
        .expect("empty item page");
    assert!(empty.entries.is_empty());
    assert_eq!(empty.total, 0);
    assert!(empty.complete);
    assert_eq!(empty.items_status, RewardFieldStatus::Available);
    assert_ne!(hidden.items_status, empty.items_status);
}

// --- Finding 6: reference edges honor unlock-based scope restrictions. ---

fn locked_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[
        ("reward", "reward:one"),
        ("reward", "reward:locked"),
        ("card", "card:strike"),
    ])
}

fn referencing_locked(
    owner_unlock: ContentUnlockState,
    target_unlock: ContentUnlockState,
) -> Vec<RewardOfferDefinitionInput> {
    let mut source = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    source.unlock_state = owner_unlock;
    source.references = vec![RewardSemanticReference {
        kind: RewardSemanticReferenceKind::Reward,
        id: "reward:locked".to_owned(),
        label: text("SECRET LOCKED LABEL"),
    }];
    let mut target = simple_reward(
        "reward:locked",
        "item:locked",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    target.unlock_state = target_unlock;
    vec![source, target]
}

#[test]
fn reference_to_locked_target_is_rejected_like_exact_lookup() {
    let content = locked_manifest();
    let error = produce(
        &content,
        referencing_locked(ContentUnlockState::Unlocked, ContentUnlockState::Locked),
    )
    .expect_err("an unlocked public reward must not expose a locked target");
    assert_eq!(
        error,
        RewardCatalogError::HiddenReferenceLeak {
            reward_id: "reward:one".to_owned(),
            reference_kind: RewardSemanticReferenceKind::Reward,
        }
    );
    assert!(
        !format!("{error:?}").contains("reward:locked"),
        "the rejection must not disclose the locked target identity"
    );

    let mut unreferenced =
        referencing_locked(ContentUnlockState::Unlocked, ContentUnlockState::Locked);
    unreferenced[0].references.clear();
    let catalog = produce(&content, unreferenced).expect("catalog");
    assert_eq!(
        catalog.get(
            &reward_definition(&catalog, "reward:locked"),
            RewardVisibilityScope::Public
        ),
        Err(RewardCatalogError::ExcludedByScope),
        "exact lookup agrees the locked target is out of public scope"
    );
    assert!(
        catalog
            .get(
                &reward_definition(&catalog, "reward:locked"),
                RewardVisibilityScope::Reference
            )
            .is_ok()
    );
}

#[test]
fn locked_owner_reference_to_locked_target_is_not_over_rejected() {
    let content = locked_manifest();
    let catalog = produce(
        &content,
        referencing_locked(ContentUnlockState::Locked, ContentUnlockState::Locked),
    )
    .expect("both locked records are reference-visible");
    let definition = catalog
        .get(
            &reward_definition(&catalog, "reward:one"),
            RewardVisibilityScope::Reference,
        )
        .expect("reference scope");
    assert_eq!(definition.references.len(), 1);
}
