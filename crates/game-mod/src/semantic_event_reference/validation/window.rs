// SPDX-License-Identifier: MIT

//! The capture window and the spans it declares incomplete.

use super::super::model::SEMANTIC_MAX_INTERVALS;
use super::super::{
    SemanticCaptureWindow, SemanticCoverageStatus, SemanticEventError, SemanticEventInput,
};
use super::bounds::require_within;

/// Validates the capture window against the events it claims to cover.
///
/// The window is what lets a consumer tell an absent event from an unwatched one, so an inconsistent
/// window is refused rather than repaired: a declared span that claims to be captured, an interval
/// that runs backwards or overlaps, and a `history_before_capture` flag that disagrees with where
/// capture began would each make the disclosure itself untrustworthy.
pub(super) fn validate_window(
    window: &SemanticCaptureWindow,
    first: Option<&SemanticEventInput>,
    last: Option<&SemanticEventInput>,
) -> Result<(), SemanticEventError> {
    require_within(window.intervals.len(), SEMANTIC_MAX_INTERVALS, "intervals")?;
    if window.capture_start_sequence == 0 {
        return Err(SemanticEventError::InvalidSequenceStart(0));
    }
    let must_have_history_before = window.capture_start_sequence != 1;
    if window.history_before_capture != must_have_history_before {
        return Err(SemanticEventError::InvalidSequenceStart(
            window.capture_start_sequence,
        ));
    }
    let (Some(first), Some(last)) = (first, last) else {
        return Ok(());
    };
    if first.sequence != window.capture_start_sequence || last.sequence < first.sequence {
        return Err(SemanticEventError::InvalidSequenceStart(first.sequence));
    }
    validate_intervals(window, last.sequence)
}

/// Validates that declared spans are incomplete, ordered, disjoint, and inside the captured range.
fn validate_intervals(
    window: &SemanticCaptureWindow,
    last_sequence: u64,
) -> Result<(), SemanticEventError> {
    let mut previous: Option<u64> = None;
    for interval in &window.intervals {
        if interval.status == SemanticCoverageStatus::Captured
            || interval.first_sequence > interval.last_sequence
            || interval.first_sequence < window.capture_start_sequence
            || interval.last_sequence > last_sequence
        {
            return Err(SemanticEventError::InvalidCoverageInterval(
                interval.first_sequence,
            ));
        }
        if previous.is_some_and(|end| interval.first_sequence <= end) {
            return Err(SemanticEventError::InvalidCoverageInterval(
                interval.first_sequence,
            ));
        }
        previous = Some(interval.last_sequence);
    }
    Ok(())
}
