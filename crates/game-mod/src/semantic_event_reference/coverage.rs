// SPDX-License-Identifier: MIT

//! Coverage: what the boundary observed, what it dropped, and what it cannot state.

/// The coverage state of one event or one interval of a history.
///
/// Coverage is a property of the record, not of the reader. A `Dropped` or `Unsupported` interval is
/// disclosed so a consumer can see that this history is incomplete; neither is ever closed by an
/// invented event, a zeroed quantity, or a renumbered sequence.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticCoverageStatus {
    /// Observed at the host boundary.
    Captured,
    /// Not observed because the capture dropped it.
    Dropped,
    /// Not expressible in this vocabulary, so it is not represented as an event.
    Unsupported,
}

impl SemanticCoverageStatus {
    /// Every status, in a stable order.
    pub const ALL: [Self; 3] = [Self::Captured, Self::Dropped, Self::Unsupported];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::Dropped => "dropped",
            Self::Unsupported => "unsupported",
        }
    }

    /// Returns whether an event with this status carries authoritative gameplay values.
    #[must_use]
    pub const fn is_observed(self) -> bool {
        matches!(self, Self::Captured)
    }
}

/// What one recorded event can say about itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventCoverage {
    /// Whether this event was observed, dropped or is unsupported here.
    pub status: SemanticCoverageStatus,
    /// Optional bounded owner-defined label naming the reason, never a substitute for a value.
    pub label: Option<String>,
}

impl SemanticEventCoverage {
    /// A captured event with no extra label.
    #[must_use]
    pub const fn captured() -> Self {
        Self {
            status: SemanticCoverageStatus::Captured,
            label: None,
        }
    }

    /// A disclosed gap carrying a bounded owner-defined reason.
    #[must_use]
    pub fn gap(status: SemanticCoverageStatus, label: &str) -> Self {
        Self {
            status,
            label: Some(label.to_owned()),
        }
    }
}

/// One contiguous span of the sequence whose events are not all captured.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCoverageInterval {
    /// Why the span is incomplete; a `Captured` span is never a declared interval.
    pub status: SemanticCoverageStatus,
    /// First sequence number covered by this span.
    pub first_sequence: u64,
    /// Last sequence number covered by this span, inclusive.
    pub last_sequence: u64,
}

/// What a history says about the extent of its own capture.
///
/// A consumer that cannot see the capture boundary cannot tell an absent event from an unwatched one,
/// so the window is stated explicitly: where capture began, whether history exists before that point,
/// and every span inside the captured range that is not fully captured.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticCaptureWindow {
    /// Sequence number at which capture began; this is the earliest event this history can own.
    pub capture_start_sequence: u64,
    /// Whether gameplay history exists before `capture_start_sequence`.
    pub history_before_capture: bool,
    /// Declared spans inside the captured range that are not fully captured.
    pub intervals: Vec<SemanticCoverageInterval>,
}

impl SemanticCaptureWindow {
    /// Returns whether the window begins with no unrepresented earlier history.
    #[must_use]
    pub const fn starts_at_beginning(&self) -> bool {
        !self.history_before_capture && self.capture_start_sequence == 1
    }

    /// Returns the declared span covering one sequence, if any.
    #[must_use]
    pub fn interval_covering(&self, sequence: u64) -> Option<&SemanticCoverageInterval> {
        self.intervals.iter().find(|interval| {
            sequence >= interval.first_sequence && sequence <= interval.last_sequence
        })
    }
}
