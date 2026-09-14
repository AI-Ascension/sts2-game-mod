// SPDX-License-Identifier: MIT

use super::super::model::{
    EventEvidence, EventField, EventNumericValue, EventSemanticReference, EventText,
    EventVisibility,
};

/// Coarse possible-effect/outcome category.
///
/// Named gain/loss, card, relic, potion, heal, and damage categories are non-negative magnitudes;
/// the subject and the category already encode the direction. `MaxHpChange` alone is a signed
/// delta because a single field represents both growth and reduction. Owner-defined, rule,
/// follow-up, and unknown categories leave the sign unspecified.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventEffectKind {
    /// Gain a card.
    AddCard,
    /// Remove a card.
    RemoveCard,
    /// Upgrade or transform a card.
    ModifyCard,
    /// Gain a relic.
    GainRelic,
    /// Lose a relic.
    LoseRelic,
    /// Gain a potion.
    GainPotion,
    /// Lose or consume a potion.
    LosePotion,
    /// Gain gold.
    GainGold,
    /// Lose gold.
    LoseGold,
    /// Heal HP.
    Heal,
    /// Take damage.
    Damage,
    /// Change maximum HP.
    MaxHpChange,
    /// Advance to another narrative page.
    FollowUp,
    /// Owner rule reference.
    Rule,
    /// Owner-defined effect.
    Custom(String),
    /// An effect is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the effect.
    Unknown,
}

/// One possible effect of an outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEffect {
    /// Stable effect identity scoped by the outcome.
    pub effect_id: String,
    /// Effect category.
    pub kind: EventEffectKind,
    /// Localized/source-defined effect label.
    pub label: EventText,
    /// Fixed, formula-backed, or unavailable amount.
    pub amount: EventNumericValue,
    /// Optional target/resource identity.
    pub target: EventField<String>,
    /// Owner rule reference.
    pub rule_reference: EventField<String>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Evidence label for this effect.
    pub evidence: EventEvidence,
    /// Visibility of the static effect.
    pub visibility: EventVisibility,
}
