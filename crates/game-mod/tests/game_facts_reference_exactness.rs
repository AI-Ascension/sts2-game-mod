// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/game_facts_reference.rs"]
mod fixture;

use fixture::{input, inventory, required_input, rule};
use sts2_game_mod::{FactsEvidenceStatus, FactsInputAvailability, FactsUnsupportedCombination};

#[test]
fn only_a_host_confirmed_status_supports_an_exact_claim() {
    assert!(FactsEvidenceStatus::Confirmed.supports_exact_claim());
    for status in FactsEvidenceStatus::ALL {
        if status != FactsEvidenceStatus::Confirmed {
            assert!(
                !status.supports_exact_claim(),
                "{status:?} is not a host comparison"
            );
        }
    }
}

#[test]
fn only_a_required_input_supports_an_exact_claim() {
    assert!(FactsInputAvailability::Required.supports_exact_claim());
    for availability in FactsInputAvailability::ALL {
        if availability != FactsInputAvailability::Required {
            assert!(
                !availability.supports_exact_claim(),
                "{availability:?} is not fully stated"
            );
        }
    }
}

#[test]
fn an_unsupported_interaction_never_reads_as_an_exact_claim() {
    let inventory = inventory(
        vec![
            rule(
                "damage.deal",
                FactsEvidenceStatus::Confirmed,
                vec![required_input("amount", "health")],
            ),
            rule(
                "block.gain",
                FactsEvidenceStatus::Confirmed,
                vec![required_input("amount", "block")],
            ),
        ],
        vec![FactsUnsupportedCombination {
            rule_ids: vec!["damage.deal".to_owned(), "block.gain".to_owned()],
            reason: "simplified model is unsupported here".to_owned(),
        }],
    )
    .expect("inventory");
    assert!(
        !inventory.is_exact_claim("damage.deal"),
        "a confirmed rule in an unsupported combination is not exact"
    );
    assert!(!inventory.is_exact_claim("absent.rule"));
}

#[test]
fn an_unqualified_rule_is_not_an_exact_claim_until_it_is_confirmed() {
    let unconfirmed = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::SourceDerived,
            vec![required_input("amount", "health")],
        )],
        Vec::new(),
    )
    .expect("inventory");
    assert!(!unconfirmed.is_exact_claim("damage.deal"));

    let confirmed = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![required_input("amount", "health")],
        )],
        Vec::new(),
    )
    .expect("inventory");
    assert!(confirmed.is_exact_claim("damage.deal"));
}

#[test]
fn a_confirmed_rule_with_a_conditional_input_is_not_an_exact_claim() {
    let inventory = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![
                required_input("amount", "health"),
                input("strength", "flat", FactsInputAvailability::Conditional),
            ],
        )],
        Vec::new(),
    )
    .expect("inventory");
    assert!(
        !inventory.is_exact_claim("damage.deal"),
        "a conditional input leaves the result open for combinations this inventory does not resolve"
    );
}

#[test]
fn a_confirmed_rule_with_an_unknown_input_is_not_an_exact_claim() {
    let inventory = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![input("amount", "health", FactsInputAvailability::Unknown)],
        )],
        Vec::new(),
    )
    .expect("inventory");
    assert!(
        !inventory.is_exact_claim("damage.deal"),
        "an unknown input cannot back an exact-looking result"
    );
}

#[test]
fn a_confirmed_rule_with_only_required_inputs_stays_an_exact_claim() {
    let inventory = inventory(
        vec![rule(
            "damage.deal",
            FactsEvidenceStatus::Confirmed,
            vec![
                required_input("amount", "health"),
                required_input("target", "entity"),
            ],
        )],
        Vec::new(),
    )
    .expect("inventory");
    assert!(inventory.is_exact_claim("damage.deal"));
    assert!(
        inventory
            .rule("damage.deal")
            .expect("declared rule")
            .inputs_support_exact_claim()
    );
}
