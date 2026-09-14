// SPDX-License-Identifier: MIT

//! Source-only reward offer definitions, selectable contents, and generation rules.
//!
//! This module copies reward offer titles, reward categories, typed quantities with preserved
//! currency units, offered item definition/instance references, selection groups with choose/skip
//! constraints and legal action references, static reward-generation pool/rarity/eligibility/
//! modifier rules, and explicit offer-state support into an immutable catalog fenced by the
//! existing content manifest and locale.  Static definition identities stay distinct from live
//! run/room/snapshot offer identities and from transient reward-action identities, a generator rule
//! never invents a probability and an unknown one stays explicitly unavailable, static rules remain
//! distinct from a future unrevealed roll, and a hidden item or rule is never revealed by a more
//! visible record.  No live run read, RNG evaluation, transport route, native extractor, or host
//! compatibility is claimed here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{RewardCatalogProducer, RewardCatalogSnapshot, RewardCatalogSource};
pub use catalog_reader::*;
pub use definition::*;
pub use error::{RewardCatalogError, RewardSourceError};
pub use model::*;
