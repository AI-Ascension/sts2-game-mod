// SPDX-License-Identifier: MIT

//! The validated immutable container for caller-supplied game-facts inventory values.

use super::GameFactsError;
use super::model::{
    FactsBuildBinding, FactsRepresentation, FactsRuleEntry, FactsUnsupportedCombination,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION,
};
use super::validation::{validate_inventory, validate_manifest_binding};
use crate::ContentManifest;

/// One caller-supplied inventory value: its claimed build and representation, declared rules, and
/// combinations it says it cannot represent exactly.
///
/// An inventory is validated once, by [`Self::new`], and is immutable afterwards, so every read is
/// of a value that passed local validation. This does not authenticate its source or prove a
/// coherent extraction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsInventory {
    producer_version: String,
    build: FactsBuildBinding,
    representation: FactsRepresentation,
    rules: Vec<FactsRuleEntry>,
    unsupported: Vec<FactsUnsupportedCombination>,
    captured_inventory_revision: Option<String>,
    captured_locale: Option<String>,
}

impl FactsInventory {
    /// Validates one caller-supplied inventory and retains it behind an immutable value.
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
        Self::validated(build, representation, rules, unsupported, None, None)
    }

    /// Captures a supplied manifest value for later identity-consistency checks.
    ///
    /// This derives build identity and the owner-local cursor from the value and records its
    /// inventory revision and locale. Since manifest fields are public, this does not authenticate
    /// producer origin or prove that facts and manifest came from one coherent source read. The
    /// caller owns that evidence boundary. The mode is supplied separately because ContentManifest
    /// has none.
    pub fn new_for_manifest(
        manifest: &ContentManifest,
        mode_id: impl Into<String>,
        representation: FactsRepresentation,
        rules: Vec<FactsRuleEntry>,
        unsupported: Vec<FactsUnsupportedCombination>,
    ) -> Result<Self, GameFactsError> {
        validate_manifest_binding(manifest)?;
        let build = FactsBuildBinding {
            build_id: manifest.game_build.clone(),
            mode_id: mode_id.into(),
            manifest: manifest.cursor_binding(),
        };
        Self::validated(
            build,
            representation,
            rules,
            unsupported,
            Some(manifest.inventory_revision.clone()),
            Some(manifest.locale.clone()),
        )
    }

    fn validated(
        build: FactsBuildBinding,
        representation: FactsRepresentation,
        rules: Vec<FactsRuleEntry>,
        unsupported: Vec<FactsUnsupportedCombination>,
        captured_inventory_revision: Option<String>,
        captured_locale: Option<String>,
    ) -> Result<Self, GameFactsError> {
        validate_inventory(&build, &representation, &rules, &unsupported)?;
        Ok(Self {
            producer_version: GAME_FACTS_REFERENCE_PRODUCER_VERSION.to_owned(),
            build,
            representation,
            rules,
            unsupported,
            captured_inventory_revision,
            captured_locale,
        })
    }

    pub(super) fn captured_inventory_revision(&self) -> Option<&str> {
        self.captured_inventory_revision.as_deref()
    }

    pub(super) fn captured_locale(&self) -> Option<&str> {
        self.captured_locale.as_deref()
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

    /// Returns whether caller-supplied labels and inputs pass local exact-claim conditions.
    ///
    /// A rule passes these local conditions only when the caller declares it, labels its evidence
    /// confirmed, fully states every copied input, and places it in no unsupported combination.
    /// A confirmed rule whose interaction with another rule is unrepresented is not exact, so an
    /// unsupported interaction never returns an exact-looking result; likewise a confirmed rule
    /// carrying a conditional or unknown input is not exact, because that input's contribution is
    /// not settled by this inventory. A true result does not authenticate evidence origin or host
    /// support.
    #[must_use]
    pub fn is_exact_claim(&self, rule_id: &str) -> bool {
        let Some(rule) = self.rule(rule_id) else {
            return false;
        };
        rule.evidence.supports_exact_claim()
            && rule.inputs_support_exact_claim()
            && !self.is_unrepresented(rule_id)
    }
}
