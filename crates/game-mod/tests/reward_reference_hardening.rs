// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    RewardCatalog, RewardCatalogError, RewardCatalogProducer, RewardField, RewardFieldStatus,
    RewardOfferDefinitionInput, RewardSemanticReference, RewardSemanticReferenceKind, RewardText,
    RewardUnavailableReason, RewardVisibility, RewardVisibilityScope,
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

// --- Finding 1: pool references must be visibility-checked for both spellings. ---

fn pool_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[
        ("reward", "reward:one"),
        ("reward", "reward:linked"),
        ("card", "card:strike"),
    ])
}

fn pool_referencing_reward(
    kind: RewardSemanticReferenceKind,
    target_visibility: RewardVisibility,
) -> Vec<RewardOfferDefinitionInput> {
    let mut source = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    source.generation[0].pool = RewardField::Available(vec![
        reference(RewardSemanticReferenceKind::Item, "item:one"),
        RewardSemanticReference {
            kind,
            id: "reward:linked".to_owned(),
            label: text("SECRET POOL LABEL"),
        },
    ]);
    let mut linked = simple_reward(
        "reward:linked",
        "item:linked",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    linked.visibility = target_visibility;
    vec![source, linked]
}

#[test]
fn pool_reward_references_are_visibility_checked_for_both_spellings() {
    let spellings = [
        RewardSemanticReferenceKind::Reward,
        RewardSemanticReferenceKind::Content {
            entity_kind: "reward".to_owned(),
        },
    ];
    for kind in spellings {
        for target in [RewardVisibility::Hidden, RewardVisibility::OwnerOnly] {
            let content = pool_manifest();
            let error = produce(&content, pool_referencing_reward(kind.clone(), target))
                .expect_err("a restricted pool target must be rejected");
            assert_eq!(
                error,
                RewardCatalogError::HiddenReferenceLeak {
                    reward_id: "reward:one".to_owned(),
                    reference_kind: RewardSemanticReferenceKind::Reward,
                }
            );
            let rendered = format!("{error:?}");
            assert!(
                !rendered.contains("reward:linked") && !rendered.contains("SECRET POOL LABEL"),
                "the pool rejection must not disclose the protected identity or label"
            );
        }
    }
}

// --- Finding 2: restricted selections and state policies are withheld. ---

fn scoped_selection_catalog() -> RewardCatalog {
    let content = manifest(&[
        ("reward", "reward:scoped"),
        ("reward", "reward:hidden"),
        ("card", "card:strike"),
    ]);
    let mut owner = simple_reward(
        "reward:scoped",
        "item:vis",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    owner.selection.visibility = RewardVisibility::OwnerOnly;
    owner.selection.label = text("SECRET SELECTION");
    owner.state_policy.visibility = RewardVisibility::OwnerOnly;
    owner.state_policy.claim_limit = RewardField::Available(7);

    let mut hidden = simple_reward(
        "reward:hidden",
        "item:hid",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    hidden.items[0].visibility = RewardVisibility::Hidden;
    hidden.generation[0].visibility = RewardVisibility::Hidden;
    hidden.selection.visibility = RewardVisibility::Hidden;
    hidden.selection.label = text("HIDDEN SELECTION");
    hidden.state_policy.visibility = RewardVisibility::Hidden;
    hidden.state_policy.capacity = RewardField::Available(9);

    produce(&content, vec![owner, hidden]).expect("catalog")
}

#[test]
fn public_reader_withholds_owner_only_selection_and_policy() {
    let catalog = scoped_selection_catalog();
    let public = catalog
        .get(
            &reward_definition(&catalog, "reward:scoped"),
            RewardVisibilityScope::Public,
        )
        .expect("public reward");
    assert_eq!(public.selection_status, RewardFieldStatus::Denied);
    assert_eq!(
        public.selection.label,
        RewardText::Unavailable(RewardUnavailableReason::Denied)
    );
    assert!(public.selection.group_id.is_empty());
    assert!(public.selection.references.is_empty());
    assert!(public.selection.legal_actions.is_empty());
    assert_eq!(
        public.state_policy_status,
        RewardFieldStatus::Denied,
        "a restricted policy must be explicitly withheld"
    );
    assert_eq!(
        public.state_policy.claim_limit,
        RewardField::Unavailable(RewardUnavailableReason::Denied)
    );

    let owner = catalog
        .get(
            &reward_definition(&catalog, "reward:scoped"),
            RewardVisibilityScope::Owner,
        )
        .expect("owner reward");
    assert_eq!(owner.selection_status, RewardFieldStatus::Available);
    assert_eq!(owner.selection.label, text("SECRET SELECTION"));
    assert_eq!(owner.selection.legal_actions.len(), 2);
    assert_eq!(owner.state_policy_status, RewardFieldStatus::Available);
    assert_eq!(owner.state_policy.claim_limit, RewardField::Available(7));
}

#[test]
fn hidden_selection_and_policy_stay_withheld_in_owner_scope() {
    let catalog = scoped_selection_catalog();
    let owner = catalog
        .get(
            &reward_definition(&catalog, "reward:hidden"),
            RewardVisibilityScope::Owner,
        )
        .expect("hidden reward");
    assert_eq!(owner.selection_status, RewardFieldStatus::Denied);
    assert_ne!(owner.selection.label, text("HIDDEN SELECTION"));
    assert_eq!(owner.state_policy_status, RewardFieldStatus::Denied);
    assert_eq!(
        owner.state_policy.capacity,
        RewardField::Unavailable(RewardUnavailableReason::Denied)
    );
}

// --- Finding 3: source availability must survive scope projection. ---

fn status_rule_catalog(status: RewardFieldStatus) -> RewardCatalog {
    let content = manifest(&[("reward", "reward:one"), ("card", "card:strike")]);
    let mut definition = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.generation[0].rarity_weights.clear();
    definition.generation[0].rarity_status = status;
    definition.generation[0].eligibility.clear();
    definition.generation[0].eligibility_status = status;
    definition.generation[0].modifiers.clear();
    definition.generation[0].modifiers_status = status;
    catalog(&content, vec![definition])
}

#[test]
fn non_available_source_collection_statuses_are_preserved() {
    let statuses = [
        RewardFieldStatus::Denied,
        RewardFieldStatus::NotObserved,
        RewardFieldStatus::Unsupported,
        RewardFieldStatus::Failed,
        RewardFieldStatus::Unknown,
        RewardFieldStatus::Partial,
        RewardFieldStatus::NotApplicable,
    ];
    for status in statuses {
        let catalog = status_rule_catalog(status);
        let definition = catalog
            .get(
                &reward_definition(&catalog, "reward:one"),
                RewardVisibilityScope::Owner,
            )
            .expect("reward");
        let rule = &definition.generation[0];
        assert_eq!(rule.rarity_status, status, "rarity source status preserved");
        assert_eq!(
            rule.eligibility_status, status,
            "eligibility source status preserved"
        );
        assert_eq!(
            rule.modifiers_status, status,
            "modifiers source status preserved"
        );
    }
}

#[test]
fn inconsistent_status_and_value_combinations_are_rejected() {
    let content = manifest(&[("reward", "reward:one"), ("card", "card:strike")]);
    let base = simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );

    let mut rarity = base.clone();
    rarity.generation[0].rarity_status = RewardFieldStatus::Denied;
    assert_eq!(
        produce(&content, vec![rarity]),
        Err(RewardCatalogError::InvalidInput("rarity_status"))
    );

    let mut eligibility = base.clone();
    eligibility.generation[0].eligibility_status = RewardFieldStatus::NotObserved;
    assert_eq!(
        produce(&content, vec![eligibility]),
        Err(RewardCatalogError::InvalidInput("eligibility_status"))
    );

    let mut modifiers = base;
    modifiers.generation[0].modifiers_status = RewardFieldStatus::Failed;
    assert_eq!(
        produce(&content, vec![modifiers]),
        Err(RewardCatalogError::InvalidInput("modifiers_status"))
    );
}
