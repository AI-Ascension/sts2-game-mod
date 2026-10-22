// SPDX-License-Identifier: MIT

//! Admission against Core's actual pinned rule/input signatures, never native evidence.

use sts2_game_core::{RULES_REFERENCE_VERSION, RuleUnit, rule_inventory};
use sts2_protocol::game_facts_reference_v1::GAME_FACTS_REFERENCE_V1_PROFILE;

use super::{
    CoreFactsAdmissionError as Error, FactsInputAvailability, FactsInventory, FactsRepresentation,
    FactsRuleEntry, FactsUnsupportedCombination,
};
use crate::ContentManifest;

/// Immutable caller-supplied metadata whose rule/input signatures agree with pinned Core v2.
///
/// Membership, ordered names, units and required inputs are checked against the imported catalog.
/// This does not authenticate source references or manifest origin, provide input values, validate
/// game applicability, or promote Core's synthetic arithmetic evidence into native confirmation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreFactsInventory {
    inventory: FactsInventory,
}

impl CoreFactsInventory {
    /// Admits a supplied subset using Core's real catalog and the existing manifest-bound model.
    ///
    /// Rules and evidence labels retain caller order and content. The representation is derived
    /// from the pinned Core version and accepted Protocol profile, rather than supplied as a label.
    /// The caller still owns authenticity, coherent extraction and exact-build applicability.
    pub fn new_for_manifest(
        manifest: &ContentManifest,
        mode_id: impl Into<String>,
        rules: Vec<FactsRuleEntry>,
        unsupported: Vec<FactsUnsupportedCombination>,
    ) -> Result<Self, Error> {
        if RULES_REFERENCE_VERSION != 2 {
            return Err(Error::UnsupportedVersion);
        }
        let inventory = FactsInventory::new_for_manifest(
            manifest,
            mode_id,
            FactsRepresentation {
                version: format!("rules-reference-v{RULES_REFERENCE_VERSION}"),
                encoding: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
            },
            rules,
            unsupported,
        )
        .map_err(Error::Inventory)?;
        validate_signatures(&inventory)?;
        Ok(Self { inventory })
    }

    /// Returns immutable, locally admitted metadata without adding an authenticity claim.
    #[must_use]
    pub fn inventory(&self) -> &FactsInventory {
        &self.inventory
    }

    /// The pinned Core inventory version whose signatures were checked.
    #[must_use]
    pub const fn rules_reference_version(&self) -> u16 {
        RULES_REFERENCE_VERSION
    }
}

fn validate_signatures(inventory: &FactsInventory) -> Result<(), Error> {
    for (rule_index, supplied) in inventory.rules().iter().enumerate() {
        let core = rule_inventory()
            .iter()
            .find(|rule| rule.id.matches_key(&supplied.rule_id))
            .ok_or(Error::UnknownRule { rule_index })?;
        if core.version != RULES_REFERENCE_VERSION {
            return Err(Error::UnsupportedVersion);
        }
        if supplied.inputs.len() != core.inputs.len() {
            return Err(Error::InputCount { rule_index });
        }
        for (input_index, (input, declared)) in supplied.inputs.iter().zip(core.inputs).enumerate()
        {
            if input.name != declared.name {
                return Err(Error::InputName {
                    rule_index,
                    input_index,
                });
            }
            if input.unit != unit_name(declared.unit) {
                return Err(Error::InputUnit {
                    rule_index,
                    input_index,
                });
            }
            let availability = if declared.required {
                FactsInputAvailability::Required
            } else {
                FactsInputAvailability::Conditional
            };
            if input.availability != availability {
                return Err(Error::InputAvailability {
                    rule_index,
                    input_index,
                });
            }
        }
    }
    Ok(())
}

fn unit_name(unit: RuleUnit) -> &'static str {
    match unit {
        RuleUnit::Damage => "damage",
        RuleUnit::HitPoints => "hit_points",
        RuleUnit::Block => "block",
        RuleUnit::Energy => "energy",
        RuleUnit::Count => "count",
        RuleUnit::Turns => "turns",
        RuleUnit::Rounds => "rounds",
        RuleUnit::Gold => "gold",
        RuleUnit::Multiplier => "multiplier",
        RuleUnit::Ratio => "ratio",
        RuleUnit::Entity => "entity",
        RuleUnit::Boolean => "boolean",
        RuleUnit::Dimensionless => "dimensionless",
    }
}
