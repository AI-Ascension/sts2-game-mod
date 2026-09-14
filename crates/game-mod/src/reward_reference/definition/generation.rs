// SPDX-License-Identifier: MIT

use super::super::model::{
    REWARD_MAX_MODIFIERS, REWARD_MAX_PARAMETERS, REWARD_MAX_RARITY_WEIGHTS,
    REWARD_MAX_REQUIREMENTS, RewardEvidence, RewardField, RewardNumericValue, RewardProbability,
    RewardQuantity, RewardRarityWeight, RewardSemanticReference, RewardText, RewardVisibility,
};

/// One visible parameter retained without evaluating hidden state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardParameter {
    /// Stable parameter identity.
    pub parameter_id: String,
    /// Localized/source-defined label.
    pub label: RewardText,
    /// Optional unit.
    pub unit: Option<String>,
    /// Fixed, formula-backed, or unavailable value.
    pub value: RewardNumericValue,
}

/// Coarse generation-rule requirement category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardRequirementKind {
    /// A rarity tier requirement.
    Rarity,
    /// A reward-pool membership requirement.
    PoolMembership,
    /// Progression or act-order requirement.
    Progression,
    /// Character or loadout identity.
    Character,
    /// Destination capacity requirement.
    Capacity,
    /// Active-modifier requirement.
    Modifier,
    /// A minimum or exact resource amount.
    Resource,
    /// Owner rule reference.
    Rule,
    /// Owner-defined predicate.
    Custom(String),
    /// Source could not classify the predicate.
    Unknown,
}

/// One typed requirement for a generation rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardRequirement {
    /// Stable requirement identity.
    pub requirement_id: String,
    /// Predicate category.
    pub kind: RewardRequirementKind,
    /// Localized/source-defined requirement label.
    pub label: RewardText,
    /// Typed parameters used by the requirement.
    pub parameters: Vec<RewardParameter>,
    /// Typed rule/content links.
    pub references: Vec<RewardSemanticReference>,
    /// Visibility of the static requirement.
    pub visibility: RewardVisibility,
}

/// Coarse reward modifier category.
///
/// Named add/remove/quantity/reroll/rarity categories are the only values this producer assigns a
/// sign rule to; owner-defined, rule-backed, and unknown modifiers leave their effect unspecified.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardModifierKind {
    /// Add an item to the candidate pool.
    AddItem,
    /// Remove an item from the candidate pool.
    RemoveItem,
    /// Increase an offered quantity.
    IncreaseQuantity,
    /// Decrease an offered quantity.
    DecreaseQuantity,
    /// Reroll a generated selection.
    Reroll,
    /// Upgrade a generated rarity tier.
    RarityUpgrade,
    /// Downgrade a generated rarity tier.
    RarityDowngrade,
    /// Owner rule reference.
    Rule,
    /// Owner-defined modifier.
    Custom(String),
    /// A modifier is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the modifier.
    Unknown,
}

/// One modifier applied to a reward generation rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardModifier {
    /// Stable modifier identity scoped by the rule.
    pub modifier_id: String,
    /// Modifier category.
    pub kind: RewardModifierKind,
    /// Localized/source-defined modifier label.
    pub label: RewardText,
    /// Fixed, formula-backed, or unavailable amount.
    pub amount: RewardQuantity,
    /// Optional condition identity that gates the modifier.
    pub condition: RewardField<String>,
    /// Owner rule reference.
    pub rule_reference: RewardField<String>,
    /// Typed rule/content links.
    pub references: Vec<RewardSemanticReference>,
    /// Evidence label for this modifier.
    pub evidence: RewardEvidence,
    /// Visibility of the static modifier.
    pub visibility: RewardVisibility,
}

/// Static reward-generation pool, rarity, eligibility, and modifier rule.
///
/// A rule states the *reference* pool and its evidence-qualified probability. It is never a
/// sampled roll: an unavailable probability stays explicit and a future unrevealed roll is not
/// represented as a value here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardGenerationRule {
    /// Stable rule identity scoped by the reward definition.
    pub rule_id: String,
    /// Localized/source-defined rule label.
    pub label: RewardText,
    /// Candidate pool members; an unavailable pool is never an observed-empty one.
    pub pool: RewardField<Vec<RewardSemanticReference>>,
    /// Weighted rarity entries for the pool.
    pub rarity_weights: Vec<RewardRarityWeight>,
    /// Availability of the rarity weights after scope withholding.
    pub rarity_status: super::super::model::RewardFieldStatus,
    /// Eligibility predicates for this generation rule.
    pub eligibility: Vec<RewardRequirement>,
    /// Availability of the eligibility predicates after scope withholding.
    pub eligibility_status: super::super::model::RewardFieldStatus,
    /// Modifier rules applied to the generated selection.
    pub modifiers: Vec<RewardModifier>,
    /// Availability of the modifiers after scope withholding.
    pub modifiers_status: super::super::model::RewardFieldStatus,
    /// Evidence-qualified probability for this rule; never a sampled result.
    pub probability: RewardProbability,
    /// Typed rule/content links.
    pub references: Vec<RewardSemanticReference>,
    /// Evidence label for this rule.
    pub evidence: RewardEvidence,
    /// Visibility of the static rule.
    pub visibility: RewardVisibility,
}

/// Maximum rarity weights accepted on one generation rule.
pub const REWARD_RARITY_WEIGHT_LIMIT: usize = REWARD_MAX_RARITY_WEIGHTS;

/// Maximum requirements accepted on one generation rule.
pub const REWARD_REQUIREMENT_LIMIT: usize = REWARD_MAX_REQUIREMENTS;

/// Maximum modifiers accepted on one generation rule.
pub const REWARD_MODIFIER_LIMIT: usize = REWARD_MAX_MODIFIERS;

/// Maximum parameters accepted on one requirement.
pub const REWARD_PARAMETER_LIMIT: usize = REWARD_MAX_PARAMETERS;
