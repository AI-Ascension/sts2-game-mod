// SPDX-License-Identifier: MIT

use super::{
    ShopCatalogBinding, ShopEntryReference, ShopEvidence, ShopField, ShopItemKind,
    ShopPurchaseActionReference, ShopSemanticReference, ShopStockState, ShopText, ShopVisibility,
    value::ShopNumber,
};

/// Sale or discount state of one inventory entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ShopSaleState {
    /// The entry is offered at its undiscounted price.
    NotOnSale,
    /// The entry carries one explicit sale adjustment.
    OnSale {
        /// Source-defined sale identity.
        sale_id: String,
        /// Discount percentage the source reports.
        percent: ShopNumber,
        /// Localized sale label.
        label: ShopText,
    },
    /// The entry carries several discount tiers that apply together.
    Stacked {
        /// Tiered discounts in source order.
        tiers: Vec<ShopDiscountTier>,
        /// Combined percentage the source reports for the stack.
        combined_percent: ShopNumber,
    },
    /// Source could not classify the sale state.
    Unknown,
}

impl ShopSaleState {
    /// Returns whether any discount is active for this entry.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::OnSale { .. } | Self::Stacked { .. })
    }

    /// Returns every discount tier the source reported.
    #[must_use]
    pub fn tiers(&self) -> &[ShopDiscountTier] {
        match self {
            Self::Stacked { tiers, .. } => tiers,
            Self::NotOnSale | Self::OnSale { .. } | Self::Unknown => &[],
        }
    }
}

/// One discount tier inside a stacked sale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopDiscountTier {
    /// Source-defined tier identity.
    pub tier_id: String,
    /// Percentage this tier contributes.
    pub percent: ShopNumber,
    /// Reference to the rule or effect that grants the tier.
    pub source: ShopSemanticReference,
}

/// Coarse reason one inventory entry is restricted.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopRestrictionKind {
    /// The destination has no free slot.
    CapacityFull,
    /// A slot-count limit applies.
    SlotLimit,
    /// A stated eligibility requirement applies.
    Eligibility,
    /// The item may only be owned once.
    Unique,
    /// The entry may be purchased only a limited number of times.
    PurchaseLimit,
    /// Owner-defined restriction kind.
    Custom(String),
    /// Source could not classify the restriction.
    Unknown,
}

/// One capacity or eligibility restriction attached to an entry or service.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopRestriction {
    /// Source-defined restriction identity.
    pub restriction_id: String,
    /// Coarse restriction kind.
    pub kind: ShopRestrictionKind,
    /// Localized restriction label.
    pub label: ShopText,
    /// Numeric capacity the restriction names, when it names one.
    pub capacity: ShopField<u32>,
    /// Whether the restriction is currently satisfied.
    pub satisfied: ShopField<bool>,
    /// Requirements that determine eligibility.
    pub requirements: Vec<ShopRequirement>,
    /// Availability of the requirement collection after scope withholding.
    pub requirements_status: super::ShopFieldStatus,
    /// Typed references for the restriction.
    pub references: Vec<ShopSemanticReference>,
}

/// State of one eligibility requirement.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopRequirementState {
    /// The observed run satisfies the requirement.
    Met,
    /// The observed run does not satisfy the requirement.
    Unmet,
    /// Source could not classify the requirement state.
    Unknown,
}

/// One eligibility requirement with its own explicit state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopRequirement {
    /// Source-defined requirement identity.
    pub requirement_id: String,
    /// Localized requirement description.
    pub label: ShopText,
    /// Observed requirement state.
    pub state: ShopRequirementState,
    /// Typed references for the requirement.
    pub references: Vec<ShopSemanticReference>,
}

/// Complete source-owned inventory entry before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopEntryInput {
    /// Stable inventory entry identity.
    pub entry_id: String,
    /// Localized entry label.
    pub label: ShopText,
    /// Kind of item the entry offers.
    pub item_kind: ShopItemKind,
    /// Definition the entry resolves to.
    pub definition: ShopSemanticReference,
    /// Displayed price, contributors, and any explicit block reason.
    pub price: super::ShopPrice,
    /// Observed stock state.
    pub stock: ShopStockState,
    /// Sale or stacked-discount state.
    pub sale: ShopSaleState,
    /// Purchase action reference, which this slice can only report as unavailable.
    pub purchase_action: ShopField<ShopPurchaseActionReference>,
    /// Capacity and eligibility restrictions.
    pub restrictions: Vec<ShopRestriction>,
    /// Typed references for the entry.
    pub references: Vec<ShopSemanticReference>,
    /// Visibility of the entry.
    pub visibility: ShopVisibility,
    /// Evidence label for the entry.
    pub evidence: ShopEvidence,
}

/// Immutable inventory entry bound to one manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopEntry {
    /// Exact static entry reference.
    pub reference: ShopEntryReference,
    /// Localized entry label.
    pub label: ShopText,
    /// Kind of item the entry offers.
    pub item_kind: ShopItemKind,
    /// Definition the entry resolves to.
    pub definition: ShopSemanticReference,
    /// Displayed price, contributors, and any explicit block reason.
    pub price: super::ShopPrice,
    /// Observed stock state.
    pub stock: ShopStockState,
    /// Sale or stacked-discount state.
    pub sale: ShopSaleState,
    /// Purchase action reference, which this slice reports as unavailable.
    pub purchase_action: ShopField<ShopPurchaseActionReference>,
    /// Capacity and eligibility restrictions.
    pub restrictions: Vec<ShopRestriction>,
    /// Typed references for the entry.
    pub references: Vec<ShopSemanticReference>,
    /// Visibility of the entry.
    pub visibility: ShopVisibility,
    /// Evidence label for the entry.
    pub evidence: ShopEvidence,
}

impl ShopEntry {
    /// Binds an input entry to one catalog and shop identity.
    pub(super) fn from_input(
        binding: &ShopCatalogBinding,
        shop_id: &str,
        input: ShopEntryInput,
    ) -> Self {
        Self {
            reference: ShopEntryReference {
                catalog: binding.clone(),
                shop_id: shop_id.to_owned(),
                entry_id: input.entry_id,
            },
            label: input.label,
            item_kind: input.item_kind,
            definition: input.definition,
            price: input.price,
            stock: input.stock,
            sale: input.sale,
            purchase_action: input.purchase_action,
            restrictions: input.restrictions,
            references: input.references,
            visibility: input.visibility,
            evidence: input.evidence,
        }
    }
}
