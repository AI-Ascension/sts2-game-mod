// SPDX-License-Identifier: MIT

use super::error::CardDefinitionInputError;

/// Owner-defined card type carried without guessing a game vocabulary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardType(String);

impl CardType {
    /// Creates a bounded card type token.
    pub fn new(value: impl Into<String>) -> Result<Self, CardDefinitionInputError> {
        let value = value.into();
        validate_identity(&value, "card_type")?;
        Ok(Self(value))
    }

    /// Returns the owner-defined type token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owner-defined rarity carried without freezing an external enum.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardRarity(String);

impl CardRarity {
    /// Creates a bounded rarity token.
    pub fn new(value: impl Into<String>) -> Result<Self, CardDefinitionInputError> {
        let value = value.into();
        validate_identity(&value, "rarity")?;
        Ok(Self(value))
    }

    /// Returns the owner-defined rarity token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Explicit reason for a source value that was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardUnavailableReason {
    /// The supported source omitted the value from its observable surface.
    NotObserved,
    /// This host/build has no supported extractor for the value.
    Unsupported,
    /// The caller's source scope does not permit the value.
    Denied,
    /// Extraction failed with a bounded sanitized result.
    Failed,
    /// The source could not classify the value.
    Unknown,
    /// A dynamic formula was required but not available.
    FormulaNotObserved,
}

/// A localized value or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardTextValue {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was not available; no placeholder is inferred.
    Unavailable(CardUnavailableReason),
}

impl CardTextValue {
    /// Creates an available bounded text value.
    pub fn available(value: impl Into<String>) -> Result<Self, CardDefinitionInputError> {
        let value = value.into();
        validate_text(&value)?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: CardUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// Optional card metadata with an explicit not-applicable state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardOptionalText {
    /// The source observed a value.
    Available(String),
    /// The metadata has no meaning for this card.
    NotApplicable,
    /// The metadata is supported but was not observed.
    Unavailable(CardUnavailableReason),
}

impl CardOptionalText {
    /// Creates an available bounded optional metadata value.
    pub fn available(value: impl Into<String>) -> Result<Self, CardDefinitionInputError> {
        let value = value.into();
        validate_identity(&value, "optional_text")?;
        Ok(Self::Available(value))
    }

    /// Creates the explicit no-meaning state.
    #[must_use]
    pub const fn not_applicable() -> Self {
        Self::NotApplicable
    }

    /// Creates an explicit unavailable state.
    #[must_use]
    pub const fn unavailable(reason: CardUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// A dynamic rule reference and the inputs that remain unresolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardFormula {
    /// Owner-defined rule identity, never an executable expression.
    pub rule_reference: String,
    /// Explicit inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

impl CardFormula {
    /// Creates a bounded formula reference.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, CardDefinitionInputError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        super::validation::validate_formula(&formula)?;
        Ok(formula)
    }
}

/// Numeric value retained as fixed, dynamic, or explicitly unavailable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardNumericValue {
    /// A value observed in the owner definition.
    Fixed(i32),
    /// A dynamic value linked to a non-executable owner rule.
    Formula(CardFormula),
    /// The value is unavailable and must not be replaced with zero.
    Unavailable(CardUnavailableReason),
}

/// A typed effect parameter value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardEffectValue {
    /// Numeric parameter, including X-cost or dynamic values.
    Number(CardNumericValue),
    /// Boolean parameter.
    Boolean(bool),
    /// Bounded owner-defined text parameter.
    Text(String),
    /// Dynamic non-numeric parameter.
    Formula(CardFormula),
    /// Explicitly unavailable parameter.
    Unavailable(CardUnavailableReason),
}

/// Cost units retained for structured alternate costs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardCostUnit {
    /// Energy spent to play the card.
    Energy,
    /// Health paid by the card.
    Health,
    /// Cards or another owner-defined count.
    Cards,
    /// A source-defined cost unit.
    Custom(String),
}

/// One additional structured cost component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardCostComponent {
    /// Stable owner-defined component identity.
    pub key: String,
    /// Typed amount, including unresolved formulas.
    pub amount: CardNumericValue,
    /// Unit for the amount.
    pub unit: CardCostUnit,
}

/// Structured card cost; X-costs remain explicit formulas or unavailable values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardCost {
    /// Primary energy value.
    pub energy: CardNumericValue,
    /// Whether the owner marks this card as consuming an X-cost input.
    pub x_cost: bool,
    /// Additional health, card, or owner-defined cost components.
    pub additional: Vec<CardCostComponent>,
}

/// Targeting copied from the owner definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardTargeting {
    /// The card has no target.
    None,
    /// The card targets its owner.
    SelfOnly,
    /// One enemy is selected.
    SingleEnemy,
    /// All enemies are affected.
    AllEnemies,
    /// The owner chooses from any legal entity.
    Any,
    /// The owner resolves targeting through a named rule.
    Dynamic(CardFormula),
    /// A source-defined targeting mode.
    Custom(String),
}

/// One keyed effect parameter copied from the source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardEffectParameter {
    /// Stable owner-defined parameter identity.
    pub key: String,
    /// Typed parameter value.
    pub value: CardEffectValue,
}

/// A structural modifier kept distinct from rendered text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardStructuralModifier {
    /// Stable owner-defined modifier identity.
    pub key: String,
    /// Optional typed modifier value.
    pub value: Option<CardEffectValue>,
}

fn validate_identity(value: &str, field: &'static str) -> Result<(), CardDefinitionInputError> {
    if value.is_empty()
        || value.len() > super::CARD_DEFINITION_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(CardDefinitionInputError::InvalidIdentity(field));
    }
    Ok(())
}

fn validate_text(value: &str) -> Result<(), CardDefinitionInputError> {
    if value.len() > super::CARD_DEFINITION_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(CardDefinitionInputError::InvalidText);
    }
    Ok(())
}

pub(super) fn validate_identity_value(
    value: &str,
    field: &'static str,
) -> Result<(), CardDefinitionInputError> {
    validate_identity(value, field)
}

pub(super) fn validate_text_value(value: &str) -> Result<(), CardDefinitionInputError> {
    validate_text(value)
}
