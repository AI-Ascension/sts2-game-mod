// SPDX-License-Identifier: MIT

//! The owner-local snapshot, source boundary and producer that bind one history to one manifest.

use crate::ContentManifest;

pub use super::catalog_reader::SemanticHistoryCatalog;
use super::error::map_source_error;
use super::validation::validate_history;
use super::{
    SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION, SemanticCatalogBinding, SemanticEventBatch,
    SemanticEventError, SemanticEventSourceError, SemanticFamilyCoverage, SemanticFamilyState,
};

/// Bounded source snapshot used to construct one immutable semantic-history catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventSnapshot {
    /// Existing content-manifest invalidation witness.
    pub manifest: crate::ContentCursorBinding,
    /// Exact owner-local producer identity.
    pub producer_version: String,
    /// Explicit support state for the history family with the counts the source reports.
    pub family: SemanticFamilyCoverage,
    /// The history, empty when the source declares the family unavailable.
    pub batch: SemanticEventBatch,
}

/// Owner-local source boundary for copied semantic gameplay history.
pub trait SemanticEventSource {
    /// Copies a bounded history without acting in the run or touching a save.
    fn read_catalog(
        &self,
        manifest: &ContentManifest,
    ) -> Result<SemanticEventSnapshot, SemanticEventSourceError>;
}

/// Producer that binds one history to an immutable content manifest.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticEventCatalogProducer;

impl SemanticEventCatalogProducer {
    /// Creates the source-only producer.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Produces an immutable catalog or rejects the entire source snapshot.
    pub fn produce<S: SemanticEventSource>(
        &self,
        manifest: &ContentManifest,
        source: &S,
    ) -> Result<SemanticHistoryCatalog, SemanticEventError> {
        let snapshot = source.read_catalog(manifest).map_err(map_source_error)?;
        validate_fences(manifest, &snapshot)?;
        let binding = SemanticCatalogBinding {
            manifest: snapshot.manifest,
            producer_version: snapshot.producer_version,
        };
        let family = snapshot.family;
        if family.state != SemanticFamilyState::Handled {
            if !is_empty_batch(&snapshot.batch) {
                return Err(SemanticEventError::InvalidInput("batch"));
            }
            return Ok(SemanticHistoryCatalog::from_parts(binding, family, None));
        }
        validate_history(&snapshot.batch, manifest)?;
        if family.event_count != snapshot.batch.events.len()
            || family.gap_count != declared_gap_count(&snapshot.batch)
        {
            return Err(SemanticEventError::FamilyCountMismatch);
        }
        Ok(SemanticHistoryCatalog::from_parts(
            binding,
            family,
            Some(snapshot.batch),
        ))
    }
}

/// Returns whether a declared-unavailable family carries no history at all.
fn is_empty_batch(batch: &SemanticEventBatch) -> bool {
    batch.scope.run_id.is_empty()
        && batch.scope.branch_id.is_empty()
        && batch.scope.episode == 0
        && batch.scope.epoch == 0
        && batch.events.is_empty()
        && batch.window.intervals.is_empty()
}

/// Returns how many records this history discloses rather than observes.
fn declared_gap_count(batch: &SemanticEventBatch) -> usize {
    batch
        .events
        .iter()
        .filter(|event| !event.is_observed())
        .count()
}

fn validate_fences(
    manifest: &ContentManifest,
    snapshot: &SemanticEventSnapshot,
) -> Result<(), SemanticEventError> {
    if snapshot.manifest != manifest.cursor_binding() {
        return Err(SemanticEventError::ManifestMismatch);
    }
    if snapshot.producer_version != SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION {
        return Err(SemanticEventError::ProducerVersionMismatch);
    }
    Ok(())
}
