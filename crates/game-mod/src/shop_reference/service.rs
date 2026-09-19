// SPDX-License-Identifier: MIT

use super::{
    ShopCatalogBinding, ShopEvidence, ShopField, ShopFieldStatus, ShopPrice,
    ShopPurchaseActionReference, ShopRequirement, ShopSemanticReference, ShopServiceKind,
    ShopServiceReference, ShopStockState, ShopText, ShopVisibility,
};

/// Domain a service operation selects from.
///
/// The domain is stated rather than assumed, because not every game or version offers the same
/// services over the same collection.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopServiceSelectionDomain {
    /// A card already in the deck.
    DeckCard,
    /// A card offered by the shop inventory.
    ShopCard,
    /// A relic.
    Relic,
    /// A potion.
    Potion,
    /// Any inventory entry.
    Inventory,
    /// Owner-defined selection domain.
    Custom(String),
    /// A domain is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the domain.
    Unknown,
}

/// What a service would change if it were used.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopProspectiveChange {
    /// Remove a selected card from the deck.
    RemoveFromDeck,
    /// Upgrade a selected card in the deck.
    UpgradeInDeck,
    /// Transform a selected card in the deck.
    TransformInDeck,
    /// Grant a selected item to the run.
    GrantItem,
    /// Owner-defined prospective change.
    Custom(String),
    /// A change is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the change.
    Unknown,
}

/// Period a service limit is counted over.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopServiceLimitUnit {
    /// The limit applies to one run.
    PerRun,
    /// The limit applies to one shop.
    PerShop,
    /// The limit applies to one visit.
    PerVisit,
    /// The limit applies to one restock generation.
    PerRestock,
    /// Owner-defined limit unit.
    Custom(String),
    /// Source could not classify the limit unit.
    Unknown,
}

/// One declared service limit with the remaining allowance kept separate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopServiceLimit {
    /// Source-defined limit identity.
    pub limit_id: String,
    /// Localized limit label.
    pub label: ShopText,
    /// Period the limit is counted over.
    pub unit: ShopServiceLimitUnit,
    /// Declared limit value.
    pub value: ShopField<u32>,
    /// Remaining allowance the source reports.
    pub remaining: ShopField<u32>,
    /// Typed references for the limit.
    pub references: Vec<ShopSemanticReference>,
}

/// Complete source-owned service definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopServiceInput {
    /// Exact-build service identity.
    pub service_id: String,
    /// Localized service label.
    pub label: ShopText,
    /// Kind of service.
    pub kind: ShopServiceKind,
    /// Cost, contributors, and any explicit block reason.
    pub cost: ShopPrice,
    /// Domain the service selects from.
    pub selection_domain: ShopServiceSelectionDomain,
    /// Candidates the service could select, when the source lists them.
    pub selection_candidates: Vec<ShopSemanticReference>,
    /// Availability of the candidate collection after scope withholding.
    pub selection_candidates_status: ShopFieldStatus,
    /// Declared limits.
    pub limits: Vec<ShopServiceLimit>,
    /// Eligibility requirements.
    pub eligibility: Vec<ShopRequirement>,
    /// Availability of the eligibility collection after scope withholding.
    pub eligibility_status: ShopFieldStatus,
    /// What the service would change.
    pub prospective_change: ShopProspectiveChange,
    /// Observed service stock state.
    pub stock: ShopStockState,
    /// Purchase action reference, which this slice can only report as unavailable.
    pub purchase_action: ShopField<ShopPurchaseActionReference>,
    /// Typed references for the service.
    pub references: Vec<ShopSemanticReference>,
    /// Visibility of the service.
    pub visibility: ShopVisibility,
    /// Evidence label for the service.
    pub evidence: ShopEvidence,
}

/// Immutable service bound to one manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopService {
    /// Exact static service reference.
    pub reference: ShopServiceReference,
    /// Localized service label.
    pub label: ShopText,
    /// Kind of service.
    pub kind: ShopServiceKind,
    /// Cost, contributors, and any explicit block reason.
    pub cost: ShopPrice,
    /// Domain the service selects from.
    pub selection_domain: ShopServiceSelectionDomain,
    /// Candidates the service could select.
    pub selection_candidates: Vec<ShopSemanticReference>,
    /// Availability of the candidate collection after scope withholding.
    pub selection_candidates_status: ShopFieldStatus,
    /// Declared limits.
    pub limits: Vec<ShopServiceLimit>,
    /// Eligibility requirements.
    pub eligibility: Vec<ShopRequirement>,
    /// Availability of the eligibility collection after scope withholding.
    pub eligibility_status: ShopFieldStatus,
    /// What the service would change.
    pub prospective_change: ShopProspectiveChange,
    /// Observed service stock state.
    pub stock: ShopStockState,
    /// Purchase action reference, which this slice reports as unavailable.
    pub purchase_action: ShopField<ShopPurchaseActionReference>,
    /// Typed references for the service.
    pub references: Vec<ShopSemanticReference>,
    /// Visibility of the service.
    pub visibility: ShopVisibility,
    /// Evidence label for the service.
    pub evidence: ShopEvidence,
}

impl ShopService {
    /// Binds an input service to one catalog and shop identity.
    pub(super) fn from_input(
        binding: &ShopCatalogBinding,
        shop_id: &str,
        input: ShopServiceInput,
    ) -> Self {
        Self {
            reference: ShopServiceReference {
                catalog: binding.clone(),
                shop_id: shop_id.to_owned(),
                service_id: input.service_id,
            },
            label: input.label,
            kind: input.kind,
            cost: input.cost,
            selection_domain: input.selection_domain,
            selection_candidates: input.selection_candidates,
            selection_candidates_status: input.selection_candidates_status,
            limits: input.limits,
            eligibility: input.eligibility,
            eligibility_status: input.eligibility_status,
            prospective_change: input.prospective_change,
            stock: input.stock,
            purchase_action: input.purchase_action,
            references: input.references,
            visibility: input.visibility,
            evidence: input.evidence,
        }
    }
}
