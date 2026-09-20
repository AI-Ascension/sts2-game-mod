// SPDX-License-Identifier: MIT

//! Source-only completed-run results, score reconciliation and prior run summaries.
//!
//! This module copies completed-run outcomes, characters, configurations, reached act/floor,
//! durations, ending decks and inventories, displayed score components and supported statistics,
//! plus scoped prior-summary records, into an immutable catalog fenced by the existing content
//! manifest and locale.  An unknown value is never converted into zero, an empty collection or an
//! invented description, because every field carries its own availability.  The game's own score
//! stays distinct from a harness evaluator or synthetic metric, a terminal presentation is not a
//! persisted result, a partial or unsettled result stays explicitly unavailable, a summary whose
//! detail the catalog does not carry stays unavailable rather than reconstructed, a history read
//! selects no profile and touches no save, and a hidden or owner-only record is never returned
//! outside a scope that may observe it.  No native run read, transport route, or host compatibility
//! is claimed here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod field;
mod identity;
mod kind;
mod model;
mod validation;
mod value;

pub use catalog::{RunResultCatalogProducer, RunResultCatalogSnapshot, RunResultCatalogSource};
pub use catalog_reader::*;
pub use definition::*;
pub use error::{RunResultError, RunResultReadAuthority, RunResultSourceError};
pub use field::{RunResultField, RunResultFieldStatus};
pub use identity::is_opaque_identity;
pub use kind::{
    ResultFinalization, ResultOrigin, RunOutcome, RunResultPersistence, RunResultVisibility,
    RunResultVisibilityScope, ScoreAuthority, ScoreMode,
};
pub use model::*;
pub use value::*;
