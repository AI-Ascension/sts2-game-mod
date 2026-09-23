// SPDX-License-Identifier: MIT

//! The validated, immutable owner inventory of supported rules.

use super::GameFactsError;
use super::model::{
    FactsBuildBinding, FactsRepresentation, FactsRuleEntry, FactsUnsupportedCombination,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION,
};
use super::validation::validate_inventory;

/// One owner inventory: the build it was taken from, the representation it is carried in, the rules
/// it supports, and the combinations it declares it cannot represent exactly.
///
/// An inventory is validated once, by [`Self::new`], and is immutable afterwards, so every read is a
/// read of a value the owner already accepted or refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsInventory {
    producer_version: String,
    build: FactsBuildBinding,
    representation: FactsRepresentation,
    rules: Vec<FactsRuleEntry>,
    unsupported: Vec<FactsUnsupportedCombination>,
}

impl FactsInventory {
    /// Validates one owner inventory and retains it behind an immutable value.
    ///
    /// The inventory is closed: an empty rule set, a rule that copies no typed input, a repeated
    /// rule id, a repeated input name and a combination that names an undeclared rule are all
    /// refused here rather than allowed to read as an exact result later.
    pub fn new(
        build: FactsBuildBinding,
        representation: FactsRepresentation,
        rules: Vec<FactsRuleEntry>,
        unsupported: Vec<FactsUnsupportedCombination>,
    ) -> Result<Self, GameFactsError> {
        validate_inventory(&build, &representation, &rules, &unsupported)?;
        Ok(Self {
            producer_version: GAME_FACTS_REFERENCE_PRODUCER_VERSION.to_owned(),
            build,
            representation,
            rules,
            unsupported,
        })
    }

    /// Returns the owner-local producer identity this inventory was built with.
    #[must_use]
    pub fn producer_version(&self) -> &str {
        &self.producer_version
    }

    /// Returns the exact build and mode this inventory was taken from.
    #[must_use]
    pub fn build(&self) -> &FactsBuildBinding {
        &self.build
    }

    /// Returns the structured representation rule facts are carried in.
    #[must_use]
    pub fn representation(&self) -> &FactsRepresentation {
        &self.representation
    }

    /// Returns every declared rule, in the order the owner declared them.
    #[must_use]
    pub fn rules(&self) -> &[FactsRuleEntry] {
        &self.rules
    }

    /// Returns every combination the owner declares it cannot represent exactly.
    #[must_use]
    pub fn unsupported_combinations(&self) -> &[FactsUnsupportedCombination] {
        &self.unsupported
    }

    /// Returns one declared rule, when the owner declares it.
    #[must_use]
    pub fn rule(&self, rule_id: &str) -> Option<&FactsRuleEntry> {
        self.rules.iter().find(|rule| rule.rule_id == rule_id)
    }

    /// Returns whether this rule participates in any recorded unsupported combination.
    #[must_use]
    pub fn is_unrepresented(&self, rule_id: &str) -> bool {
        self.unsupported
            .iter()
            .any(|combination| combination.rule_ids.iter().any(|id| id == rule_id))
    }

    /// Returns whether one rule may be read as an exact claim.
    ///
    /// A rule reads as exact only when the owner declares it *and* its own evidence is host
    /// confirmed *and* it takes part in no unsupported combination. A confirmed rule whose
    /// interaction with another rule is unrepresented is not exact, so an unsupported interaction
    /// never returns an exact-looking result.
    #[must_use]
    pub fn is_exact_claim(&self, rule_id: &str) -> bool {
        let Some(rule) = self.rule(rule_id) else {
            return false;
        };
        rule.evidence.supports_exact_claim() && !self.is_unrepresented(rule_id)
    }
}
