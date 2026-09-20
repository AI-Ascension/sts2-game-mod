// SPDX-License-Identifier: MIT

//! Source-only legal-action availability and target-specific consequence reference.
//!
//! This module copies what a supported legal-action frame offers into an immutable catalog fenced
//! by the existing content-manifest cursor and locale: typed legal-action definitions with their
//! parent operation, exact-build family, localized label, explicit legal-action generation,
//! resolved availability, bounded cost contributors with the amounts the source observed, target
//! restrictions, the targets the host presents, and named coverage for anything the host reports
//! without a typed record; and typed previews that state a target's consequences with their own
//! classification, stated assumptions, and named omissions.
//!
//! Availability is explained rather than asserted. A refused action carries the host's own reason
//! code and reason text, and every cost contributor that currently blocks it and every restriction
//! that currently refuses it is reported with the amounts behind it, so an unmet resource, an
//! invalid or dead target, a destination with no free capacity, a disabled option, and a selection
//! the action still requires each name the record that produces them instead of publishing a bare
//! disabled label. A contributor whose required or available amount was never observed refuses
//! rather than claiming an affordability, and a cost that would change nothing is rejected rather
//! than published as a consequence.
//!
//! A preview states only what the source can document. A consequence is bound to one exact target,
//! a random or unsupported chain must name the interaction it omits, an omitted chain under an
//! exact class is refused, and a preview the caller approximated by applying and undoing a real
//! action is refused by name. Because the retained snapshot is immutable and every read returns a
//! clone of it, repeating a preview is byte-identical by construction: a withheld future outcome is
//! never reconstructed by asking again, and a preview carries [`ActionDispatchAuthority::NotGranted`]
//! with the fresh catalog, epoch, and target validation a caller must still perform.
//!
//! The slice is read-only by construction: it exposes no play, use, buy, end, confirm, cancel, or
//! advance entry point, performs no action, consumes no RNG, retains no transient host action in
//! static data, and cannot admit or continue a run. A legal-action frame reference is fenced by the
//! generation that produced it, so a reference from an earlier frame is refused rather than answered
//! with the current frame's consequences, and a static query may not carry a live fence while a live
//! query may not omit one. No native extraction, live host read, or transport route is claimed here.

mod catalog;
mod cost;
mod cursor;
mod definition;
mod eligibility;
mod encoding;
mod error;
mod explain;
mod field;
mod identity;
mod kind;
mod model;
mod page;
mod preview;
mod preview_query;
mod projection;
mod reader;
mod source;
mod target;
mod validation;

pub use catalog::{ActionCatalogProducer, ActionCatalogSnapshot, ActionCatalogSource};
pub use cost::{ActionCostContributor, ActionTargetRestriction};
pub use definition::{
    ActionCatalog, ActionCoverageRecord, ActionCoverageState, ActionDefinition,
    ActionDefinitionInput,
};
pub use eligibility::ActionEligibility;
pub use error::{ActionError, ActionSourceError};
pub use explain::{ActionAvailabilityExplanation, ActionAvailabilityQuery};
pub use field::{ActionField, ActionFieldStatus, ActionText, ActionUnavailableReason};
pub use model::*;
pub use page::{
    ActionContinuation, ActionDefinitionPage, ActionDefinitionSummary, ActionListQuery,
    ActionTargetContinuation, ActionTargetListQuery, ActionTargetPage, ActionTargetSummary,
};
pub use preview::{
    ActionPreview, ActionPreviewAssumption, ActionPreviewChange, ActionPreviewInput,
    ActionPreviewMovement, ActionPreviewOmission, ActionPreviewSelection, ActionPreviewStatus,
};
pub use preview_query::{ActionPreviewQuery, ActionPreviewResult};
pub use reader::ActionReader;
pub use source::{FailingActionSource, FixtureActionFailure, FixtureActionSource};
pub use target::{ActionTarget, ActionTargetInput, ActionTargetKind};
