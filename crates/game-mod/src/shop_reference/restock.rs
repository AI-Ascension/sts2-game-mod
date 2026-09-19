// SPDX-License-Identifier: MIT

use super::{
    ShopEvidence, ShopField, ShopSemanticReference, ShopText, ShopVisibility,
    model::{SHOP_MAX_ENTRIES, SHOP_MAX_REFERENCES},
};

/// What causes a shop to restock.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopRestockTriggerKind {
    /// Entering the shop again restocks it.
    ShopVisit,
    /// A dedicated restock event restocks it.
    RestockEvent,
    /// Purchasing past a threshold restocks it.
    PurchaseThreshold,
    /// Using a service restocks it.
    ServiceUse,
    /// Resolving another event restocks it.
    EventResolution,
    /// Owner-defined trigger kind.
    Custom(String),
    /// A trigger is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the trigger.
    Unknown,
}

/// One named restock trigger.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopRestockTrigger {
    /// Source-defined trigger identity.
    pub trigger_id: String,
    /// Coarse trigger kind.
    pub kind: ShopRestockTriggerKind,
    /// Localized trigger label.
    pub label: ShopText,
}

/// One restock rule and the inventory generation it produces.
///
/// The rule carries the generation it produces so a stock reference can be fenced to the exact
/// generation that produced it; a rule that would produce the current or an earlier generation is
/// rejected rather than silently reusing observed stock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopRestockRule {
    /// Source-defined rule identity.
    pub rule_id: String,
    /// Localized rule label.
    pub label: ShopText,
    /// Trigger that fires this rule.
    pub trigger: ShopRestockTrigger,
    /// Generation this rule produces, strictly greater than the shop's observed generation.
    pub generation: u64,
    /// Whether the rule replaces sold-out entries.
    pub replaces_sold_out: ShopField<bool>,
    /// Entry identities this rule restocks, bounded by the shop entry bound.
    pub restocks_entries: Vec<ShopSemanticReference>,
    /// Typed references for the rule.
    pub references: Vec<ShopSemanticReference>,
    /// Visibility of the rule.
    pub visibility: ShopVisibility,
    /// Evidence label for the rule.
    pub evidence: ShopEvidence,
}

impl ShopRestockRule {
    /// Returns the bounded number of entries this rule claims to restock.
    #[must_use]
    pub fn restocked_entry_count(&self) -> usize {
        self.restocks_entries.len()
    }
}

/// Returns whether a rule stays within the per-shop restock bounds.
#[must_use]
pub(super) fn within_restock_bounds(rule: &ShopRestockRule) -> bool {
    rule.restocks_entries.len() <= SHOP_MAX_ENTRIES && rule.references.len() <= SHOP_MAX_REFERENCES
}
