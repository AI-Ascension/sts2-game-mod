// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    RewardCatalog, RewardCatalogError, RewardCatalogProducer, RewardFieldStatus, RewardKind,
    RewardOfferDefinitionInput, RewardSemanticReference, RewardSemanticReferenceKind,
    RewardVisibility, RewardVisibilityScope,
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

fn linked_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[
        ("reward", "reward:one"),
        ("reward", "reward:linked"),
        ("card", "card:strike"),
    ])
}

fn single_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[("reward", "reward:one"), ("card", "card:strike")])
}

fn referencing_reward(
    kind: RewardSemanticReferenceKind,
    target_visibility: RewardVisibility,
) -> Vec<RewardOfferDefinitionInput> {
    let mut first = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    first.references = vec![RewardSemanticReference {
        kind,
        id: "reward:linked".to_owned(),
        label: text("SECRET LABEL"),
    }];
    let mut linked = simple_reward(
        "reward:linked",
        "item:linked",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    linked.visibility = target_visibility;
    vec![first, linked]
}

#[test]
fn visible_reward_referencing_hidden_reward_is_rejected_without_leaking() {
    let content = linked_manifest();
    let error = produce(
        &content,
        referencing_reward(
            RewardSemanticReferenceKind::Reward,
            RewardVisibility::Hidden,
        ),
    )
    .expect_err("hidden reward edge must be rejected");
    assert_eq!(
        error,
        RewardCatalogError::HiddenReferenceLeak {
            reward_id: "reward:one".to_owned(),
            reference_kind: RewardSemanticReferenceKind::Reward,
        }
    );
    let rendered = format!("{error:?}");
    assert!(
        !rendered.contains("reward:linked") && !rendered.contains("SECRET LABEL"),
        "the rejection must not disclose the protected reward identity or label"
    );
}

#[test]
fn visible_reward_referencing_owner_only_reward_is_rejected() {
    let content = linked_manifest();
    assert_eq!(
        produce(
            &content,
            referencing_reward(
                RewardSemanticReferenceKind::Reward,
                RewardVisibility::OwnerOnly
            ),
        ),
        Err(RewardCatalogError::HiddenReferenceLeak {
            reward_id: "reward:one".to_owned(),
            reference_kind: RewardSemanticReferenceKind::Reward,
        })
    );
}

#[test]
fn content_kind_reward_alias_is_subject_to_visibility() {
    for target in [RewardVisibility::Hidden, RewardVisibility::OwnerOnly] {
        let content = linked_manifest();
        let expected = Err(RewardCatalogError::HiddenReferenceLeak {
            reward_id: "reward:one".to_owned(),
            reference_kind: RewardSemanticReferenceKind::Reward,
        });
        let canonical = produce(
            &content,
            referencing_reward(RewardSemanticReferenceKind::Reward, target),
        );
        let alias = produce(
            &content,
            referencing_reward(
                RewardSemanticReferenceKind::Content {
                    entity_kind: "reward".to_owned(),
                },
                target,
            ),
        );
        assert_eq!(canonical, expected, "canonical Reward path");
        assert_eq!(
            alias, expected,
            "reserved-family alias must match canonical enforcement"
        );
        let rendered = format!("{:?}", alias.expect_err("alias must be rejected"));
        assert!(
            !rendered.contains("reward:linked") && !rendered.contains("SECRET LABEL"),
            "the alias rejection must not disclose the protected identity or label"
        );
    }
}

#[test]
fn visible_rule_referencing_hidden_item_is_a_hidden_future_leak() {
    let content = single_manifest();
    let mut definition = simple_reward(
        "reward:one",
        "item:vis",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.items.push(item(
        "item:hid",
        RewardSemanticReferenceKind::Card,
        "card:strike",
        1,
        RewardVisibility::Hidden,
    ));
    definition.generation = vec![rule("rule:vis", &["item:vis", "item:hid"])];
    assert_eq!(
        produce(&content, vec![definition]),
        Err(RewardCatalogError::HiddenFutureLeak {
            reward_id: "reward:one".to_owned(),
            item_id: "item:hid".to_owned(),
        })
    );
}

#[test]
fn visible_selection_referencing_hidden_item_is_rejected() {
    let content = single_manifest();
    let mut definition = simple_reward(
        "reward:one",
        "item:vis",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.items.push(item(
        "item:hid",
        RewardSemanticReferenceKind::Card,
        "card:strike",
        1,
        RewardVisibility::Hidden,
    ));
    let mut hidden_rule = rule("rule:hid", &["item:hid"]);
    hidden_rule.visibility = RewardVisibility::Hidden;
    definition.generation = vec![rule("rule:vis", &["item:vis"]), hidden_rule];
    definition.selection.references =
        vec![reference(RewardSemanticReferenceKind::Item, "item:hid")];
    assert_eq!(
        produce(&content, vec![definition]),
        Err(RewardCatalogError::HiddenReferenceLeak {
            reward_id: "reward:one".to_owned(),
            reference_kind: RewardSemanticReferenceKind::Item,
        })
    );
}

fn withheld_catalog() -> RewardCatalog {
    let content = manifest(&[
        ("reward", "reward:withheld"),
        ("reward", "reward:empty"),
        ("reward", "reward:partial"),
        ("card", "card:strike"),
    ]);

    let mut withheld = simple_reward(
        "reward:withheld",
        "item:hid",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    withheld.items[0].visibility = RewardVisibility::Hidden;
    withheld.generation[0].visibility = RewardVisibility::Hidden;

    let mut empty = reward_definition_input("reward:empty", RewardKind::Card, Vec::new());
    empty.generation.clear();

    let mut partial = simple_reward(
        "reward:partial",
        "item:vis",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    partial.items.push(item(
        "item:hid",
        RewardSemanticReferenceKind::Card,
        "card:strike",
        1,
        RewardVisibility::Hidden,
    ));
    let mut hidden_rule = rule("rule:hid", &["item:hid"]);
    hidden_rule.visibility = RewardVisibility::Hidden;
    partial.generation.push(hidden_rule);
    partial.generation[0] = rule("rule:vis", &["item:vis"]);

    produce(&content, vec![withheld, empty, partial]).expect("catalog")
}

#[test]
fn withheld_collections_are_distinct_from_observed_empty() {
    let catalog = withheld_catalog();
    let withheld = catalog
        .get(
            &reward_definition(&catalog, "reward:withheld"),
            RewardVisibilityScope::Owner,
        )
        .expect("withheld reward");
    assert_eq!(withheld.items_status, RewardFieldStatus::Denied);
    assert!(withheld.items.is_empty());
    assert_eq!(withheld.generation_status, RewardFieldStatus::Denied);
    assert!(withheld.generation.is_empty());

    let empty = catalog
        .get(
            &reward_definition(&catalog, "reward:empty"),
            RewardVisibilityScope::Owner,
        )
        .expect("empty reward");
    assert_eq!(empty.items_status, RewardFieldStatus::Available);
    assert!(empty.items.is_empty());
    assert_eq!(empty.generation_status, RewardFieldStatus::Available);
    assert!(empty.generation.is_empty());
}

#[test]
fn partially_withheld_collections_report_partial() {
    let catalog = withheld_catalog();
    let partial = catalog
        .get(
            &reward_definition(&catalog, "reward:partial"),
            RewardVisibilityScope::Owner,
        )
        .expect("partial reward");
    assert_eq!(partial.items_status, RewardFieldStatus::Partial);
    assert_eq!(partial.items.len(), 1);
    assert_eq!(partial.items[0].reference.item_id, "item:vis");
    assert_eq!(partial.generation_status, RewardFieldStatus::Partial);
    assert_eq!(partial.generation.len(), 1);
    assert_eq!(partial.generation[0].rule_id, "rule:vis");
}

#[test]
fn list_summaries_distinguish_withheld_from_empty_collections() {
    let catalog = withheld_catalog();
    let mut reader = catalog.reader();
    let page = reader
        .list(&list_query("en-US", RewardVisibilityScope::Owner, 8))
        .expect("list");
    let withheld = page
        .entries
        .iter()
        .find(|entry| entry.reference.reward_id == "reward:withheld")
        .expect("withheld summary");
    assert_eq!(withheld.items_status, RewardFieldStatus::Denied);
    assert_eq!(withheld.generation_status, RewardFieldStatus::Denied);
    let empty = page
        .entries
        .iter()
        .find(|entry| entry.reference.reward_id == "reward:empty")
        .expect("empty summary");
    assert_eq!(empty.items_status, RewardFieldStatus::Available);
    assert_eq!(empty.generation_status, RewardFieldStatus::Available);
    let partial = page
        .entries
        .iter()
        .find(|entry| entry.reference.reward_id == "reward:partial")
        .expect("partial summary");
    assert_eq!(partial.items_status, RewardFieldStatus::Partial);
}
