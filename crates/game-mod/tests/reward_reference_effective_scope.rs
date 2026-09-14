// SPDX-License-Identifier: MIT

//! Effective-scope enforcement for local and cross-definition reward references.
//!
//! These regressions pin the round-2/round-3 review findings: an owner-only target whose unlock
//! state is unknown must be excluded in every scope (matching exact lookup), a `Rule` or `Modifier`
//! reference that resolves to a restricted local record must be rejected, a modifier inherits its
//! parent rule's restriction, and a nested source record inherits its parent rule's restriction so
//! a hidden rule is not over-rejected. No test reveals a protected target identity or label.

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod support;

use sts2_game_mod::{
    ContentManifest, ContentUnlockState, REWARD_REFERENCE_ENTITY_KIND, RewardCatalog,
    RewardCatalogError, RewardCatalogProducer, RewardEvidence, RewardField, RewardFormula,
    RewardNumericValue, RewardOfferDefinitionInput, RewardProbability, RewardSemanticReferenceKind,
    RewardVisibility,
};
use support::*;

fn try_catalog(
    manifest: &ContentManifest,
    definitions: Vec<RewardOfferDefinitionInput>,
) -> Result<RewardCatalog, RewardCatalogError> {
    RewardCatalogProducer::new().produce(
        manifest,
        &RewardSource {
            snapshot: Ok(snapshot(manifest, definitions)),
        },
    )
}

fn assert_no_target_leak(result: &Result<RewardCatalog, RewardCatalogError>, target: &str) {
    if let Err(error) = result {
        let rendered = format!("{error:?}");
        assert!(
            !rendered.contains(target),
            "error must not leak target identity {target}: {rendered}"
        );
    }
}

fn card_reward_def(reward_id: &str, item_id: &str, card_id: &str) -> RewardOfferDefinitionInput {
    simple_reward(
        reward_id,
        item_id,
        RewardSemanticReferenceKind::Card,
        card_id,
    )
}

#[test]
fn owner_only_unknown_unlock_target_is_rejected_like_exact_lookup() {
    // Owner-visible source: without the Unknown-unlock exclusion this reference would be accepted
    // (owner scope equals owner scope), while exact lookup excludes the target in every scope.
    let manifest = manifest(&[
        ("reward", "reward:card"),
        ("reward", "reward:secret"),
        ("card", "card:strike"),
        ("card", "card:defend"),
    ]);
    for kind in [
        RewardSemanticReferenceKind::Reward,
        RewardSemanticReferenceKind::Content {
            entity_kind: REWARD_REFERENCE_ENTITY_KIND.to_owned(),
        },
    ] {
        let mut referencing = card_reward_def("reward:card", "item:card", "card:strike");
        referencing.visibility = RewardVisibility::OwnerOnly;
        referencing.references = vec![reference(kind.clone(), "reward:secret")];
        let mut secret = card_reward_def("reward:secret", "item:secret", "card:defend");
        secret.visibility = RewardVisibility::OwnerOnly;
        secret.unlock_state = ContentUnlockState::Unknown;

        let result = try_catalog(&manifest, vec![referencing, secret]);
        assert!(
            matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
            "owner-only/unknown-unlock target must be excluded for {kind:?}"
        );
        assert_no_target_leak(&result, "reward:secret");
    }
}

#[test]
fn owner_only_unlocked_target_stays_reachable_in_owner_scope() {
    let manifest = manifest(&[
        ("reward", "reward:card"),
        ("reward", "reward:owner"),
        ("card", "card:strike"),
        ("card", "card:defend"),
    ]);
    let mut referencing = card_reward_def("reward:card", "item:card", "card:strike");
    referencing.visibility = RewardVisibility::OwnerOnly;
    referencing.references = vec![reference(
        RewardSemanticReferenceKind::Reward,
        "reward:owner",
    )];
    let mut owner = card_reward_def("reward:owner", "item:owner", "card:defend");
    owner.visibility = RewardVisibility::OwnerOnly;
    owner.unlock_state = ContentUnlockState::Unlocked;

    let result = try_catalog(&manifest, vec![referencing, owner]);
    assert!(
        result.is_ok(),
        "owner-only/unlocked target must remain valid: {result:?}"
    );
}

#[test]
fn local_rule_reference_to_hidden_target_is_rejected() {
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    visible_rule.references = vec![reference(RewardSemanticReferenceKind::Rule, "rule:hidden")];
    let mut hidden_rule = rule("rule:hidden", &[]);
    hidden_rule.visibility = RewardVisibility::Hidden;

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![visible_rule, hidden_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a visible rule must not disclose a hidden local rule"
    );
    assert_no_target_leak(&result, "rule:hidden");
}

#[test]
fn local_modifier_reference_to_hidden_target_is_rejected() {
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_modifier = modifier("mod:visible");
    visible_modifier.references = vec![reference(
        RewardSemanticReferenceKind::Modifier,
        "mod:hidden",
    )];
    let mut hidden_modifier = modifier("mod:hidden");
    hidden_modifier.visibility = RewardVisibility::Hidden;

    let mut rule_one = rule("rule:one", &["item:card"]);
    rule_one.modifiers = vec![visible_modifier, hidden_modifier];

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![rule_one];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a visible modifier must not disclose a hidden local modifier"
    );
    assert_no_target_leak(&result, "mod:hidden");
}

#[test]
fn modifier_inside_restricted_rule_is_rejected_when_referenced() {
    // A Visible modifier nested in a Hidden rule must inherit the parent rule's restriction.
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    visible_rule.references = vec![reference(
        RewardSemanticReferenceKind::Modifier,
        "mod:inside",
    )];
    let mut hidden_rule = rule("rule:hidden", &[]);
    hidden_rule.visibility = RewardVisibility::Hidden;
    hidden_rule.modifiers = vec![modifier("mod:inside")];

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![visible_rule, hidden_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a modifier inside a hidden rule must not be referenced visibly"
    );
    assert_no_target_leak(&result, "mod:inside");
}

#[test]
fn hidden_rule_requirement_self_reference_is_not_over_rejected() {
    // A Visible requirement nested in a Hidden rule referencing its own rule is withheld together
    // with the rule and must not reject the whole catalog.
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    visible_rule.references = Vec::new();
    let mut hidden_rule = rule("rule:hidden", &[]);
    hidden_rule.visibility = RewardVisibility::Hidden;
    hidden_rule.references = Vec::new();
    let mut requirement = requirement(
        "req:inside",
        sts2_game_mod::RewardRequirementKind::Progression,
    );
    requirement.references = vec![reference(RewardSemanticReferenceKind::Rule, "rule:hidden")];
    hidden_rule.eligibility = vec![requirement];

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![visible_rule, hidden_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        result.is_ok(),
        "a hidden rule's nested requirement self-reference must not over-reject: {result:?}"
    );
}

#[test]
fn modifier_rule_reference_to_hidden_target_is_rejected() {
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    let mut referencing = modifier("mod:refs_rule");
    referencing.rule_reference = RewardField::Available("rule:hidden".to_owned());
    visible_rule.modifiers = vec![referencing];
    let mut hidden_rule = rule("rule:hidden", &[]);
    hidden_rule.visibility = RewardVisibility::Hidden;

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![visible_rule, hidden_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a modifier rule-reference must not disclose a hidden local rule"
    );
    assert_no_target_leak(&result, "rule:hidden");
}

#[test]
fn probability_rule_reference_to_hidden_target_is_rejected() {
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    visible_rule.probability = RewardProbability::Rule {
        rule_reference: "rule:hidden".to_owned(),
        evidence: RewardEvidence::SourceDerived,
    };
    let mut hidden_rule = rule("rule:hidden", &[]);
    hidden_rule.visibility = RewardVisibility::Hidden;

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![visible_rule, hidden_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a probability rule-reference must not disclose a hidden local rule"
    );
    assert_no_target_leak(&result, "rule:hidden");
}

#[test]
fn item_formula_rule_reference_to_hidden_target_is_rejected() {
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    visible_rule.references = Vec::new();
    let mut hidden_rule = rule("rule:hidden", &[]);
    hidden_rule.visibility = RewardVisibility::Hidden;

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.items[0].quantity.base_amount = RewardNumericValue::Formula(RewardFormula {
        rule_reference: "rule:hidden".to_owned(),
        unresolved_inputs: Vec::new(),
    });
    definition.generation = vec![visible_rule, hidden_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a formula rule-reference must not disclose a hidden local rule"
    );
    assert_no_target_leak(&result, "rule:hidden");
}

#[test]
fn rule_reference_strings_to_visible_local_rule_are_accepted() {
    let manifest = manifest(&[("reward", "reward:card"), ("card", "card:strike")]);
    let mut visible_rule = rule("rule:one", &["item:card"]);
    let mut referencing = modifier("mod:refs_rule");
    referencing.rule_reference = RewardField::Available("rule:one".to_owned());
    visible_rule.modifiers = vec![referencing];
    visible_rule.probability = RewardProbability::Rule {
        rule_reference: "rule:one".to_owned(),
        evidence: RewardEvidence::SourceDerived,
    };
    visible_rule.references = Vec::new();

    let mut definition = card_reward_def("reward:card", "item:card", "card:strike");
    definition.generation = vec![visible_rule];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        result.is_ok(),
        "references to a visible local rule must build: {result:?}"
    );
}
