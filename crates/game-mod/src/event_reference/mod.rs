// SPDX-License-Identifier: MIT

//! Source-only event definitions, narrative pages, and choice/branch references.
//!
//! This module copies event titles, localized narrative pages, event/option eligibility,
//! structured costs, and possible effects/outcomes into an immutable catalog fenced by the
//! existing content manifest and locale.  Each option has a stable definition reference and a
//! branch graph of possible outcomes with evidence-qualified probabilities when the source defines
//! one; a probability is never invented and an unknown one stays explicitly unavailable.  Static
//! definition identities stay distinct from live run/instance identities and from transient
//! button/action identities, and a hidden page is never revealed by a more visible branch.  No live
//! run read, RNG evaluation, transport route, native extractor, or host compatibility is claimed
//! here.

mod catalog;
mod catalog_reader;
mod definition;
mod error;
mod model;
mod validation;

pub use catalog::{EventCatalogProducer, EventCatalogSnapshot, EventCatalogSource};
pub use catalog_reader::*;
pub use definition::*;
pub use error::{EventCatalogError, EventSourceError};
pub use model::*;
