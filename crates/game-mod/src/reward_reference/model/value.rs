// SPDX-License-Identifier: MIT

use super::{REWARD_MAX_FORMULA_INPUTS, RewardEvidence, validate_identity, validate_text};

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardUnavailableReason {
    /// The source supports the field but did not observe it.
    NotObserved,
    /// The supported host/build has no extractor for the field.
    Unsupported,
    /// The caller's source scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
    /// The field has no meaning for the selected definition.
    NotApplicable,
}

/// Coarse availability status for one source-owned reward field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardFieldStatus {
    /// The source observed the field, including a known empty value.
    Available,
    /// Some collection entries were withheld by the selected scope.
    ///
    /// The visible remainder is still returned, but the collection must not be read as complete.
    Partial,
    /// The field has no meaning for the selected definition.
    NotApplicable,
    /// The field is supported but was not observed.
    NotObserved,
    /// No extractor is supported for the field.
    Unsupported,
    /// The source denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
}

impl RewardUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> RewardFieldStatus {
        match self {
            Self::NotObserved => RewardFieldStatus::NotObserved,
            Self::Unsupported => RewardFieldStatus::Unsupported,
            Self::Denied => RewardFieldStatus::Denied,
            Self::Failed => RewardFieldStatus::Failed,
            Self::Unknown => RewardFieldStatus::Unknown,
            Self::NotApplicable => RewardFieldStatus::NotApplicable,
        }
    }
}

/// A source field that distinguishes an observed empty value from an unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(RewardUnavailableReason),
}

impl<T> RewardField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> RewardFieldStatus {
        match self {
            Self::Available(_) => RewardFieldStatus::Available,
            Self::Unavailable(reason) => reason.status(),
        }
    }

    /// Returns the observed value, including an observed empty collection.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            Self::Unavailable(_) => None,
        }
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(RewardUnavailableReason),
}

impl RewardText {
    /// Creates a bounded available localized text value.
    pub fn available(value: impl Into<String>) -> Result<Self, super::super::RewardCatalogError> {
        let value = value.into();
        validate_text(&value, "text")?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: RewardUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// A source-owned dynamic formula without an executable expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardFormula {
    /// Stable owner rule identity.
    pub rule_reference: String,
    /// Inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

impl RewardFormula {
    /// Creates a bounded formula reference with explicit unresolved inputs.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, super::super::RewardCatalogError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        validate_identity(&formula.rule_reference, "formula_rule")?;
        if formula.unresolved_inputs.len() > REWARD_MAX_FORMULA_INPUTS {
            return Err(super::super::RewardCatalogError::InvalidInput(
                "formula_inputs",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for input in &formula.unresolved_inputs {
            validate_identity(input, "formula_input")?;
            if !seen.insert(input.as_str()) {
                return Err(super::super::RewardCatalogError::InvalidInput(
                    "duplicate_formula_input",
                ));
            }
        }
        Ok(formula)
    }
}

/// A fixed, formula-backed, or unavailable numeric value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardNumericValue {
    /// Value observed directly in the owner definition.
    Fixed(i64),
    /// Dynamic value linked to a non-executable owner rule.
    Formula(RewardFormula),
    /// Value was not safely available.
    Unavailable(RewardUnavailableReason),
}

/// An exact typed quantity, preserving currency units and visible modified values.
///
/// `base_amount` is the unmodified definition amount and `visible_amount` is the exact value the
/// player observes after any applied modifiers. A currency quantity must carry an observed unit
/// identity; card, relic, and potion counts leave the unit as explicitly not-applicable rather
/// than inventing one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardQuantity {
    /// Currency or owner-defined unit identity, when one applies.
    pub unit: RewardField<String>,
    /// Unmodified base amount.
    pub base_amount: RewardNumericValue,
    /// Exact visible amount after applied modifiers.
    pub visible_amount: RewardNumericValue,
    /// Whether the visible amount differs from the base amount.
    pub modified: RewardField<bool>,
}

impl RewardQuantity {
    /// Creates a fixed quantity with an explicit unit availability.
    #[must_use]
    pub fn fixed(unit: RewardField<String>, amount: i64, modified: RewardField<bool>) -> Self {
        Self {
            unit,
            base_amount: RewardNumericValue::Fixed(amount),
            visible_amount: RewardNumericValue::Fixed(amount),
            modified,
        }
    }

    /// Returns the observed unit identity, if any.
    #[must_use]
    pub fn unit_value(&self) -> Option<&str> {
        self.unit.value().map(String::as_str)
    }
}

/// Evidence-qualified probability retained without evaluating hidden RNG.
///
/// This states a reference probability for a possible generation outcome. It is never a sampled
/// result and never invents a value: an unknown probability is [`RewardProbability::Unavailable`],
/// and a probability that depends on unrevealed state is a [`RewardProbability::Conditional`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardProbability {
    /// Exact rational probability and its evidence label.
    Exact {
        /// Numerator.
        numerator: u32,
        /// Positive denominator.
        denominator: u32,
        /// Provenance/evidence for the value.
        evidence: RewardEvidence,
    },
    /// Probability governed by a conditional or unknown owner rule.
    Rule {
        /// Stable owner rule identity; the value is never invented.
        rule_reference: String,
        /// Provenance/evidence for the rule link.
        evidence: RewardEvidence,
    },
    /// Probability that depends on an explicit condition that is not revealed here.
    Conditional {
        /// Stable owner rule identity; the value is never invented.
        rule_reference: String,
        /// Stable owner condition identity the probability depends on.
        condition_reference: String,
        /// Provenance/evidence for the rule link.
        evidence: RewardEvidence,
    },
    /// No safe probability was available.
    Unavailable(RewardUnavailableReason),
}

/// Typed semantic reference to a reward, content definition, or rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardSemanticReference {
    /// Reference family.
    pub kind: RewardSemanticReferenceKind,
    /// Stable owner-defined identity.
    pub id: String,
    /// Localized/source-defined label.
    pub label: RewardText,
}

/// Supported semantic reference families.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardSemanticReferenceKind {
    /// Another reward offer definition in the content manifest.
    Reward,
    /// A card definition in the content manifest.
    Card,
    /// A relic definition in the content manifest.
    Relic,
    /// A potion definition in the content manifest.
    Potion,
    /// A currency-unit definition in the content manifest.
    Currency,
    /// An offered item within the same reward definition.
    Item,
    /// A generation, rarity, or transition rule.
    Rule,
    /// A source-owned condition.
    Condition,
    /// A modifier rule.
    Modifier,
    /// An owner-defined content family not otherwise named here.
    Content {
        /// Owner-defined entity family.
        entity_kind: String,
    },
    /// A family not classified by the source.
    Unknown,
}

/// One weighted rarity entry in a reward generation rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardRarityWeight {
    /// Owner-defined rarity identity, never collapsed into a known tier.
    pub rarity: String,
    /// Weight quantity, preserving its unit and visible value.
    pub weight: RewardQuantity,
    /// Evidence-qualified selection probability for this rarity entry.
    pub probability: RewardProbability,
    /// Evidence label for this weight.
    pub evidence: RewardEvidence,
}

impl RewardRarityWeight {
    /// Creates a bounded rarity weight.
    pub fn new(
        rarity: impl Into<String>,
        weight: RewardQuantity,
        probability: RewardProbability,
        evidence: RewardEvidence,
    ) -> Result<Self, super::super::RewardCatalogError> {
        let rarity = rarity.into();
        validate_identity(&rarity, "rarity")?;
        Ok(Self {
            rarity,
            weight,
            probability,
            evidence,
        })
    }
}
