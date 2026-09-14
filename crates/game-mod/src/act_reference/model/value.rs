// SPDX-License-Identifier: MIT

use super::{ACT_MAX_FORMULA_INPUTS, ActEvidence, validate_identity, validate_text};

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActUnavailableReason {
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

/// Coarse availability status for one source-owned act field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActFieldStatus {
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

impl ActUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> ActFieldStatus {
        match self {
            Self::NotObserved => ActFieldStatus::NotObserved,
            Self::Unsupported => ActFieldStatus::Unsupported,
            Self::Denied => ActFieldStatus::Denied,
            Self::Failed => ActFieldStatus::Failed,
            Self::Unknown => ActFieldStatus::Unknown,
            Self::NotApplicable => ActFieldStatus::NotApplicable,
        }
    }
}

/// A source field that distinguishes an observed empty value from an unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(ActUnavailableReason),
}

impl<T> ActField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> ActFieldStatus {
        match self {
            Self::Available(_) => ActFieldStatus::Available,
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
pub enum ActText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(ActUnavailableReason),
}

impl ActText {
    /// Creates a bounded available localized text value.
    pub fn available(value: impl Into<String>) -> Result<Self, super::super::ActReferenceError> {
        let value = value.into();
        validate_text(&value, "text")?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: ActUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// A source-owned dynamic formula without an executable expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActFormula {
    /// Stable owner rule identity.
    pub rule_reference: String,
    /// Inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

impl ActFormula {
    /// Creates a bounded formula reference with explicit unresolved inputs.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, super::super::ActReferenceError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        validate_identity(&formula.rule_reference, "formula_rule")?;
        if formula.unresolved_inputs.len() > ACT_MAX_FORMULA_INPUTS {
            return Err(super::super::ActReferenceError::InvalidInput(
                "formula_inputs",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for input in &formula.unresolved_inputs {
            validate_identity(input, "formula_input")?;
            if !seen.insert(input.as_str()) {
                return Err(super::super::ActReferenceError::InvalidInput(
                    "duplicate_formula_input",
                ));
            }
        }
        Ok(formula)
    }
}

/// A fixed, formula-backed, or unavailable numeric value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActNumericValue {
    /// Value observed directly in the owner definition.
    Fixed(i64),
    /// Dynamic value linked to a non-executable owner rule.
    Formula(ActFormula),
    /// Value was not safely available.
    Unavailable(ActUnavailableReason),
}

/// A generation weight or probability retained without evaluating hidden RNG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerationWeight {
    /// Exact rational weight/probability and its evidence label.
    Exact {
        /// Numerator.
        numerator: u32,
        /// Positive denominator.
        denominator: u32,
        /// Provenance/evidence for the value.
        evidence: ActEvidence,
    },
    /// Weight governed by a conditional or unknown owner rule.
    Rule {
        /// Stable owner rule identity; the value is never invented.
        rule_reference: String,
        /// Provenance/evidence for the rule link.
        evidence: ActEvidence,
    },
    /// No safe weight was available.
    Unavailable(ActUnavailableReason),
}

/// Typed semantic reference to an act, encounter, room category, or content definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActSemanticReference {
    /// Reference family.
    pub kind: ActSemanticReferenceKind,
    /// Stable owner-defined identity.
    pub id: String,
    /// Localized/source-defined label.
    pub label: ActText,
}

/// Supported semantic reference families.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActSemanticReferenceKind {
    /// An enemy definition in the content manifest.
    Enemy,
    /// An encounter definition in the content manifest.
    Encounter,
    /// Another act definition in the content manifest.
    Act,
    /// A room/node category, which has no shared manifest family.
    RoomCategory,
    /// An effect definition or rule link.
    Effect,
    /// A generation, timing, or transition rule.
    Rule,
    /// A source-owned condition.
    Condition,
    /// An owner-defined content family not otherwise named here.
    Content {
        /// Owner-defined entity family.
        entity_kind: String,
    },
    /// A family not classified by the source.
    Unknown,
}
