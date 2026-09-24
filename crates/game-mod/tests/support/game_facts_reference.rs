// SPDX-License-Identifier: MIT

//! Synthetic source-only game-facts fixture: one build binding, one representation, and the typed
//! rule/input builders shared by the game-facts reference tests.

#![allow(clippy::expect_used, dead_code)]

use sts2_game_mod::{
    ContentCursorBinding, FactsBuildBinding, FactsEvidenceStatus, FactsInputAvailability,
    FactsInventory, FactsRepresentation, FactsRuleEntry, FactsRuleInput,
    FactsUnsupportedCombination, GameFactsError,
};

pub fn manifest() -> ContentCursorBinding {
    ContentCursorBinding {
        catalog_generation: 7,
        adapter_compatibility: "adapter-v1".to_owned(),
        content_set_revision: "content-rev-7".to_owned(),
        localized_text_revision: "text-rev-7".to_owned(),
        inventory_revision: "inventory-rev-7".to_owned(),
    }
}

pub fn build() -> FactsBuildBinding {
    FactsBuildBinding {
        build_id: "build-2026-09".to_owned(),
        mode_id: "standard".to_owned(),
        manifest: manifest(),
    }
}

pub fn representation() -> FactsRepresentation {
    FactsRepresentation {
        version: "facts-v1".to_owned(),
        encoding: "structured-facts".to_owned(),
    }
}

pub fn required_input(name: &str, unit: &str) -> FactsRuleInput {
    FactsRuleInput {
        name: name.to_owned(),
        unit: unit.to_owned(),
        availability: FactsInputAvailability::Required,
    }
}

pub fn input(name: &str, unit: &str, availability: FactsInputAvailability) -> FactsRuleInput {
    FactsRuleInput {
        name: name.to_owned(),
        unit: unit.to_owned(),
        availability,
    }
}

pub fn rule(
    id: &str,
    evidence: FactsEvidenceStatus,
    inputs: Vec<FactsRuleInput>,
) -> FactsRuleEntry {
    FactsRuleEntry {
        rule_id: id.to_owned(),
        evidence,
        inputs,
    }
}

pub fn inventory(
    rules: Vec<FactsRuleEntry>,
    unsupported: Vec<FactsUnsupportedCombination>,
) -> Result<FactsInventory, GameFactsError> {
    FactsInventory::new(build(), representation(), rules, unsupported)
}

pub fn fixture() -> FactsInventory {
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
