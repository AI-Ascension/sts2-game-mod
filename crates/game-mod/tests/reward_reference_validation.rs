// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    RewardCatalog, RewardCatalogError, RewardCatalogProducer, RewardField, RewardNumericValue,
    RewardOfferDefinitionInput, RewardProbability, RewardQuantity, RewardSemanticReferenceKind,
    RewardUnavailableReason, RewardVisibility,
};

fn produce(
    content: &sts2_game_mod::ContentManifest,
    definition: RewardOfferDefinitionInput,
) -> Result<RewardCatalog, RewardCatalogError> {
    RewardCatalogProducer::new().produce(
        content,
        &RewardSource {
            snapshot: Ok(snapshot(content, vec![definition])),
        },
    )
}

fn reward_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[("reward", "reward:one"), ("card", "card:strike")])
}

fn base_reward() -> RewardOfferDefinitionInput {
    simple_reward(
        "reward:one",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    )
}

#[test]
fn duplicate_identities_are_rejected() {
    let content = reward_manifest();

    let mut duplicate_item = base_reward();
    duplicate_item.items.push(item(
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
        1,
        RewardVisibility::Visible,
    ));
    assert_eq!(
        produce(&content, duplicate_item),
        Err(RewardCatalogError::InvalidInput("duplicate_item"))
    );

    let mut duplicate_rule = base_reward();
    duplicate_rule
        .generation
        .push(rule("rule:one", &["item:one"]));
    assert_eq!(
        produce(&content, duplicate_rule),
        Err(RewardCatalogError::InvalidInput("duplicate_rule"))
    );

    let mut duplicate_requirement = base_reward();
    duplicate_requirement.generation[0]
        .eligibility
        .push(requirement(
            "req:act",
            sts2_game_mod::RewardRequirementKind::Progression,
        ));
    assert_eq!(
        produce(&content, duplicate_requirement),
        Err(RewardCatalogError::InvalidInput("duplicate_requirement"))
    );

    let mut duplicate_modifier = base_reward();
    duplicate_modifier.generation[0]
        .modifiers
        .push(modifier("mod:one"));
    assert_eq!(
        produce(&content, duplicate_modifier),
        Err(RewardCatalogError::InvalidInput("duplicate_modifier"))
    );

    let mut duplicate_action = base_reward();
    duplicate_action
        .selection
        .legal_actions
        .push(action(sts2_game_mod::RewardActionKind::Choose));
    assert_eq!(
        produce(&content, duplicate_action),
        Err(RewardCatalogError::InvalidInput("duplicate_action"))
    );

    let mut duplicate_reference = base_reward();
    duplicate_reference.references = vec![
        reference(RewardSemanticReferenceKind::Card, "card:strike"),
        reference(RewardSemanticReferenceKind::Card, "card:strike"),
    ];
    assert_eq!(
        produce(&content, duplicate_reference),
        Err(RewardCatalogError::InvalidInput("duplicate_reference"))
    );
}

#[test]
fn duplicate_and_unknown_definitions_are_rejected() {
    let content = reward_manifest();
    let first = base_reward();
    let second = base_reward();
    let mut parts = snapshot(&content, vec![first, second]);
    parts.family.definition_count = 1;
    assert_eq!(
        RewardCatalogProducer::new().produce(
            &content,
            &RewardSource {
                snapshot: Ok(parts),
            }
        ),
        Err(RewardCatalogError::DuplicateDefinition(
            "reward:one".to_owned()
        ))
    );

    let ghost_manifest = manifest(&[("reward", "reward:one"), ("card", "card:strike")]);
    let mut ghost = base_reward();
    ghost.reward_id = "reward:ghost".to_owned();
    assert_eq!(
        produce(&ghost_manifest, ghost),
        Err(RewardCatalogError::UnknownDefinition(
            "reward:ghost".to_owned()
        ))
    );
}

#[test]
fn dangling_manifest_references_are_rejected() {
    let content = reward_manifest();

    let mut unknown_card = base_reward();
    unknown_card.items[0].reference = reference(RewardSemanticReferenceKind::Card, "card:missing");
    assert_eq!(
        produce(&content, unknown_card),
        Err(RewardCatalogError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card:missing".to_owned(),
        })
    );

    let mut unknown_relic = base_reward();
    unknown_relic.references = vec![reference(
        RewardSemanticReferenceKind::Relic,
        "relic:missing",
    )];
    assert_eq!(
        produce(&content, unknown_relic),
        Err(RewardCatalogError::UnknownManifestReference {
            entity_kind: "relic".to_owned(),
            namespaced_id: "relic:missing".to_owned(),
        })
    );
}

#[test]
fn dangling_intra_definition_item_references_are_rejected() {
    let content = reward_manifest();
    let mut definition = base_reward();
    definition.generation[0].pool = RewardField::Available(vec![reference(
        RewardSemanticReferenceKind::Item,
        "item:ghost",
    )]);
    assert_eq!(
        produce(&content, definition),
        Err(RewardCatalogError::UnknownItemReference {
            reward_id: "reward:one".to_owned(),
            item_id: "item:ghost".to_owned(),
        })
    );
}

#[test]
fn invalid_probabilities_are_rejected() {
    let content = reward_manifest();

    let mut zero_denominator = base_reward();
    zero_denominator.generation[0].probability = RewardProbability::Exact {
        numerator: 1,
        denominator: 0,
        evidence: sts2_game_mod::RewardEvidence::SourceDerived,
    };
    assert_eq!(
        produce(&content, zero_denominator),
        Err(RewardCatalogError::InvalidInput("probability"))
    );

    let mut empty_rule = base_reward();
    empty_rule.generation[0].probability = RewardProbability::Rule {
        rule_reference: String::new(),
        evidence: sts2_game_mod::RewardEvidence::SourceDerived,
    };
    assert_eq!(
        produce(&content, empty_rule),
        Err(RewardCatalogError::InvalidInput("probability_rule"))
    );

    let mut empty_condition = base_reward();
    empty_condition.generation[0].probability = RewardProbability::Conditional {
        rule_reference: "rule:prob".to_owned(),
        condition_reference: String::new(),
        evidence: sts2_game_mod::RewardEvidence::IndependentlyAuthored,
    };
    assert_eq!(
        produce(&content, empty_condition),
        Err(RewardCatalogError::InvalidInput("probability_condition"))
    );
}

#[test]
fn invalid_quantities_are_rejected() {
    let content = reward_manifest();

    let mut negative = base_reward();
    negative.items[0].quantity.base_amount = RewardNumericValue::Fixed(-1);
    negative.items[0].quantity.visible_amount = RewardNumericValue::Fixed(-1);
    assert_eq!(
        produce(&content, negative),
        Err(RewardCatalogError::InvalidInput("item_quantity"))
    );

    let mut currency_without_unit = reward_definition_input(
        "reward:one",
        sts2_game_mod::RewardKind::Currency,
        vec![item_with_quantity(
            "item:one",
            RewardSemanticReferenceKind::Currency,
            "card:strike",
            quantity(None, 5),
            RewardVisibility::Visible,
        )],
    );
    currency_without_unit.kind = sts2_game_mod::RewardKind::Currency;
    assert_eq!(
        produce(&content, currency_without_unit),
        Err(RewardCatalogError::InvalidInput("quantity_unit"))
    );

    let mut inconsistent = base_reward();
    inconsistent.items[0].quantity = RewardQuantity {
        unit: RewardField::Unavailable(RewardUnavailableReason::NotApplicable),
        base_amount: RewardNumericValue::Fixed(5),
        visible_amount: RewardNumericValue::Fixed(5),
        modified: RewardField::Available(true),
    };
    assert_eq!(
        produce(&content, inconsistent),
        Err(RewardCatalogError::InvalidInput("quantity_modified"))
    );
}

#[test]
fn invalid_bounds_and_visibility_are_rejected() {
    let content = reward_manifest();

    let mut bad_bounds = base_reward();
    bad_bounds.selection.choose_min = RewardField::Available(3);
    bad_bounds.selection.choose_max = RewardField::Available(1);
    assert_eq!(
        produce(&content, bad_bounds),
        Err(RewardCatalogError::InvalidInput("choose_bounds"))
    );

    let mut bad_visibility = base_reward();
    bad_visibility.visibility = RewardVisibility::Unknown;
    assert_eq!(
        produce(&content, bad_visibility),
        Err(RewardCatalogError::InvalidInput("visibility"))
    );

    let mut bad_state = base_reward();
    bad_state.state_policy.claim_limit = RewardField::Available(0);
    assert_eq!(
        produce(&content, bad_state),
        Err(RewardCatalogError::InvalidInput("claim_limit"))
    );

    let mut bad_capacity = base_reward();
    bad_capacity.state_policy.capacity = RewardField::Available(0);
    assert_eq!(
        produce(&content, bad_capacity),
        Err(RewardCatalogError::InvalidInput("capacity"))
    );

    let mut bad_stage = base_reward();
    bad_stage.state_policy.multi_stage = RewardField::Available(0);
    assert_eq!(
        produce(&content, bad_stage),
        Err(RewardCatalogError::InvalidInput("multi_stage"))
    );
}

#[test]
fn malformed_and_oversized_identities_are_rejected() {
    let content = reward_manifest();

    let mut empty = base_reward();
    empty.reward_id = String::new();
    assert_eq!(
        produce(&content, empty),
        Err(RewardCatalogError::InvalidInput("reward_id"))
    );

    let mut oversized = base_reward();
    oversized.reward_id = "x".repeat(sts2_game_mod::REWARD_MAX_IDENTITY_BYTES + 1);
    assert_eq!(
        produce(&content, oversized),
        Err(RewardCatalogError::InvalidInput("reward_id"))
    );
}
