// SPDX-License-Identifier: MIT

use super::{
    field::{RestField, RestText},
    model::RestSemanticReference,
};

/// Rounding the documented healing formula applies.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestRoundingMode {
    /// The formula is exact and needs no rounding.
    None,
    /// The formula rounds down.
    Down,
    /// The formula rounds up.
    Up,
    /// The formula rounds to the nearest integer.
    Nearest,
    /// Source did not state the rounding.
    Unknown,
}

/// Kind of one healing modifier, kept distinct from the base fraction it alters.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestHealModifierKind {
    /// A modifier derived from maximum health.
    MaxHealthPercent,
    /// A flat bonus added to the resolved amount.
    FlatBonus,
    /// An ascension-level penalty.
    AscensionPenalty,
    /// A modifier contributed by a held relic.
    RelicBonus,
    /// Owner-defined modifier kind.
    Custom(String),
    /// Source could not classify the modifier.
    Unknown,
}

/// One documented contributor to a healing amount.
///
/// A modifier stays a separate contributor; it is never folded into the base fraction, so a
/// documented formula remains a formula rather than a single invented total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestHealModifier {
    /// Stable modifier identity within its effect.
    pub modifier_id: String,
    /// Localized modifier text.
    pub label: RestText,
    /// Kind of modifier.
    pub kind: RestHealModifierKind,
    /// Resolved percentage contribution, or an explicit non-value.
    pub percent: RestField<i32>,
    /// Definitions this modifier refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Documented healing amount with its contributors kept separate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestHealAmount {
    /// Documented base percentage of maximum health, or an explicit non-value.
    pub base_percent: RestField<u32>,
    /// Documented flat bonus, or an explicit non-value.
    pub flat_bonus: RestField<u32>,
    /// Documented healing modifiers.
    pub modifiers: Vec<RestHealModifier>,
    /// Rounding the documented formula applies.
    pub rounding: RestRoundingMode,
    /// Resolved percentage the source reports, or an explicit non-value.
    ///
    /// This is the only field that may carry a resolved amount, and it is left unavailable when the
    /// source did not supply it: the slice never folds the contributors into a total of its own.
    pub resolved_percent: RestField<u32>,
}

/// Kind of effect a rest option produces.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestEffectKind {
    /// Restore health.
    Heal,
    /// Remove a card from the deck.
    RemoveCard,
    /// Add a card to the deck.
    AddCard,
    /// Transform one card into another.
    TransformCard,
    /// Upgrade a card.
    UpgradeCard,
    /// Mend a card a prior smith damaged.
    MendCard,
    /// Grant a relic.
    GrantRelic,
    /// Hatch a held relic.
    HatchRelic,
    /// Kindle a held relic.
    KindleRelic,
    /// Lift a held relic.
    LiftRelic,
    /// Owner-defined effect kind.
    Custom(String),
    /// A kind is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the effect.
    Unknown,
}

/// One effect a rest option would produce, described without performing it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestEffect {
    /// Stable effect identity within its option.
    pub effect_id: String,
    /// Localized effect text.
    pub label: RestText,
    /// Kind of effect.
    pub kind: RestEffectKind,
    /// Target the effect applies to, or an explicit non-value.
    pub target: RestField<RestSemanticReference>,
    /// Documented healing amount, present only for a healing effect.
    pub healing: Option<RestHealAmount>,
    /// Definitions this effect refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Whether a prospective comparison is documented or currently observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestComparisonBasis {
    /// The comparison restates a documented definition or rule.
    Definition,
    /// The comparison restates a value currently observed by the source.
    Observed,
}

/// Kind of one prospective before/after change.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestProspectiveChangeKind {
    /// A card cost changes.
    CardCost,
    /// A card is upgraded.
    CardUpgrade,
    /// Maximum health changes.
    MaxHealth,
    /// Current health changes.
    CurrentHealth,
    /// Gold changes.
    Gold,
    /// A held relic changes state.
    RelicState,
    /// A card is added to the deck.
    CardAdded,
    /// A card is removed from the deck.
    CardRemoved,
    /// Owner-defined change kind.
    Custom(String),
    /// Source could not classify the change.
    Unknown,
}

/// One prospective before/after change, described without performing the option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestProspectiveChange {
    /// Stable change identity within its comparison.
    pub change_id: String,
    /// Localized change text.
    pub label: RestText,
    /// Kind of change.
    pub kind: RestProspectiveChangeKind,
    /// Target the change applies to, or an explicit non-value.
    pub target: RestField<RestSemanticReference>,
    /// Value before the option, or an explicit non-value.
    pub before: RestField<String>,
    /// Value after the option, or an explicit non-value.
    pub after: RestField<String>,
    /// Whether this change is documented or currently observed.
    pub basis: RestComparisonBasis,
    /// Definitions this change refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Documented upgrade choice detail for one card upgrade comparison.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestUpgradePreview {
    /// Candidate the preview describes.
    pub candidate: RestSemanticReference,
    /// Card text or cost before the upgrade, or an explicit non-value.
    pub before: RestField<String>,
    /// Card text or cost after the upgrade, or an explicit non-value.
    pub after: RestField<String>,
    /// Definitions this preview refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Prospective comparison for one rest option.
///
/// `complete` is derived rather than accepted from the source: a comparison that carries any
/// unavailable before/after value is reported incomplete, so a partial result is never labelled
/// complete.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestProspectiveComparison {
    /// Option identity the comparison describes.
    ///
    /// The comparison is carried inside one option, so it names that option's identity rather than
    /// carrying a second catalog binding of its own.
    pub option_id: String,
    /// Whether the comparison is documented or currently observed.
    pub basis: RestComparisonBasis,
    /// Prospective changes in deterministic order.
    pub changes: Vec<RestProspectiveChange>,
    /// Documented upgrade choice detail, or an explicit non-value.
    pub upgrade: RestField<RestUpgradePreview>,
    /// Whether every change carries a resolved before and after value.
    pub complete: bool,
}

impl RestProspectiveComparison {
    /// Builds a comparison whose completeness is derived from its own changes.
    #[must_use]
    pub fn from_changes(
        option_id: String,
        basis: RestComparisonBasis,
        changes: Vec<RestProspectiveChange>,
        upgrade: RestField<RestUpgradePreview>,
    ) -> Self {
        let complete = !changes.is_empty()
            && changes
                .iter()
                .all(|change| change.before.is_available() && change.after.is_available());
        Self {
            option_id,
            basis,
            changes,
            upgrade,
            complete,
        }
    }
}
