// SPDX-License-Identifier: MIT

//! Fail-closed validation of one history before it enters an immutable catalog.

mod bounds;
mod causal;
mod events;
mod manifest;
mod subjects;
mod window;

use std::collections::BTreeMap;

use crate::ContentManifest;

use self::bounds::{history_bytes, require_within};
use self::events::{validate_event, validate_sequences};
use self::subjects::validate_subjects;
use self::window::validate_window;

use super::identity::validate_opaque_identity;
use super::model::{SEMANTIC_MAX_EVENTS, SEMANTIC_MAX_HISTORY_BYTES};
use super::{SemanticEventBatch, SemanticEventError, SemanticEventInput};

/// Validates one source-owned history before it enters an immutable catalog.
pub(super) fn validate_history(
    batch: &SemanticEventBatch,
    manifest: &ContentManifest,
) -> Result<(), SemanticEventError> {
    validate_opaque_identity(&batch.scope.run_id, "run_id")?;
    validate_opaque_identity(&batch.scope.branch_id, "branch_id")?;
    if batch.events.is_empty() {
        return Err(SemanticEventError::EmptyPresentCollection("events"));
    }
    require_within(batch.events.len(), SEMANTIC_MAX_EVENTS, "events")?;
    validate_window(&batch.window, batch.events.first(), batch.events.last())?;
    validate_sequences(&batch.events, &batch.window)?;
    let observed = observed_events(&batch.events);
    for event in &batch.events {
        validate_event(event, &batch.window, manifest)?;
        validate_subjects(event, &batch.scope)?;
        causal::validate_causal_parent(event, &observed)?;
    }
    require_within(
        history_bytes(batch),
        SEMANTIC_MAX_HISTORY_BYTES,
        "history_bytes",
    )
}

/// Returns the identity and position of every event this boundary observed.
///
/// A disclosed gap is not an event and is deliberately absent, so a parent that names one is refused
/// as unknown rather than accepted as a cause that was never observed.
fn observed_events(events: &[SemanticEventInput]) -> BTreeMap<&str, u64> {
    events
        .iter()
        .filter(|event| event.is_observed())
        .map(|event| (event.event_id.as_str(), event.sequence))
        .collect()
}
