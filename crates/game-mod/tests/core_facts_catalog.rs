// SPDX-License-Identifier: MIT

#[path = "support/core_facts_catalog.rs"]
mod catalog;
#[allow(dead_code)]
#[path = "support/game_facts_protocol_v1.rs"]
mod support;

use sts2_game_mod::{
    CoreFactsInventory, FactsInputAvailability, FactsRuleEntry, FactsUnsupportedCombination,
    GAME_FACTS_MAX_INPUTS_PER_RULE, GAME_FACTS_MAX_RULES, GameFactsError,
};

fn admit(rules: Vec<FactsRuleEntry>) -> Result<CoreFactsInventory, GameFactsError> {
    CoreFactsInventory::new_for_manifest(&support::manifest(), "standard", rules, Vec::new())
}

#[test]
fn all_sixteen_independent_signatures_are_admitted_without_reordering() -> Result<(), String> {
    let mut rules = catalog::rules();
    assert_eq!(rules.len(), 16);
    assert_eq!(sts2_game_core::rule_inventory().len(), 16);
    rules.reverse();
    let admitted = admit(rules.clone()).map_err(|error| error.to_string())?;
    assert_eq!(admitted.as_inventory().rules(), rules);
    assert_eq!(
        admitted.as_inventory().representation().version,
        "rules-reference-v2"
    );
    assert_eq!(
        admitted.as_inventory().representation().encoding,
        "game-facts-reference-v1"
    );
    assert!(
        admitted
            .as_inventory()
            .rules()
            .iter()
            .all(|rule| !admitted.as_inventory().is_exact_claim(&rule.rule_id))
    );
    Ok(())
}

#[test]
fn unknown_and_case_changed_rule_ids_are_refused() {
    let mut rule = catalog::rules().remove(0);
    rule.rule_id = "damage.unknown".to_owned();
    assert_eq!(
        admit(vec![rule.clone()]),
        Err(GameFactsError::InvalidInput("core_rule_id"))
    );
    rule.rule_id = "Damage.nominal_card".to_owned();
    assert_eq!(
        admit(vec![rule]),
        Err(GameFactsError::InvalidInput("core_rule_id"))
    );
}

#[test]
fn every_rule_refuses_missing_extra_and_reordered_inputs() {
    for original in catalog::rules() {
        let mut missing = original.clone();
        missing.inputs.pop();
        assert_eq!(
            admit(vec![missing]),
            Err(GameFactsError::InvalidInput("core_input_count"))
        );
        let mut extra = original.clone();
        let mut input = extra.inputs[0].clone();
        input.name = "extra_input".to_owned();
        extra.inputs.push(input);
        assert_eq!(
            admit(vec![extra]),
            Err(GameFactsError::InvalidInput("core_input_count"))
        );
        for index in 1..original.inputs.len() {
            let mut reordered = original.clone();
            reordered.inputs.swap(index - 1, index);
            assert_eq!(
                admit(vec![reordered]),
                Err(GameFactsError::InvalidInput("core_input_name"))
            );
        }
    }
}

#[test]
fn every_input_refuses_name_unit_case_and_availability_changes() {
    for original in catalog::rules() {
        for index in 0..original.inputs.len() {
            for changed_name in [
                "wrong_name".to_owned(),
                original.inputs[index].name.to_uppercase(),
            ] {
                let mut changed = original.clone();
                changed.inputs[index].name = changed_name;
                assert_eq!(
                    admit(vec![changed]),
                    Err(GameFactsError::InvalidInput("core_input_name"))
                );
            }
            for changed_unit in [
                "incorrect_unit".to_owned(),
                original.inputs[index].unit.to_uppercase(),
            ] {
                let mut changed = original.clone();
                changed.inputs[index].unit = changed_unit;
                assert_eq!(
                    admit(vec![changed]),
                    Err(GameFactsError::InvalidInput("core_input_unit"))
                );
            }
            for availability in [
                FactsInputAvailability::Conditional,
                FactsInputAvailability::Unknown,
            ] {
                let mut changed = original.clone();
                changed.inputs[index].availability = availability;
                assert_eq!(
                    admit(vec![changed]),
                    Err(GameFactsError::InvalidInput("core_input_availability"))
                );
            }
        }
    }
}

#[test]
fn legacy_synthetic_multiplier_cannot_be_split_into_new_signature() -> Result<(), String> {
    let mut legacy = catalog::rules().remove(1);
    admit(vec![legacy.clone()]).map_err(|error| error.to_string())?;
    assert_eq!(legacy.inputs[2].name, "multiplier");
    let mut denominator = legacy.inputs[2].clone();
    denominator.name = "multiplier_denominator".to_owned();
    legacy.inputs[2].name = "multiplier_numerator".to_owned();
    legacy.inputs.insert(3, denominator);
    assert_eq!(
        admit(vec![legacy]),
        Err(GameFactsError::InvalidInput("core_input_count"))
    );
    Ok(())
}

#[test]
fn admission_retains_ordinary_inventory_refusals() {
    assert_eq!(
        admit(Vec::new()),
        Err(GameFactsError::EmptyPresentCollection("rules"))
    );
    let rule = catalog::rules().remove(0);
    assert!(matches!(
        admit(vec![rule.clone(), rule.clone()]),
        Err(GameFactsError::DuplicateRule(_))
    ));
    assert_eq!(
        admit(vec![rule.clone(); GAME_FACTS_MAX_RULES + 1]),
        Err(GameFactsError::InventoryTooLarge {
            limit: GAME_FACTS_MAX_RULES,
            actual: GAME_FACTS_MAX_RULES + 1
        })
    );
    let mut too_many_inputs = rule.clone();
    too_many_inputs.inputs = vec![rule.inputs[0].clone(); GAME_FACTS_MAX_INPUTS_PER_RULE + 1];
    assert_eq!(
        admit(vec![too_many_inputs]),
        Err(GameFactsError::InventoryTooLarge {
            limit: GAME_FACTS_MAX_INPUTS_PER_RULE,
            actual: GAME_FACTS_MAX_INPUTS_PER_RULE + 1
        })
    );
    let mut bad_source = rule.clone();
    if let Some(source) = bad_source.inputs[0].source.as_mut() {
        source.reference = "/private/capture".to_owned();
    }
    assert_eq!(
        admit(vec![bad_source]),
        Err(GameFactsError::InvalidInput("source_ref"))
    );
    assert!(matches!(
        CoreFactsInventory::new_for_manifest(
            &support::manifest(),
            "standard",
            vec![rule],
            vec![FactsUnsupportedCombination {
                rule_ids: vec!["damage.nominal_card".to_owned(), "unknown.rule".to_owned()],
                reason: "Synthetic exclusion".to_owned()
            }]
        ),
        Err(GameFactsError::UnknownRuleReference { .. })
    ));
}
