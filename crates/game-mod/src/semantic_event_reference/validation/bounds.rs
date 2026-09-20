// SPDX-License-Identifier: MIT

//! Local count and byte bounds.

use super::super::SemanticEventError;
use super::super::definition::{SemanticEventBatch, SemanticEventInput};
use super::subjects::event_bytes;

pub(super) fn require_within(
    actual: usize,
    limit: usize,
    _field: &'static str,
) -> Result<(), SemanticEventError> {
    if actual > limit {
        return Err(SemanticEventError::HistoryTooLarge { limit, actual });
    }
    Ok(())
}

/// Returns the aggregate bytes one history record would publish.
pub(super) fn history_bytes(batch: &SemanticEventBatch) -> usize {
    batch.events.iter().map(event_bytes).sum::<usize>()
        + batch.scope.run_id.len()
        + batch.scope.branch_id.len()
}

/// Returns the bytes one optional owner-defined label contributes.
pub(super) fn label_bytes(event: &SemanticEventInput) -> usize {
    event.coverage.label.as_ref().map_or(0, String::len)
        + event.label.as_ref().map_or(0, String::len)
}
