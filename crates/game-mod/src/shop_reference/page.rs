// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    entry::ShopSaleState,
    field::{ShopFieldStatus, ShopText},
    model::{
        ShopCatalogBinding, ShopDefinitionReference, ShopEntryReference, ShopEvidence,
        ShopItemKind, ShopSemanticReference, ShopServiceKind, ShopServiceReference, ShopStockState,
        ShopVisibility, ShopVisibilityScope,
    },
    service::{ShopProspectiveChange, ShopServiceSelectionDomain},
    value::ShopPrice,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use shop-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ShopContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use inventory-entry-list continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopEntryContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ShopEntryContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use service-list continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ShopServiceContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ShopServiceContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded shop-definition-list request.
#[derive(Debug, Eq, PartialEq)]
pub struct ShopListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: ShopVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ShopContinuation>,
}

/// Typed summary returned by one bounded shop page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopDefinitionSummary {
    /// Exact static definition reference.
    pub reference: ShopDefinitionReference,
    /// Localized shop label or explicit unavailable state.
    pub label: ShopText,
    /// Visibility of the definition.
    pub visibility: ShopVisibility,
    /// Evidence label for the definition.
    pub evidence: ShopEvidence,
    /// Observed restock generation.
    pub generation: u64,
    /// Number of visible inventory entries.
    pub entry_count: usize,
    /// Availability of inventory entries after scope withholding.
    pub entries_status: ShopFieldStatus,
    /// Number of visible services.
    pub service_count: usize,
    /// Availability of services after scope withholding.
    pub services_status: ShopFieldStatus,
    /// Number of visible restock rules.
    pub restock_count: usize,
    /// Availability of restock rules after scope withholding.
    pub restock_status: ShopFieldStatus,
}

/// Complete or partial shop-definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct ShopDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: ShopCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<ShopDefinitionSummary>,
    /// Number of visible shop definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ShopContinuation>,
}

/// Bounded entry-list request scoped to one exact shop definition.
#[derive(Debug, Eq, PartialEq)]
pub struct ShopEntryListQuery {
    /// Exact shop definition whose entries are listed.
    pub shop: ShopDefinitionReference,
    /// Visibility scope.
    pub scope: ShopVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ShopEntryContinuation>,
}

/// Typed summary returned by one bounded entry page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopEntrySummary {
    /// Exact static entry reference.
    pub reference: ShopEntryReference,
    /// Localized entry label.
    pub label: ShopText,
    /// Kind of item the entry offers.
    pub item_kind: ShopItemKind,
    /// Definition the entry resolves to.
    pub definition: ShopSemanticReference,
    /// Displayed price, contributors, and any explicit block reason.
    pub price: ShopPrice,
    /// Observed stock state.
    pub stock: ShopStockState,
    /// Sale or stacked-discount state.
    pub sale: ShopSaleState,
    /// Visibility of the entry.
    pub visibility: ShopVisibility,
}

/// Complete or partial inventory-entry page.
#[derive(Debug, Eq, PartialEq)]
pub struct ShopEntryPage {
    /// Catalog witness for every entry.
    pub binding: ShopCatalogBinding,
    /// Deterministically ordered entry summaries.
    pub entries: Vec<ShopEntrySummary>,
    /// Number of visible entries.
    pub total: usize,
    /// Availability of entries after scope withholding, independent of pagination exhaustion.
    pub entries_status: ShopFieldStatus,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ShopEntryContinuation>,
}

/// Bounded service-list request scoped to one exact shop definition.
#[derive(Debug, Eq, PartialEq)]
pub struct ShopServiceListQuery {
    /// Exact shop definition whose services are listed.
    pub shop: ShopDefinitionReference,
    /// Visibility scope.
    pub scope: ShopVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ShopServiceContinuation>,
}

/// Typed summary returned by one bounded service page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopServiceSummary {
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
    /// What the service would change.
    pub prospective_change: ShopProspectiveChange,
    /// Observed service stock state.
    pub stock: ShopStockState,
    /// Visibility of the service.
    pub visibility: ShopVisibility,
}

/// Complete or partial service page.
#[derive(Debug, Eq, PartialEq)]
pub struct ShopServicePage {
    /// Catalog witness for every entry.
    pub binding: ShopCatalogBinding,
    /// Deterministically ordered service summaries.
    pub entries: Vec<ShopServiceSummary>,
    /// Number of visible services.
    pub total: usize,
    /// Availability of services after scope withholding, independent of pagination exhaustion.
    pub services_status: ShopFieldStatus,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ShopServiceContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct ShopCursorState {
    pub(super) binding: ShopCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: ShopVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ShopEntryCursorState {
    pub(super) binding: ShopCatalogBinding,
    pub(super) shop_id: String,
    pub(super) scope: ShopVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct ShopServiceCursorState {
    pub(super) binding: ShopCatalogBinding,
    pub(super) shop_id: String,
    pub(super) scope: ShopVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
