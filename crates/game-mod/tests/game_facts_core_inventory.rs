// SPDX-License-Identifier: MIT

#[path = "support/game_facts_core_signatures.rs"]
mod signatures;
#[path = "support/game_facts_manifest.rs"]
mod support;

use sts2_game_core::{RULES_REFERENCE_VERSION, SourceStatus, rule_inventory};
use sts2_game_mod::{
    CoreFactsAdmissionError as Error, CoreFactsInventory, FactsEvidenceStatus,
    FactsInputAvailability, FactsRuleEntry, GameFactsError,
};
use sts2_protocol::game_facts_reference_v1::GAME_FACTS_REFERENCE_V1_PROFILE;

fn admit(rules: Vec<FactsRuleEntry>) -> Result<CoreFactsInventory, Error> {
    CoreFactsInventory::new_for_manifest(&support::manifest(), "standard", rules, Vec::new())
}

#[test]
fn all_reviewed_signatures_agree_with_the_actual_core_catalog_without_promoting_evidence()
-> Result<(), Error> {
    assert_eq!(RULES_REFERENCE_VERSION, 2);
    assert_eq!(rule_inventory().len(), 16);
    assert_eq!(signatures::SIGNATURES.len(), 16);
    assert!(
        rule_inventory()
            .iter()
            .all(|rule| rule.source == SourceStatus::Synthetic)
    );
    let mut rules = signatures::rules();
    // The subset's order belongs to the caller, not the catalog's storage order.
    rules.reverse();
    for (index, rule) in rules.iter_mut().enumerate() {
        rule.evidence = FactsEvidenceStatus::ALL[index % FactsEvidenceStatus::ALL.len()];
    }
    let expected = rules.clone();
    let admitted = admit(rules)?;
    assert_eq!(admitted.rules_reference_version(), 2);
    assert_eq!(admitted.inventory().rules(), expected);
    assert_eq!(
        admitted.inventory().representation().version,
        "rules-reference-v2"
    );
    assert_eq!(
        admitted.inventory().representation().encoding,
        GAME_FACTS_REFERENCE_V1_PROFILE
    );
    assert_eq!(admitted.inventory().build().mode_id, "standard");
    assert_eq!(admitted.inventory().build().build_id, "build-fixture");
    Ok(())
}

#[test]
fn every_rule_refuses_missing_extra_reordered_renamed_or_mistyped_inputs() {
    for original in signatures::rules() {
        let mut changed = original.clone();
        changed.inputs.pop();
        assert_eq!(
            admit(vec![changed]),
            Err(Error::InputCount { rule_index: 0 })
        );

        let mut changed = original.clone();
        let mut extra = changed.inputs[0].clone();
        extra.name = "unexpected_input".to_owned();
        changed.inputs.push(extra);
        assert_eq!(
            admit(vec![changed]),
            Err(Error::InputCount { rule_index: 0 })
        );

        let mut changed = original.clone();
        changed.inputs.swap(0, 1);
        assert_eq!(
            admit(vec![changed]),
            Err(Error::InputName {
                rule_index: 0,
                input_index: 0
            })
        );

        for input_index in 0..original.inputs.len() {
            let mut changed = original.clone();
            changed.inputs[input_index].name = "invented_input".to_owned();
            assert_eq!(
                admit(vec![changed]),
                Err(Error::InputName {
                    rule_index: 0,
                    input_index
                })
            );

            let mut changed = original.clone();
            changed.inputs[input_index].unit = "dimensionless".to_owned();
            assert_eq!(
                admit(vec![changed]),
                Err(Error::InputUnit {
                    rule_index: 0,
                    input_index
                })
            );

            for availability in [
                FactsInputAvailability::Conditional,
                FactsInputAvailability::Unknown,
            ] {
                let mut changed = original.clone();
                changed.inputs[input_index].availability = availability;
                assert_eq!(
                    admit(vec![changed]),
                    Err(Error::InputAvailability {
                        rule_index: 0,
                        input_index
                    })
                );
            }
        }
    }
}

#[test]
fn unknown_rules_and_ordinary_inventory_violations_remain_refusals() {
    let mut unknown = signatures::rules().remove(0);
    unknown.rule_id = "resource.unknown_rule".to_owned();
    assert_eq!(
        admit(vec![unknown]),
        Err(Error::UnknownRule { rule_index: 0 })
    );
    assert_eq!(
        admit(Vec::new()),
        Err(Error::Inventory(GameFactsError::EmptyPresentCollection(
            "rules"
        )))
    );

    let rule = signatures::rules().remove(0);
    assert!(matches!(
        admit(vec![rule.clone(), rule]),
        Err(Error::Inventory(GameFactsError::DuplicateRule(_)))
    ));
    let mut unsafe_source = signatures::rules().remove(0);
    if let Some(source) = &mut unsafe_source.inputs[0].source {
        source.reference = "file-private-source".to_owned();
    }
    let error = admit(vec![unsafe_source]);
    assert_eq!(
        error,
        Err(Error::Inventory(GameFactsError::InvalidInput("source_ref")))
    );
    assert_eq!(
        Error::Inventory(GameFactsError::InvalidInput("source_ref")).to_string(),
        "the supplied facts inventory is invalid"
    );
}

#[test]
fn the_legacy_single_multiplier_is_not_silently_rewritten_as_a_rational_pair() {
    let mut synthetic = signatures::rules().remove(1);
    synthetic.inputs[2].name = "multiplier_numerator".to_owned();
    assert_eq!(
        admit(vec![synthetic]),
        Err(Error::InputName {
            rule_index: 0,
            input_index: 2
        })
    );
}
