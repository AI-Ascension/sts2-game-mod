// SPDX-License-Identifier: MIT

//! Source-only card, player, and item selection reference catalog.
//!
//! This module copies what a supported selector asks the player to choose and which candidates it
//! presents into an immutable catalog fenced by the existing content-manifest cursor and locale:
//! typed selection definitions with their parent operation, localized prompt, exact-build family,
//! required/minimum/maximum picks, ordering and duplicate rules, confirmation and cancellation
//! semantics, and an explicit selector generation; and typed candidates with the definition they
//! resolve to, resolved eligibility and refusal reason, documented prospective effects, and explicit
//! evidence/visibility labels.
//!
//! Every candidate the host reports is either described by a typed record or named by a coverage
//! record, so a newly audited entry is refused rather than silently omitted. A selector never
//! contradicts the definition family its candidates resolve to, so no supported prompt can stay
//! hard-coded unknown. A required selector always requires at least one pick even when the source
//! states no minimum, and a documented prospective effect that would change nothing is refused
//! rather than published as a change. A selector reference is fenced by the selector generation that
//! produced it, so a reference from an earlier prompt is refused rather than answered with the
//! current candidate domain, and a multi-step selector names the next domain by identity and
//! generation instead of leaving the caller to guess it.
//!
//! The slice is read-only by construction: it exposes no click, confirm, cancel, back, or advance
//! entry point, performs no selection, retains no transient host action identity, and cannot admit
//! or continue a run. Reconciling a caller's picks reports the remaining count, the candidates still
//! selectable, and whether the prompt may be confirmed, so an early confirmation, a duplicate choice,
//! and a stale selector are never represented as legal. No native extraction, live host read, or
//! transport route is claimed here.

mod candidate;
mod catalog;
mod cursor;
mod definition;
mod encoding;
mod error;
mod field;
mod identity;
mod kind;
mod model;
mod page;
mod progress;
mod projection;
mod reader;
mod reader_progress;
mod source;
mod validation;

pub use candidate::{
    SelectionBlockReason, SelectionCandidate, SelectionCandidateInput, SelectionEligibility,
    SelectionEligibilityState, SelectionProspectiveEffect, SelectionProspectiveEffectKind,
};
pub use catalog::{SelectionCatalogProducer, SelectionCatalogSnapshot, SelectionCatalogSource};
pub use definition::{
    SelectionCatalog, SelectionCoverageRecord, SelectionCoverageState, SelectionDefinition,
    SelectionDefinitionInput, SelectionPickRule,
};
pub use error::{SelectionError, SelectionSourceError};
pub use field::{SelectionField, SelectionFieldStatus, SelectionText, SelectionUnavailableReason};
pub use model::*;
pub use page::{
    SelectionCandidateContinuation, SelectionCandidateListQuery, SelectionCandidatePage,
    SelectionCandidateSummary, SelectionContinuation, SelectionDefinitionPage,
    SelectionDefinitionSummary, SelectionListQuery,
};
pub use progress::{SelectionConfirmationState, SelectionProgress, SelectionProgressInput};
pub use reader::SelectionReader;
pub use source::{FailingSelectionSource, FixtureSelectionFailure, FixtureSelectionSource};
