// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/game_facts_reference.rs"]
mod fixture;

use fixture::{fixture, inventory, manifest, required_input, rule};
use sts2_game_mod::{
    FactsEvidenceStatus, FactsInputAvailability, FactsUnsupportedCombination,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION,
};

#[test]
fn an_inventory_binds_every_rule_to_one_build_and_representation() {
    let inventory = fixture();
    assert_eq!(
        inventory.producer_version(),
        GAME_FACTS_REFERENCE_PRODUCER_VERSION
    );
    assert_eq!(inventory.build().build_id, "build-2026-09");
    assert_eq!(inventory.build().manifest, manifest());
    assert_eq!(inventory.representation().encoding, "structured-facts");
    let damage = inventory.rule("damage.deal").expect("rule");
    assert_eq!(damage.evidence, FactsEvidenceStatus::Confirmed);
    assert_eq!(damage.input("amount").expect("input").unit, "health");
    assert!(inventory.rule("absent.rule").is_none());
}

#[test]
fn the_evidence_statuses_are_closed_and_named() {
    let names = FactsEvidenceStatus::ALL
        .iter()
        .map(|status| status.name())
        .collect::<Vec<_>>();
    assert_eq!(names.len(), 5);
    assert_eq!(names[0], "confirmed");
    assert!(names.contains(&"source_derived"));
    let unique = names.iter().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(unique.len(), names.len(), "names are unique");
}

#[test]
fn the_input_availability_is_closed_and_named() {
    let names = FactsInputAvailability::ALL
        .iter()
        .map(|availability| availability.name())
        .collect::<Vec<_>>();
    assert_eq!(names, ["required", "conditional", "unknown"]);
}

#[test]
fn a_declared_unsupported_combination_records_the_pair() {
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
            reason: "ordering not represented".to_owned(),
        }],
    )
    .expect("inventory");
    assert_eq!(inventory.unsupported_combinations().len(), 1);
    assert!(inventory.is_unrepresented("damage.deal"));
}
