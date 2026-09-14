// SPDX-License-Identifier: MIT

use super::{EVENT_MAX_FORMULA_INPUTS, EventEvidence, validate_identity, validate_text};

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventUnavailableReason {
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

/// Coarse availability status for one source-owned event field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventFieldStatus {
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

impl EventUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> EventFieldStatus {
        match self {
            Self::NotObserved => EventFieldStatus::NotObserved,
            Self::Unsupported => EventFieldStatus::Unsupported,
            Self::Denied => EventFieldStatus::Denied,
            Self::Failed => EventFieldStatus::Failed,
            Self::Unknown => EventFieldStatus::Unknown,
            Self::NotApplicable => EventFieldStatus::NotApplicable,
        }
    }
}

/// A source field that distinguishes an observed empty value from an unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(EventUnavailableReason),
}

impl<T> EventField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> EventFieldStatus {
        match self {
            Self::Available(_) => EventFieldStatus::Available,
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
pub enum EventText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(EventUnavailableReason),
}

impl EventText {
    /// Creates a bounded available localized text value.
    pub fn available(value: impl Into<String>) -> Result<Self, super::super::EventCatalogError> {
        let value = value.into();
        validate_text(&value, "text")?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: EventUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// A source-owned dynamic formula without an executable expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventFormula {
    /// Stable owner rule identity.
    pub rule_reference: String,
    /// Inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

impl EventFormula {
    /// Creates a bounded formula reference with explicit unresolved inputs.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, super::super::EventCatalogError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        validate_identity(&formula.rule_reference, "formula_rule")?;
        if formula.unresolved_inputs.len() > EVENT_MAX_FORMULA_INPUTS {
            return Err(super::super::EventCatalogError::InvalidInput(
                "formula_inputs",
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        for input in &formula.unresolved_inputs {
            validate_identity(input, "formula_input")?;
            if !seen.insert(input.as_str()) {
                return Err(super::super::EventCatalogError::InvalidInput(
                    "duplicate_formula_input",
                ));
            }
        }
        Ok(formula)
    }
}

/// A fixed, formula-backed, or unavailable numeric value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventNumericValue {
    /// Value observed directly in the owner definition.
    Fixed(i64),
    /// Dynamic value linked to a non-executable owner rule.
    Formula(EventFormula),
    /// Value was not safely available.
    Unavailable(EventUnavailableReason),
}

/// Evidence-qualified probability retained without evaluating hidden RNG.
///
/// This states a reference probability for a possible outcome. It is never a sampled result and
/// never invents a value: an unknown probability is [`EventProbability::Unavailable`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventProbability {
    /// Exact rational probability and its evidence label.
    Exact {
        /// Numerator.
        numerator: u32,
        /// Positive denominator.
        denominator: u32,
        /// Provenance/evidence for the value.
        evidence: EventEvidence,
    },
    /// Probability governed by a conditional or unknown owner rule.
    Rule {
        /// Stable owner rule identity; the value is never invented.
        rule_reference: String,
        /// Provenance/evidence for the rule link.
        evidence: EventEvidence,
    },
    /// No safe probability was available.
    Unavailable(EventUnavailableReason),
}

/// Typed semantic reference to an event, page, option, or content definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventSemanticReference {
    /// Reference family.
    pub kind: EventSemanticReferenceKind,
    /// Stable owner-defined identity.
    pub id: String,
    /// Localized/source-defined label.
    pub label: EventText,
}

/// Supported semantic reference families.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventSemanticReferenceKind {
    /// Another event definition in the content manifest.
    Event,
    /// An encounter definition in the content manifest.
    Encounter,
    /// An enemy definition in the content manifest.
    Enemy,
    /// A relic definition in the content manifest.
    Relic,
    /// A card definition in the content manifest.
    Card,
    /// A potion definition in the content manifest.
    Potion,
    /// An act definition in the content manifest.
    Act,
    /// A narrative page within the same event definition.
    Page,
    /// An option within the same event definition.
    Option,
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
