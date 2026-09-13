// SPDX-License-Identifier: MIT

use super::error::CardDefinitionInputError;
use super::model::{CardFormula, CardUnavailableReason, validate_identity_value};

/// Provenance attached to an acquisition or unlock rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardRuleProvenance {
    /// Owner-defined source kind, not a path or host type name.
    pub source_kind: String,
    /// Source revision copied with the rule.
    pub source_revision: String,
}

impl CardRuleProvenance {
    /// Creates bounded provenance for one rule.
    pub fn new(
        source_kind: impl Into<String>,
        source_revision: impl Into<String>,
    ) -> Result<Self, CardDefinitionInputError> {
        let source_kind = source_kind.into();
        let source_revision = source_revision.into();
        validate_identity_value(&source_kind, "source_kind")?;
        validate_identity_value(&source_revision, "source_revision")?;
        Ok(Self {
            source_kind,
            source_revision,
        })
    }
}

/// Acquisition channel retained as a typed owner-local value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardAcquisitionKind {
    /// Character or shared pool acquisition.
    CharacterPool,
    /// Combat or event reward selection.
    Reward,
    /// Shop inventory.
    Shop,
    /// Event-specific acquisition.
    Event,
    /// A generated card source.
    Generated,
    /// A status card source.
    Status,
    /// A curse card source.
    Curse,
    /// A mod-owned acquisition source.
    ModOrigin,
    /// An owner-defined channel.
    Custom(String),
}

/// Acquisition or unlock condition copied without executing it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardRuleCondition {
    /// The rule has no additional condition.
    Always,
    /// A character or pool identity is required.
    RequiresCharacter(String),
    /// A source-defined progression flag is required.
    RequiresFlag(String),
    /// Another card or definition identity is required.
    RequiresCard(String),
    /// An event identity is required.
    RequiresEvent(String),
    /// A dynamic rule reference with unresolved inputs.
    Formula(CardFormula),
    /// The source cannot expose this condition.
    Unavailable(CardUnavailableReason),
}

/// One static acquisition rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardAcquisitionRule {
    /// Typed acquisition channel.
    pub kind: CardAcquisitionKind,
    /// Condition that gates this channel.
    pub condition: CardRuleCondition,
    /// Source provenance for the rule.
    pub provenance: CardRuleProvenance,
}

/// Unlock condition and provenance, kept separate from acquisition channels.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardUnlockRule {
    /// Condition required before the definition is eligible.
    pub condition: CardRuleCondition,
    /// Source provenance for the unlock rule.
    pub provenance: CardRuleProvenance,
}

pub(super) fn validate_rule_provenance(
    provenance: &CardRuleProvenance,
) -> Result<(), CardDefinitionInputError> {
    validate_identity_value(&provenance.source_kind, "source_kind")?;
    validate_identity_value(&provenance.source_revision, "source_revision")?;
    Ok(())
}

pub(super) fn validate_acquisition_kind(
    kind: &CardAcquisitionKind,
) -> Result<(), CardDefinitionInputError> {
    if let CardAcquisitionKind::Custom(value) = kind {
        validate_identity_value(value, "acquisition_kind")?;
    }
    Ok(())
}

pub(super) fn validate_condition(
    condition: &CardRuleCondition,
) -> Result<(), CardDefinitionInputError> {
    match condition {
        CardRuleCondition::RequiresCharacter(value) => {
            validate_identity_value(value, "condition_character")
        }
        CardRuleCondition::RequiresFlag(value) => validate_identity_value(value, "condition_flag"),
        CardRuleCondition::RequiresCard(value) => validate_identity_value(value, "condition_card"),
        CardRuleCondition::RequiresEvent(value) => {
            validate_identity_value(value, "condition_event")
        }
        CardRuleCondition::Formula(formula) => super::validation::validate_formula(formula),
        CardRuleCondition::Always | CardRuleCondition::Unavailable(_) => Ok(()),
    }
}
