// SPDX-License-Identifier: MIT

//! Source-only semantic gameplay events: a bounded, owned vocabulary of what happened, with the
//! source/target of each event and its causal parent where one is known.
//!
//! This is the game-mod companion half of the harness-owned run-history feature: the host states the
//! events it authoritatively observed, and nothing more. The module's contract is refusal rather than
//! reconstruction, because a history that guesses is worse than a history that admits a gap:
//!
//! - Every event carries its own coverage, so an interval the boundary could not observe is
//!   disclosed as `Dropped` or `Unsupported` instead of being closed by an invented event, a zeroed
//!   quantity, or a renumbered sequence.
//! - A causal parent is either explicitly stated by the host or explicitly absent. It is never
//!   inferred from a difference between two snapshots, and a parent that does not exist in the same
//!   sequence, or that does not precede its child, is refused rather than accepted.
//! - Sequence order is monotonic within one run, branch, episode and epoch, and a gap in it must be
//!   declared as coverage; it is never silently renumbered away.
//! - Definition identities, live instance identities and action identities stay distinct, so an event
//!   cannot be read as naming the wrong namespace.
//! - Identities are opaque: a filesystem path, a control byte or a traversal segment is refused.
//!
//! Read-only by construction: the single production seam reads a bounded owned snapshot, and history
//! authority is not granted here. Emitting native events, persisting or indexing them, querying them,
//! paginating them and traversing causality are the harness owner's work.
//!
//! No native event capture, transport route, or host compatibility is claimed here.

mod catalog;
mod catalog_reader;
mod causal;
mod coverage;
mod definition;
mod error;
mod identity;
mod kind;
mod model;
mod namespace;
mod origin;
mod sequence;
mod subject;
mod validation;

pub use catalog::{
    SemanticEventCatalogProducer, SemanticEventSnapshot, SemanticEventSource,
    SemanticHistoryCatalog,
};
pub use catalog_reader::{
    SemanticEventContinuation, SemanticEventListQuery, SemanticEventPage, SemanticEventReader,
    SemanticEventSummary, SemanticHistoryView,
};
pub use causal::{SemanticCausalParent, SemanticCausalProvenance};
pub use coverage::{
    SemanticCaptureWindow, SemanticCoverageInterval, SemanticCoverageStatus, SemanticEventCoverage,
};
pub use definition::{SemanticEventBatch, SemanticEventInput, SemanticEventRecord};
pub use error::{
    SemanticEventError, SemanticEventSourceError, SemanticHistoryAuthority, SemanticHistoryScope,
};
pub use identity::is_opaque_semantic_identity;
pub use kind::SemanticEventKind;
pub use model::*;
pub use namespace::SemanticIdentityNamespace;
pub use origin::SemanticEventOrigin;
pub use sequence::SemanticEventSequence;
pub use subject::{SemanticEventSubject, SemanticSubjectRole};
