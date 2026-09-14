// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    RewardFieldStatus, RewardKind, RewardOfferState, RewardVisibility, RewardVisibilityScope,
};

fn special_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[("reward", "reward:special"), ("card", "card:strike")])
}

#[test]
fn offer_states_remain_distinct() {
    let states = [
        RewardOfferState::Offered,
        RewardOfferState::Claimed,
        RewardOfferState::BlockedCapacity,
        RewardOfferState::ReplacementRequired,
        RewardOfferState::Skipped,
        RewardOfferState::MultiStage { stage: 2, total: 3 },
        RewardOfferState::Unknown,
    ];
    for (index, state) in states.iter().enumerate() {
        assert!(
            !states[index + 1..].contains(state),
            "offer states must remain distinct"
        );
    }
    assert_eq!(
        RewardOfferState::MultiStage { stage: 2, total: 3 },
        RewardOfferState::MultiStage { stage: 2, total: 3 }
    );

    let content = full_manifest();
    let catalog = rich_catalog(&content);
    let definition = catalog
        .get(
            &reward_definition(&catalog, "reward:card"),
            RewardVisibilityScope::Owner,
        )
        .expect("reward");
    assert_eq!(
        definition.state_policy.claim_limit,
        sts2_game_mod::RewardField::Available(1)
    );
    assert_eq!(
        definition.state_policy.capacity,
        sts2_game_mod::RewardField::Available(3)
    );
    assert_eq!(
        definition.state_policy.replacement,
        sts2_game_mod::RewardField::Available(sts2_game_mod::RewardReplacementPolicy::NotRequired)
    );
    assert_eq!(
        definition.state_policy.multi_stage,
        sts2_game_mod::RewardField::Unavailable(
            sts2_game_mod::RewardUnavailableReason::NotApplicable
        )
    );
}

#[test]
fn all_named_reward_categories_are_representable() {
    let content = special_manifest();
    let special = reward_definition_input(
        "reward:special",
        RewardKind::SpecialGrant,
        vec![item(
            "item:bonus",
            sts2_game_mod::RewardSemanticReferenceKind::Card,
            "card:strike",
            1,
            RewardVisibility::Visible,
        )],
    );
    let catalog = catalog(&content, vec![special]);
    let definition = catalog
        .get(
            &reward_definition(&catalog, "reward:special"),
            RewardVisibilityScope::Owner,
        )
        .expect("special");
    assert_eq!(definition.kind, RewardKind::SpecialGrant);
    assert_eq!(
        definition.selection.legal_actions.len(),
        2,
        "choose and skip actions stay visible"
    );
    assert_eq!(
        definition.selection.legal_actions_status,
        RewardFieldStatus::Available
    );
}
