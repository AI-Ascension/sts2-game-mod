// SPDX-License-Identifier: MIT

//! Admission of caller-supplied signatures against the actual pinned pure Core catalog.

use sts2_game_core::{RULES_REFERENCE_VERSION, RuleUnit, rule_inventory};
use sts2_protocol::game_facts_reference_v1::GAME_FACTS_REFERENCE_V1_PROFILE;

use super::{
    FactsInputAvailability, FactsInventory, FactsRepresentation, FactsRuleEntry,
    FactsUnsupportedCombination, GameFactsError,
};
use crate::ContentManifest;

/// Immutable caller-supplied facts whose ordered input signatures agree with pinned Core v2.
///
/// Admission checks catalog membership, not source authenticity, content applicability, native
/// support or calculation parity. Evidence labels and provenance remain the caller's claims;
/// Core's synthetic fixture evidence is never copied into them. No inputs or values are invented.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoreFactsInventory {
    inventory: FactsInventory,
}

impl CoreFactsInventory {
    /// Validates the ordinary inventory bounds and manifest binding, then every Core signature.
    ///
    /// The representation is derived from the pinned Core version and accepted Protocol profile.
    /// Rule order, evidence, source references and unsupported interactions are retained verbatim.
    /// A missing rule or any input-count, order, name, unit or required/conditional mismatch is an
    /// error. No alias, implicit unit conversion or partial admission is performed.
    pub fn new_for_manifest(
        manifest: &ContentManifest,
        mode_id: impl Into<String>,
        rules: Vec<FactsRuleEntry>,
        unsupported: Vec<FactsUnsupportedCombination>,
    ) -> Result<Self, GameFactsError> {
        let inventory = FactsInventory::new_for_manifest(
            manifest,
            mode_id,
            FactsRepresentation {
                version: format!("rules-reference-v{RULES_REFERENCE_VERSION}"),
                encoding: GAME_FACTS_REFERENCE_V1_PROFILE.to_owned(),
            },
            rules,
            unsupported,
        )?;
        for rule in inventory.rules() {
            let reference = rule_inventory()
                .iter()
                .find(|reference| reference.id.as_str() == rule.rule_id)
                .ok_or(GameFactsError::InvalidInput("core_rule_id"))?;
            if rule.inputs.len() != reference.inputs.len() {
                return Err(GameFactsError::InvalidInput("core_input_count"));
            }
            for (supplied, declared) in rule.inputs.iter().zip(reference.inputs) {
                if supplied.name != declared.name {
                    return Err(GameFactsError::InvalidInput("core_input_name"));
                }
                if supplied.unit != unit_name(declared.unit) {
                    return Err(GameFactsError::InvalidInput("core_input_unit"));
                }
                let availability = if declared.required {
                    FactsInputAvailability::Required
                } else {
                    FactsInputAvailability::Conditional
                };
                if supplied.availability != availability {
                    return Err(GameFactsError::InvalidInput("core_input_availability"));
                }
            }
        }
        Ok(Self { inventory })
    }

    /// Reads the validated inventory without exposing a mutable path around admission.
    #[must_use]
    pub fn as_inventory(&self) -> &FactsInventory {
        &self.inventory
    }
}

/// Exact vocabulary mapping; this does not duplicate the catalog's signatures or semantics.
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
