// SPDX-License-Identifier: MIT

//! Source-only rest-site and rest-option reference catalog.
//!
//! This module copies what one rest site offers and what each option would do into an immutable
//! catalog fenced by the existing content-manifest cursor and locale: typed site definitions with
//! localized name and description; and typed options with their reported kind, the definition they
//! resolve to, resolved availability and refusal reason, requirements, costs, limits, effects,
//! selection domain with its candidates, documented prospective comparison, and explicit
//! evidence/visibility labels.
//!
//! Every option the host reports is either described by a typed record or named by a coverage
//! record, so a newly audited button is refused rather than silently omitted. A reported kind never
//! contradicts the definition family it resolves to, so no supported option can stay hard-coded
//! unknown. A documented healing formula keeps its base fraction, flat bonus, and named modifiers
//! separate instead of publishing a folded total, and rounding is named rather than inferred. A
//! comparison's completeness is derived from its own values, so a partial result is never labelled
//! complete. Option availability is fenced by the option-set generation that produced it, so a
//! reference from an earlier menu is refused rather than answered with current options.
//!
//! The slice is read-only by construction: it exposes no rest, heal, smith, upgrade, transform, or
//! mend entry point, performs no selection, retains no transient host action identity, and cannot
//! admit or continue a run. No native extraction, live host read, or transport route is claimed
//! here.

mod catalog;
mod cursor;
mod definition;
mod effect;
mod encoding;
mod error;
mod field;
mod identity;
mod model;
mod option;
mod page;
mod projection;
mod reader;
mod requirement;
mod source;
mod validation;

pub use catalog::{RestSiteCatalogProducer, RestSiteCatalogSnapshot, RestSiteCatalogSource};
pub use definition::{RestSiteCatalog, RestSiteDefinition, RestSiteDefinitionInput, RestSiteKind};
pub use effect::{
    RestComparisonBasis, RestEffect, RestEffectKind, RestHealAmount, RestHealModifier,
    RestHealModifierKind, RestProspectiveChange, RestProspectiveChangeKind,
    RestProspectiveComparison, RestRoundingMode, RestUpgradePreview,
};
pub use error::{RestSiteError, RestSourceError};
pub use field::{RestField, RestFieldStatus, RestText, RestUnavailableReason};
pub use model::*;
pub use option::{
    RestBlockReason, RestCoverageRecord, RestCoverageState, RestOption, RestOptionAvailability,
    RestOptionInput, RestOptionState,
};
pub use page::{
    RestOptionContinuation, RestOptionListQuery, RestOptionPage, RestOptionSummary,
    RestSiteContinuation, RestSiteDefinitionPage, RestSiteDefinitionSummary, RestSiteListQuery,
};
pub use reader::RestSiteReader;
pub use requirement::{
    RestCandidate, RestCost, RestCostUnit, RestLimit, RestLimitUnit, RestRequirement,
    RestRequirementState, RestSelectionDomain, RestSelectionRequirement,
};
pub use source::{FailingRestSiteSource, FixtureRestFailure, FixtureRestSiteSource};
