// SPDX-License-Identifier: MIT

//! Source-only shop inventory, service, pricing, and restock reference catalog.
//!
//! This module copies what a shop offers and why it costs what it costs into an immutable catalog
//! fenced by the existing content-manifest cursor and locale: typed inventory entries with
//! definition references, displayed price and currency, stock state, sale and stacked-discount
//! state, purchase action and capacity/eligibility restrictions; supported services with their
//! exact-build service identity, cost, selection domain, limits, eligibility and prospective
//! change; static pricing contributors and reference formulas; and generation-bound restock rules.
//!
//! A reported kind never contradicts the definition family it resolves to: an entry whose
//! reference resolves to a card, relic, potion or service definition while claiming `Unknown` is
//! rejected, so no supported item can stay hard-coded unknown. A documented formula stays distinct
//! from the currently observed price, rounding is named rather than inferred, and an unresolved
//! input is retained instead of being folded into a total. Stock references are fenced by the
//! restock generation that produced them, so a reference from an earlier restock is refused rather
//! than answered with current stock.
//!
//! The slice is read-only by construction: it exposes no purchase, sale, restock, or gold-spending
//! entry point, retains no currency balance it could change, and cannot admit or continue a run. No
//! native extraction, live host read, or transport route is claimed here.

mod catalog;
mod cursor;
mod definition;
mod encoding;
mod entry;
mod error;
mod field;
mod identity;
mod model;
mod page;
mod pricing;
mod projection;
mod reader;
mod restock;
mod rules;
mod service;
mod source;
mod validation;
mod value;

pub use catalog::{ShopCatalogProducer, ShopCatalogSnapshot, ShopCatalogSource};
pub use definition::{ShopCatalog, ShopDefinition, ShopDefinitionInput};
pub use entry::{
    ShopDiscountTier, ShopEntry, ShopEntryInput, ShopRequirement, ShopRequirementState,
    ShopRestriction, ShopRestrictionKind, ShopSaleState,
};
pub use error::{ShopCatalogError, ShopSourceError};
pub use field::{ShopField, ShopFieldStatus, ShopText, ShopUnavailableReason};
pub use model::*;
pub use page::{
    ShopContinuation, ShopDefinitionPage, ShopDefinitionSummary, ShopEntryContinuation,
    ShopEntryListQuery, ShopEntryPage, ShopEntrySummary, ShopListQuery, ShopServiceContinuation,
    ShopServiceListQuery, ShopServicePage, ShopServiceSummary,
};
pub use pricing::ShopPriceComparison;
pub use reader::ShopCatalogReader;
pub use restock::{ShopRestockRule, ShopRestockTrigger, ShopRestockTriggerKind};
pub use service::{
    ShopProspectiveChange, ShopService, ShopServiceInput, ShopServiceLimit, ShopServiceLimitUnit,
    ShopServiceSelectionDomain,
};
pub use source::{FailingShopSource, FixtureShopFailure, FixtureShopSource};
pub use value::{
    ShopBlockedReason, ShopFormula, ShopMoney, ShopNumber, ShopPrice, ShopPricingContributor,
    ShopPricingContributorKind, ShopRoundingMode,
};
