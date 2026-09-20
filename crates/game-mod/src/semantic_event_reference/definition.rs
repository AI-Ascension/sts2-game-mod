// SPDX-License-Identifier: MIT

//! Owner-reported events and gaps, the batch they arrive in, and the record a catalog retains.

use super::model::{
    SemanticCatalogBinding, SemanticEventScope, SemanticQuantity, SemanticReference,
};
use super::sequence::SemanticEventSequence;
use super::{
    SemanticCaptureWindow, SemanticCausalParent, SemanticEventCoverage, SemanticEventKind,
    SemanticEventOrigin, SemanticEventSubject,
};

/// One authoritative event, or one gap the boundary discloses instead of inventing an event.
///
/// The coverage decides which fields exist: a captured record carries a kind, an origin and the
/// detail that kind requires, while a dropped or unsupported record carries none of them because
/// nothing was observed. Both live in one type so a gap still occupies its sequence number and is
/// never renumbered away to hide that something happened.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventInput {
    /// Opaque event identity, unique inside its history.
    pub event_id: String,
    /// Sequence number this record occupies inside the batch's scope.
    pub sequence: u64,
    /// Whether this record was observed, dropped or is unsupported here.
    pub coverage: SemanticEventCoverage,
    /// Kind of event; present exactly when the record was captured.
    pub kind: Option<SemanticEventKind>,
    /// Where the event came from; present exactly when the record was captured.
    pub origin: Option<SemanticEventOrigin>,
    /// The subjects of the event, actor first; empty for a disclosed gap.
    pub subjects: Vec<SemanticEventSubject>,
    /// The causal parent, stated or explicitly absent; present exactly when the kind admits a cause.
    pub causal_parent: Option<SemanticCausalParent>,
    /// The gameplay quantity this event reports, for a kind that carries one.
    pub value: Option<SemanticQuantity>,
    /// The content identity this event names, for a kind that names one.
    pub reference: Option<SemanticReference>,
    /// Bounded owner-defined display text, never a substitute for a typed value.
    pub label: Option<String>,
}

impl SemanticEventInput {
    /// Returns whether this record was observed rather than disclosed as a gap.
    #[must_use]
    pub fn is_observed(&self) -> bool {
        self.coverage.status.is_observed()
    }
}

/// One validated event retained by a catalog, bound to its catalog and its scope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventRecord {
    /// Catalog binding this record was validated against.
    pub binding: SemanticCatalogBinding,
    /// The scope this event's sequence number is monotonic inside.
    pub scope: SemanticEventScope,
    /// Validated event value.
    pub event: SemanticEventInput,
}

impl SemanticEventRecord {
    /// Returns the opaque event identity.
    #[must_use]
    pub fn event_id(&self) -> &str {
        &self.event.event_id
    }

    /// Returns whether this record was observed rather than disclosed as a gap.
    #[must_use]
    pub fn is_observed(&self) -> bool {
        self.event.is_observed()
    }

    /// Returns this event's position inside its scope.
    #[must_use]
    pub fn sequence(&self) -> SemanticEventSequence {
        SemanticEventSequence::new(self.scope.clone(), self.event.sequence)
    }
}

/// Owner-reported history: one scope, a capture window, and the events or gaps inside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventBatch {
    /// The run, branch, episode and epoch every sequence number here is monotonic inside.
    pub scope: SemanticEventScope,
    /// What the boundary observed and which spans it could not fully observe.
    pub window: SemanticCaptureWindow,
    /// Records in ascending sequence order, each occupying exactly one sequence number.
    pub events: Vec<SemanticEventInput>,
}
