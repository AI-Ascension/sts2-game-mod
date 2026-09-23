// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

use sts2_game_mod::{
    ContentCursorBinding, FactsBuildBinding, FactsEvidenceStatus, FactsInputAvailability,
    FactsInventory, FactsRepresentation, FactsRuleEntry, FactsRuleInput,
    FactsUnsupportedCombination, GAME_FACTS_MAX_INPUTS_PER_RULE,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION, GameFactsError, is_opaque_facts_identity,
};

fn manifest() -> ContentCursorBinding {
    ContentCursorBinding {
        catalog_generation: 7,
        adapter_compatibility: "adapter-v1".to_owned(),
        content_set_revision: "content-rev-7".to_owned(),
        localized_text_revision: "text-rev-7".to_owned(),
        inventory_revision: "inventory-rev-7".to_owned(),
    }
}

fn build() -> FactsBuildBinding {
    FactsBuildBinding {
        build_id: "build-2026-09".to_owned(),
        mode_id: "standard".to_owned(),
        manifest: manifest(),
    }
}

fn representation() -> FactsRepresentation {
    FactsRepresentation {
        version: "facts-v1".to_owned(),
        encoding: "structured-facts".to_owned(),
    }
}

fn required_input(name: &str, unit: &str) -> FactsRuleInput {
    FactsRuleInput {
        name: name.to_owned(),
        unit: unit.to_owned(),
        availability: FactsInputAvailability::Required,
    }
}

fn rule(id: &str, evidence: FactsEvidenceStatus, inputs: Vec<FactsRuleInput>) -> FactsRuleEntry {
    FactsRuleEntry {
        rule_id: id.to_owned(),
        evidence,
        inputs,
    }
}

fn inventory(
    rules: Vec<FactsRuleEntry>,
    unsupported: Vec<FactsUnsupportedCombination>,
) -> Result<FactsInventory, GameFactsError> {
    FactsInventory::new(build(), representation(), rules, unsupported)
}

fn fixture() -> FactsInventory {
    inventory(
        vec![
            rule(
                "damage.deal",
                FactsEvidenceStatus::Confirmed,
                vec![
                    required_input("amount", "health"),
                    required_input("target", "entity"),
                ],
            ),
            rule(
                "block.gain",
                FactsEvidenceStatus::SourceDerived,
                vec![required_input("amount", "block")],
            ),
        ],
        Vec::new(),
    )
    .expect("inventory")
}

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
fn the_input_availability_is_closed_and_named() {
    let names = FactsInputAvailability::ALL
        .iter()
        .map(|availability| availability.name())
        .collect::<Vec<_>>();
    assert_eq!(names, ["required", "conditional", "unknown"]);
}

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
