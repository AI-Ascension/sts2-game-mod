// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    RewardCatalog, RewardCatalogError, RewardCatalogProducer, RewardOfferDefinitionInput,
    RewardSemanticReferenceKind, RewardVisibilityScope,
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

fn reward_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[("reward", "reward:one"), ("card", "card:strike")])
}

#[test]
fn uncovered_item_is_rejected() {
    let content = reward_manifest();
    let mut definition = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.items.push(item(
        "item:two",
        RewardSemanticReferenceKind::Card,
        "card:strike",
        1,
        sts2_game_mod::RewardVisibility::Visible,
    ));
    assert_eq!(
        produce(&content, vec![definition]),
        Err(RewardCatalogError::UncoveredItem {
            reward_id: "reward:one".to_owned(),
            item_id: "item:two".to_owned(),
        })
    );
}

#[test]
fn item_offered_by_two_rules_is_rejected() {
    let content = reward_manifest();
    let mut definition = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.generation.push(rule("rule:two", &["item:one"]));
    assert_eq!(
        produce(&content, vec![definition]),
        Err(RewardCatalogError::DuplicateItemMembership {
            reward_id: "reward:one".to_owned(),
            item_id: "item:one".to_owned(),
        })
    );
}

#[test]
fn item_offered_twice_on_one_rule_is_rejected() {
    let content = reward_manifest();
    let mut definition = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.generation[0].pool = sts2_game_mod::RewardField::Available(vec![
        reference(RewardSemanticReferenceKind::Item, "item:one"),
        reference(RewardSemanticReferenceKind::Item, "item:one"),
    ]);
    assert_eq!(
        produce(&content, vec![definition]),
        Err(RewardCatalogError::InvalidInput("duplicate_reference"))
    );
}

#[test]
fn membership_and_item_definition_references_are_readable() {
    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let definition = catalog
        .get(
            &reward_definition(&catalog, "reward:card"),
            RewardVisibilityScope::Owner,
        )
        .expect("reward");
    assert_eq!(definition.generation.len(), 1);
    let pool = definition.generation[0]
        .pool
        .value()
        .expect("pool available");
    assert_eq!(
        pool.iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        ["item:strike", "item:defend"]
    );
    assert!(
        definition
            .items
            .iter()
            .all(|item| matches!(item.definition.kind, RewardSemanticReferenceKind::Card))
    );
}
