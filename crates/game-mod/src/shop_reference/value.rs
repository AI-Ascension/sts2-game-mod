// SPDX-License-Identifier: MIT

use super::{ShopField, ShopFieldStatus, ShopText, ShopUnavailableReason};

/// Amount of one named currency.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopMoney {
    /// Currency unit definition identity.
    pub currency_id: String,
    /// Amount in that unit, which may be a formula instead of a number.
    pub amount: ShopNumber,
}

impl ShopMoney {
    /// Returns the fixed amount when the source reported one.
    #[must_use]
    pub const fn fixed_amount(&self) -> Option<i64> {
        fixed_value(&self.amount)
    }
}

/// A price amount that stays a formula rather than being folded into a total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShopNumber {
    /// The source reported one exact amount.
    Fixed(i64),
    /// The source reported a reference formula for the amount.
    Formula(ShopFormula),
    /// The amount is explicitly unavailable.
    Unavailable(ShopUnavailableReason),
}

impl ShopNumber {
    /// Returns the stable wire token for the amount shape.
    #[must_use]
    pub const fn shape(&self) -> &'static str {
        match self {
            Self::Fixed(_) => "fixed",
            Self::Formula(_) => "formula",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

/// Reference formula retained instead of being evaluated into an invented total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopFormula {
    /// Source-defined formula identity.
    pub formula_id: String,
    /// Rounding the source documents for this formula.
    pub rounding: ShopRoundingMode,
    /// Formula inputs the host could not resolve, retained rather than assumed.
    pub unresolved_inputs: Vec<String>,
}

/// Rounding behavior that is named rather than inferred.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopRoundingMode {
    /// The source documents an exact amount with no rounding step.
    Exact,
    /// Round toward negative infinity.
    Down,
    /// Round half away from zero.
    HalfUp,
    /// Round toward positive infinity.
    Up,
    /// Round half to even.
    NearestEven,
    /// Source could not classify the rounding behavior.
    Unknown,
}

/// Role one pricing contributor plays in the displayed price.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopPricingContributorKind {
    /// The base price for the item definition.
    Base,
    /// A rarity-dependent adjustment.
    Rarity,
    /// A mode- or ascension-dependent adjustment.
    Mode,
    /// An explicit sale adjustment.
    Sale,
    /// A stacked discount adjustment.
    Discount,
    /// A restock-dependent adjustment.
    Restock,
    /// A modifier-dependent adjustment.
    Modifier,
    /// A currency-conversion adjustment.
    Currency,
    /// Owner-defined contributor kind.
    Custom(String),
    /// Source could not classify the contributor.
    Unknown,
}

/// One named contributor to a documented or displayed price.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopPricingContributor {
    /// Source-defined contributor identity.
    pub contributor_id: String,
    /// Role this contributor plays.
    pub kind: ShopPricingContributorKind,
    /// Localized contributor label.
    pub label: ShopText,
    /// Signed adjustment, which may itself be a formula.
    pub delta: ShopNumber,
}

/// Explicit reason a purchase is blocked, kept separate from the price it would cost.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopBlockedReason {
    /// The observed balance is below the displayed price.
    Unaffordable,
    /// The destination has no free slot.
    FullSlot,
    /// A stated eligibility requirement is unmet.
    NotEligible,
    /// The entry is sold out for the current generation.
    SoldOut,
    /// A capacity limit is exhausted.
    CapacityExhausted,
    /// The next restock generation has not been observed yet.
    RestockPending,
    /// Source could not classify the reason.
    Unknown,
}

/// Price of one entry or service, separating documented rules from observed amounts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopPrice {
    /// Currency unit the price is denominated in.
    pub currency: ShopField<String>,
    /// Currently observed price.
    pub displayed: ShopField<ShopMoney>,
    /// Documented or reference price, which may be a formula.
    pub documented: ShopField<ShopMoney>,
    /// Named pricing contributors in source order.
    pub contributors: Vec<ShopPricingContributor>,
    /// Availability of the contributor collection after scope withholding.
    pub contributors_status: ShopFieldStatus,
    /// Rounding the source documents for the displayed price.
    pub rounding: ShopRoundingMode,
    /// Formula inputs the host could not resolve.
    pub unresolved_inputs: Vec<String>,
    /// Explicit reason a purchase would be refused right now.
    pub blocked_reason: ShopField<ShopBlockedReason>,
}

impl ShopPrice {
    /// Returns whether the displayed amount contradicts the documented amount.
    #[must_use]
    pub fn matches_documented(&self) -> Option<bool> {
        let displayed = self.displayed.value()?.fixed_amount()?;
        let documented = self.documented.value()?.fixed_amount()?;
        Some(displayed == documented)
    }
}

/// Returns the fixed value of an amount that was reported as a number.
#[must_use]
pub(super) const fn fixed_value(number: &ShopNumber) -> Option<i64> {
    match number {
        ShopNumber::Fixed(value) => Some(*value),
        ShopNumber::Formula(_) | ShopNumber::Unavailable(_) => None,
    }
}

/// Returns the fixed value as a non-negative percentage when the source reported one.
#[must_use]
pub(super) fn fixed_percent(number: &ShopNumber) -> Option<u32> {
    let value = fixed_value(number)?;
    u32::try_from(value).ok()
}
