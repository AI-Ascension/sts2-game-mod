// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/game_facts_reference.rs"]
mod fixture;

use fixture::{inventory, required_input, rule};
use sts2_game_mod::{
    FactsEvidenceStatus, FactsUnsupportedCombination, GAME_FACTS_MAX_INPUTS_PER_RULE,
    GameFactsError, is_opaque_facts_identity,
};

#[test]
fn an_empty_rule_inventory_states_nothing() {
    let error = inventory(Vec::new(), Vec::new()).expect_err("refused");
    assert_eq!(error, GameFactsError::EmptyPresentCollection("rules"));
}

#[test]
fn a_rule_without_a_typed_input_is_refused_so_facts_are_not_hidden_in_prose() {
    let error = inventory(
        vec![rule(
            "opaque.rule",
            FactsEvidenceStatus::Proposed,
            Vec::new(),
        )],
        Vec::new(),
    )
    .expect_err("refused");
    assert_eq!(error, GameFactsError::EmptyPresentCollection("inputs"));
}

#[test]
fn a_non_opaque_rule_id_is_refused() {
    let error = inventory(
        vec![rule(
            "../../etc/passwd",
            FactsEvidenceStatus::Confirmed,
            vec![required_input("amount", "health")],
        )],
        Vec::new(),
    )
    .expect_err("refused");
    assert_eq!(error, GameFactsError::NonOpaqueIdentity("rule_id"));
    assert!(!is_opaque_facts_identity("file:///save"));
    assert!(is_opaque_facts_identity("damage.deal"));
}

#[test]
fn a_duplicate_rule_id_is_refused() {
    let duplicated = rule(
        "damage.deal",
        FactsEvidenceStatus::Confirmed,
        vec![required_input("amount", "health")],
    );
    let error = inventory(vec![duplicated.clone(), duplicated], Vec::new()).expect_err("refused");
    assert_eq!(
        error,
        GameFactsError::DuplicateRule("damage.deal".to_owned())
    );
}

#[test]
fn a_duplicate_input_name_within_one_rule_is_refused() {
    let error = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![
                required_input("amount", "health"),
                required_input("amount", "flat"),
            ],
        )],
        Vec::new(),
    )
    .expect_err("refused");
    assert_eq!(
        error,
        GameFactsError::DuplicateInput {
            rule_id: "damage.deal".to_owned(),
            name: "amount".to_owned(),
        }
    );
}

#[test]
fn an_input_without_a_unit_is_refused() {
    let error = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![required_input("amount", "")],
        )],
        Vec::new(),
    )
    .expect_err("refused");
    assert_eq!(error, GameFactsError::NonOpaqueIdentity("input_unit"));
}

#[test]
fn an_input_count_beyond_the_bound_is_refused() {
    let inputs = (0..=GAME_FACTS_MAX_INPUTS_PER_RULE)
        .map(|index| required_input(&format!("input-{index}"), "flat"))
        .collect::<Vec<_>>();
    let error = inventory(
        vec![rule("damage.deal", FactsEvidenceStatus::Confirmed, inputs)],
        Vec::new(),
    )
    .expect_err("refused");
    assert!(matches!(error, GameFactsError::InventoryTooLarge { .. }));
}

#[test]
fn an_unsupported_combination_naming_an_unknown_rule_is_refused() {
    let error = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![required_input("amount", "health")],
        )],
        vec![FactsUnsupportedCombination {
            rule_ids: vec!["damage.deal".to_owned(), "block.gain".to_owned()],
            reason: "interaction not represented".to_owned(),
        }],
    )
    .expect_err("refused");
    assert_eq!(
        error,
        GameFactsError::UnknownRuleReference {
            combination: 0,
            rule_id: "block.gain".to_owned(),
        }
    );
}

#[test]
fn an_unsupported_combination_needs_two_distinct_members() {
    let single = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![required_input("amount", "health")],
        )],
        vec![FactsUnsupportedCombination {
            rule_ids: vec!["damage.deal".to_owned()],
            reason: "not a combination".to_owned(),
        }],
    )
    .expect_err("refused");
    assert_eq!(
        single,
        GameFactsError::InvalidInput("unsupported_combination")
    );

    let repeated = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![required_input("amount", "health")],
        )],
        vec![FactsUnsupportedCombination {
            rule_ids: vec!["damage.deal".to_owned(), "damage.deal".to_owned()],
            reason: "one rule is not a combination".to_owned(),
        }],
    )
    .expect_err("refused");
    assert_eq!(
        repeated,
        GameFactsError::InvalidInput("unsupported_combination_member")
    );
}
