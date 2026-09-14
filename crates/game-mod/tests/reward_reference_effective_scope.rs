// SPDX-License-Identifier: MIT

//! Effective-scope enforcement for local and cross-definition reward references.
//!
//! These regressions pin the round-2 review findings: an owner-only target whose unlock state is
//! unknown must be excluded in every scope (matching exact lookup), and a `Rule` or `Modifier`
//! reference that resolves to a restricted local record must be rejected rather than falling
//! through. No test reveals a protected target identity or label.

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod support;

use sts2_game_mod::{
    ContentManifest, ContentUnlockState, REWARD_REFERENCE_ENTITY_KIND, RewardCatalog,
    RewardCatalogError, RewardCatalogProducer, RewardOfferDefinitionInput,
    RewardSemanticReferenceKind, RewardVisibility,
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

#[test]
fn owner_only_unknown_unlock_target_is_rejected_like_exact_lookup() {
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
        let mut referencing = simple_reward(
            "reward:card",
            "item:card",
            RewardSemanticReferenceKind::Card,
            "card:strike",
        );
        referencing.references = vec![reference(kind.clone(), "reward:secret")];
        let mut secret = simple_reward(
            "reward:secret",
            "item:secret",
            RewardSemanticReferenceKind::Card,
            "card:defend",
        );
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
    // Guard against over-rejection: an owner-visible record may reference a known owner-only
    // target (exact lookup accepts it in owner scope), so the catalog must build.
    let manifest = manifest(&[
        ("reward", "reward:card"),
        ("reward", "reward:owner"),
        ("card", "card:strike"),
        ("card", "card:defend"),
    ]);
    let mut referencing = simple_reward(
        "reward:card",
        "item:card",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    referencing.visibility = RewardVisibility::OwnerOnly;
    referencing.references = vec![reference(
        RewardSemanticReferenceKind::Reward,
        "reward:owner",
    )];
    let mut owner = simple_reward(
        "reward:owner",
        "item:owner",
        RewardSemanticReferenceKind::Card,
        "card:defend",
    );
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

    let mut definition = simple_reward(
        "reward:card",
        "item:card",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
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

    let mut definition = simple_reward(
        "reward:card",
        "item:card",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );
    definition.generation = vec![rule_one];

    let result = try_catalog(&manifest, vec![definition]);
    assert!(
        matches!(result, Err(RewardCatalogError::HiddenReferenceLeak { .. })),
        "a visible modifier must not disclose a hidden local modifier"
    );
    assert_no_target_leak(&result, "mod:hidden");
}
