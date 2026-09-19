// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    entry::ShopSaleState,
    error::ShopCatalogError,
    field::{ShopField, ShopFieldStatus, ShopUnavailableReason},
    identity::{validate_identity, validate_text_value},
    model::{
        SHOP_MAX_CONTRIBUTORS, SHOP_MAX_DISCOUNT_PERCENT, SHOP_MAX_DISCOUNT_TIERS,
        SHOP_MAX_UNRESOLVED_INPUTS,
    },
    value::{ShopMoney, ShopNumber, ShopPrice, fixed_percent, fixed_value},
};

/// Comparison between a displayed price, its documented price, and its named contributors.
///
/// Every part is a [`ShopField`] rather than a substituted number: an amount the source reported
/// as a formula stays unavailable here instead of being folded into an invented total.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopPriceComparison {
    /// Currency the comparison is denominated in.
    pub currency: ShopField<String>,
    /// Fixed displayed amount, when the source reported one.
    pub displayed: ShopField<i64>,
    /// Fixed documented amount, when the source reported one.
    pub documented: ShopField<i64>,
    /// Sum of fixed contributor deltas, when every contributor reported one.
    pub contributor_total: ShopField<i64>,
    /// Whether the displayed amount equals the documented amount.
    pub matches_documented: ShopField<bool>,
    /// Whether the displayed amount equals the documented amount plus every contributor delta.
    pub consistent_with_contributors: ShopField<bool>,
}

impl ShopPriceComparison {
    /// Compares one price without evaluating or inventing an amount.
    #[must_use]
    pub fn of(price: &ShopPrice) -> Self {
        let displayed = fixed_amount(&price.displayed);
        let documented = fixed_amount(&price.documented);
        let contributor_total = contributor_total(price);
        let matches_documented = match price.matches_documented() {
            Some(matches) => ShopField::Available(matches),
            None => ShopField::Unavailable(ShopUnavailableReason::NotApplicable),
        };
        let consistent_with_contributors = match (
            displayed.value(),
            documented.value(),
            contributor_total.value(),
        ) {
            (Some(displayed), Some(documented), Some(total)) => {
                ShopField::Available(*displayed == documented.saturating_add(*total))
            }
            _ => ShopField::Unavailable(ShopUnavailableReason::NotApplicable),
        };
        Self {
            currency: price.currency.clone(),
            displayed,
            documented,
            contributor_total,
            matches_documented,
            consistent_with_contributors,
        }
    }
}

/// Returns the fixed amount of one money field without substituting a value.
fn fixed_amount(field: &ShopField<ShopMoney>) -> ShopField<i64> {
    match field {
        ShopField::Unavailable(reason) => ShopField::Unavailable(*reason),
        ShopField::Available(money) => match money.fixed_amount() {
            Some(value) => ShopField::Available(value),
            None => ShopField::Unavailable(ShopUnavailableReason::NotApplicable),
        },
    }
}

/// Sums fixed contributor deltas, or reports why no total applies.
fn contributor_total(price: &ShopPrice) -> ShopField<i64> {
    if price.contributors_status != ShopFieldStatus::Available || price.contributors.is_empty() {
        return ShopField::Unavailable(ShopUnavailableReason::NotApplicable);
    }
    let mut total = 0_i64;
    for contributor in &price.contributors {
        let Some(value) = fixed_value(&contributor.delta) else {
            return ShopField::Unavailable(ShopUnavailableReason::NotApplicable);
        };
        total = total.saturating_add(value);
    }
    ShopField::Available(total)
}

/// Rejects a price whose currency, amounts, or contributors contradict each other.
pub(super) fn validate_price(
    price: &ShopPrice,
    shop_id: &str,
    owner_id: &str,
) -> Result<(), ShopCatalogError> {
    let invalid = || ShopCatalogError::InvalidPrice {
        shop_id: shop_id.to_owned(),
        entry_id: owner_id.to_owned(),
    };
    if price.contributors.len() > SHOP_MAX_CONTRIBUTORS
        || price.unresolved_inputs.len() > SHOP_MAX_UNRESOLVED_INPUTS
    {
        return Err(ShopCatalogError::InvalidInput("contributors"));
    }
    if let ShopField::Available(currency) = &price.currency {
        validate_identity(currency, "currency").map_err(|_| invalid())?;
    }
    let carries_amount = price.displayed.value().is_some() || price.documented.value().is_some();
    if carries_amount && !price.currency.is_available() {
        return Err(invalid());
    }
    for money in [price.displayed.value(), price.documented.value()]
        .into_iter()
        .flatten()
    {
        validate_money(price, money, invalid)?;
    }
    for input in &price.unresolved_inputs {
        validate_identity(input, "unresolved_input").map_err(|_| invalid())?;
    }
    validate_contributors(price, shop_id, owner_id)
}

fn validate_money(
    price: &ShopPrice,
    money: &ShopMoney,
    invalid: impl Fn() -> ShopCatalogError,
) -> Result<(), ShopCatalogError> {
    validate_identity(&money.currency_id, "money_currency").map_err(|_| invalid())?;
    if !price.currency.is_available() || price.currency.value() != Some(&money.currency_id) {
        return Err(invalid());
    }
    if let Some(amount) = fixed_value(&money.amount)
        && amount < 0
    {
        return Err(invalid());
    }
    Ok(())
}

fn validate_contributors(
    price: &ShopPrice,
    shop_id: &str,
    owner_id: &str,
) -> Result<(), ShopCatalogError> {
    let mut seen = BTreeSet::new();
    for contributor in &price.contributors {
        validate_identity(&contributor.contributor_id, "contributor_id")
            .map_err(|_| invalid_price(shop_id, owner_id))?;
        validate_text_value(&contributor.label, "contributor_label")
            .map_err(|_| invalid_price(shop_id, owner_id))?;
        if let ShopNumber::Formula(formula) = &contributor.delta {
            validate_identity(&formula.formula_id, "contributor_formula")
                .map_err(|_| invalid_price(shop_id, owner_id))?;
        }
        if !seen.insert(contributor.contributor_id.as_str()) {
            return Err(ShopCatalogError::DuplicateContributor {
                shop_id: shop_id.to_owned(),
                owner_id: owner_id.to_owned(),
                contributor_id: contributor.contributor_id.clone(),
            });
        }
    }
    Ok(())
}

fn invalid_price(shop_id: &str, owner_id: &str) -> ShopCatalogError {
    ShopCatalogError::InvalidPrice {
        shop_id: shop_id.to_owned(),
        entry_id: owner_id.to_owned(),
    }
}

/// Rejects sale or stacked-discount state that contradicts its own reported percentages.
pub(super) fn validate_sale(
    sale: &ShopSaleState,
    shop_id: &str,
    entry_id: &str,
) -> Result<(), ShopCatalogError> {
    let invalid = || ShopCatalogError::InvalidDiscount {
        shop_id: shop_id.to_owned(),
        entry_id: entry_id.to_owned(),
    };
    match sale {
        ShopSaleState::NotOnSale | ShopSaleState::Unknown => Ok(()),
        ShopSaleState::OnSale {
            sale_id,
            percent,
            label,
        } => {
            validate_identity(sale_id, "sale_id").map_err(|_| invalid())?;
            validate_text_value(label, "sale_label").map_err(|_| invalid())?;
            validate_percent(percent).map_err(|_| invalid())?;
            Ok(())
        }
        ShopSaleState::Stacked {
            tiers,
            combined_percent,
        } => validate_stack(tiers, combined_percent, invalid),
    }
}

fn validate_stack(
    tiers: &[super::entry::ShopDiscountTier],
    combined_percent: &ShopNumber,
    invalid: impl Fn() -> ShopCatalogError,
) -> Result<(), ShopCatalogError> {
    if tiers.len() < 2 || tiers.len() > SHOP_MAX_DISCOUNT_TIERS {
        return Err(invalid());
    }
    let combined = validate_percent(combined_percent).map_err(|_| invalid())?;
    let mut seen = BTreeSet::new();
    for tier in tiers {
        validate_identity(&tier.tier_id, "tier_id").map_err(|_| invalid())?;
        let percent = validate_percent(&tier.percent).map_err(|_| invalid())?;
        if let (Some(combined), Some(percent)) = (combined, percent)
            && percent > combined
        {
            return Err(invalid());
        }
        if !seen.insert(tier.tier_id.as_str()) {
            return Err(invalid());
        }
    }
    Ok(())
}

/// Returns a reported percentage when it is a representable discount, else rejects it.
fn validate_percent(number: &ShopNumber) -> Result<Option<u32>, ShopCatalogError> {
    match number {
        ShopNumber::Fixed(_) => {
            let percent = fixed_percent(number).ok_or(ShopCatalogError::InvalidInput("percent"))?;
            if percent > SHOP_MAX_DISCOUNT_PERCENT {
                return Err(ShopCatalogError::InvalidInput("percent"));
            }
            Ok(Some(percent))
        }
        ShopNumber::Formula(formula) => {
            validate_identity(&formula.formula_id, "percent_formula")?;
            Ok(None)
        }
        ShopNumber::Unavailable(_) => Ok(None),
    }
}
