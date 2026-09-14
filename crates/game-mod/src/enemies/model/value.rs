// SPDX-License-Identifier: MIT

use super::{ENEMY_MAX_FORMULA_INPUTS, EnemyEvidence, validate_identity, validate_text};

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyUnavailableReason {
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

/// A source field that distinguishes an observed empty value from an unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(EnemyUnavailableReason),
}

impl<T> EnemyField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> EnemyFieldStatus {
        match self {
            Self::Available(_) => EnemyFieldStatus::Available,
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

    /// Creates an explicit unavailable field value.
    #[must_use]
    pub const fn unavailable(reason: EnemyUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// Coarse availability status for one source-owned enemy field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyFieldStatus {
    /// The source observed the field, including a known empty value.
    Available,
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

impl EnemyUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> EnemyFieldStatus {
        match self {
            Self::NotObserved => EnemyFieldStatus::NotObserved,
            Self::Unsupported => EnemyFieldStatus::Unsupported,
            Self::Denied => EnemyFieldStatus::Denied,
            Self::Failed => EnemyFieldStatus::Failed,
            Self::Unknown => EnemyFieldStatus::Unknown,
            Self::NotApplicable => EnemyFieldStatus::NotApplicable,
        }
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(EnemyUnavailableReason),
}

impl EnemyText {
    /// Creates a bounded available localized text value.
    pub fn available(value: impl Into<String>) -> Result<Self, super::super::EnemyCatalogError> {
        let value = value.into();
        if value.is_empty() {
            return Err(super::super::EnemyCatalogError::InvalidInput("text"));
        }
        validate_text(&value, "text")?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: EnemyUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// A source-owned dynamic formula without an executable expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyFormula {
    /// Stable owner rule identity.
    pub rule_reference: String,
    /// Inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

impl EnemyFormula {
    /// Creates a bounded formula reference with explicit unresolved inputs.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, super::super::EnemyCatalogError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        validate_identity(&formula.rule_reference, "formula_rule")?;
        if formula.unresolved_inputs.len() > ENEMY_MAX_FORMULA_INPUTS {
            return Err(super::super::EnemyCatalogError::InvalidInput(
                "formula_inputs",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for input in &formula.unresolved_inputs {
            validate_identity(input, "formula_input")?;
            if !seen.insert(input.as_str()) {
                return Err(super::super::EnemyCatalogError::InvalidInput(
                    "duplicate_formula_input",
                ));
            }
        }
        Ok(formula)
    }
}

/// A fixed, formula-backed, or unavailable numeric value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyNumericValue {
    /// Value observed directly in the owner definition.
    Fixed(i64),
    /// Dynamic value linked to a non-executable owner rule.
    Formula(EnemyFormula),
    /// Value was not safely available.
    Unavailable(EnemyUnavailableReason),
}

/// A static targeting domain used by a move or effect.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyTargetDomain {
    /// The acting enemy itself.
    SelfOnly,
    /// One player target.
    AnyPlayer,
    /// All player targets.
    AllPlayers,
    /// One enemy target.
    AnyEnemy,
    /// All enemy targets.
    AllEnemies,
    /// One source-defined entity.
    SpecificEntity,
    /// The effect has no target.
    None,
    /// The source could not classify the target domain.
    Unknown,
}

/// Targeting domain plus a typed target count where one is available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyTargeting {
    /// Public targeting domain.
    pub domain: EnemyTargetDomain,
    /// Number of affected targets, or an explicit unavailable/formula value.
    pub count: EnemyNumericValue,
}

/// A probability or weight retained without evaluating hidden RNG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyProbability {
    /// Exact rational probability/weight and its evidence label.
    Exact {
        /// Numerator.
        numerator: u32,
        /// Positive denominator.
        denominator: u32,
        /// Provenance/evidence for the value.
        evidence: EnemyEvidence,
    },
    /// Non-executable owner formula with explicit evidence.
    Formula {
        /// Formula and unresolved inputs.
        formula: EnemyFormula,
        /// Provenance/evidence for the formula.
        evidence: EnemyEvidence,
    },
    /// No safe probability was available.
    Unavailable(EnemyUnavailableReason),
}

/// A typed reset boundary for repetition/cooldown restrictions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyResetBoundary {
    /// Never resets during the retained definition lifetime.
    Never,
    /// Reset after an enemy turn.
    Turn,
    /// Reset after a combat round.
    Round,
    /// Reset when the room/combat ends.
    Combat,
    /// Reset at a phase boundary.
    Phase,
    /// Source-defined reset boundary is unknown.
    Unknown,
}
