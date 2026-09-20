// SPDX-License-Identifier: MIT

//! One fixture combat history and the source that answers with it.

use std::cell::Cell;

use sts2_game_mod::{
    ContentManifest, SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION, SemanticCaptureWindow,
    SemanticCatalogBinding, SemanticCausalParent, SemanticCoverageStatus, SemanticEventBatch,
    SemanticEventCatalogProducer, SemanticEventCoverage, SemanticEventInput, SemanticEventKind,
    SemanticEventOrigin, SemanticEventReference, SemanticEventScope, SemanticEventSnapshot,
    SemanticEventSource, SemanticEventSourceError, SemanticEventSubject, SemanticFamilyCoverage,
    SemanticFamilyState, SemanticHistoryCatalog, SemanticHistoryFence, SemanticIdentityNamespace,
    SemanticQuantity, SemanticReference, SemanticSubjectRole,
};

use crate::manifest_fixture::fixture_manifest;

/// The scope every fixture history is monotonic inside.
pub fn fixture_scope() -> SemanticEventScope {
    SemanticEventScope {
        run_id: "run.alpha".to_owned(),
        branch_id: "branch.main".to_owned(),
        episode: 1,
        epoch: 12,
    }
}

pub fn actor() -> SemanticEventSubject {
    subject(SemanticSubjectRole::Actor, "peer.local")
}

pub fn target(id: &str) -> SemanticEventSubject {
    subject(SemanticSubjectRole::Target, id)
}

fn subject(role: SemanticSubjectRole, id: &str) -> SemanticEventSubject {
    SemanticEventSubject {
        role,
        namespace: SemanticIdentityNamespace::LiveInstance,
        subject_id: id.to_owned(),
    }
}

fn quantity(amount: i64, unit: &str) -> SemanticQuantity {
    SemanticQuantity {
        amount,
        unit: unit.to_owned(),
    }
}

fn reference(entity_kind: &str, id: &str) -> SemanticReference {
    SemanticReference {
        entity_kind: entity_kind.to_owned(),
        namespaced_id: id.to_owned(),
    }
}

/// A captured event with the coverage a fully observed record carries.
pub fn captured(
    event_id: &str,
    sequence: u64,
    kind: SemanticEventKind,
    subjects: Vec<SemanticEventSubject>,
) -> SemanticEventInput {
    SemanticEventInput {
        event_id: event_id.to_owned(),
        sequence,
        coverage: SemanticEventCoverage::captured(),
        kind: Some(kind),
        origin: Some(SemanticEventOrigin::Native),
        subjects,
        causal_parent: if kind.admits_cause() {
            Some(SemanticCausalParent::not_stated())
        } else {
            None
        },
        value: None,
        reference: None,
        label: None,
    }
}

/// A disclosed gap occupying one sequence number.
pub fn gap(event_id: &str, sequence: u64, status: SemanticCoverageStatus) -> SemanticEventInput {
    SemanticEventInput {
        event_id: event_id.to_owned(),
        sequence,
        coverage: SemanticEventCoverage::gap(status, "capture dropped this step"),
        kind: None,
        origin: None,
        subjects: Vec::new(),
        causal_parent: None,
        value: None,
        reference: None,
        label: None,
    }
}

/// The five-event fixture history: a card play, a damage with a stated parent, a block, a heal and
/// a room transition.
pub fn fixture_events() -> Vec<SemanticEventInput> {
    let mut card = captured("event.1", 1, SemanticEventKind::CardPlayed, vec![actor()]);
    card.reference = Some(reference("card", "card.strike"));
    let mut damage = captured(
        "event.2",
        2,
        SemanticEventKind::Damage,
        vec![actor(), target("enemy.slime")],
    );
    damage.value = Some(quantity(6, "health"));
    damage.causal_parent = Some(SemanticCausalParent::stated("event.1"));
    let mut block = captured(
        "event.3",
        3,
        SemanticEventKind::Block,
        vec![actor(), target("peer.local")],
    );
    block.value = Some(quantity(5, "block"));
    let mut heal = captured(
        "event.4",
        4,
        SemanticEventKind::Heal,
        vec![actor(), target("peer.local")],
    );
    heal.value = Some(quantity(2, "health"));
    heal.causal_parent = Some(SemanticCausalParent::not_stated());
    vec![
        card,
        damage,
        block,
        heal,
        captured(
            "event.5",
            5,
            SemanticEventKind::RoomTransitioned,
            vec![actor()],
        ),
    ]
}

/// The window every fixture history states: capture began at sequence 1 with nothing before it.
pub fn fixture_window() -> SemanticCaptureWindow {
    SemanticCaptureWindow {
        capture_start_sequence: 1,
        history_before_capture: false,
        intervals: Vec::new(),
    }
}

/// The batch the fixture source answers with.
pub fn fixture_batch() -> SemanticEventBatch {
    SemanticEventBatch {
        scope: fixture_scope(),
        window: fixture_window(),
        events: fixture_events(),
    }
}

/// The declared coverage that agrees with the fixture history.
pub fn fixture_family(batch: &SemanticEventBatch) -> SemanticFamilyCoverage {
    SemanticFamilyCoverage {
        state: SemanticFamilyState::Handled,
        event_count: batch.events.len(),
        gap_count: batch
            .events
            .iter()
            .filter(|event| !event.is_observed())
            .count(),
    }
}

/// The source snapshot the fixture producer is handed.
pub fn fixture_snapshot() -> SemanticEventSnapshot {
    let batch = fixture_batch();
    SemanticEventSnapshot {
        manifest: fixture_manifest().cursor_binding(),
        producer_version: SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: fixture_family(&batch),
        batch,
    }
}

/// A source that answers with one owned snapshot and counts its reads.
pub struct FixtureSource {
    pub snapshot: SemanticEventSnapshot,
    pub reads: Cell<usize>,
}

impl SemanticEventSource for FixtureSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<SemanticEventSnapshot, SemanticEventSourceError> {
        self.reads.set(self.reads.get() + 1);
        Ok(self.snapshot.clone())
    }
}

pub fn source(snapshot: SemanticEventSnapshot) -> FixtureSource {
    FixtureSource {
        snapshot,
        reads: Cell::new(0),
    }
}

pub fn produce_snapshot(
    snapshot: SemanticEventSnapshot,
) -> Result<SemanticHistoryCatalog, sts2_game_mod::SemanticEventError> {
    SemanticEventCatalogProducer::new().produce(&fixture_manifest(), &source(snapshot))
}

/// Produces the fixture catalog and the manifest it was bound to.
pub fn fixture_catalog() -> (ContentManifest, SemanticHistoryCatalog) {
    let manifest = fixture_manifest();
    let catalog = SemanticEventCatalogProducer::new()
        .produce(&manifest, &source(fixture_snapshot()))
        .expect("catalog");
    (manifest, catalog)
}

pub fn event_reference(catalog: &SemanticHistoryCatalog, event_id: &str) -> SemanticEventReference {
    SemanticEventReference {
        catalog: catalog.binding().clone(),
        event_id: event_id.to_owned(),
    }
}

pub fn fence() -> SemanticHistoryFence {
    SemanticHistoryFence {
        run_id: "run.alpha".to_owned(),
        branch_id: "branch.main".to_owned(),
        episode: 1,
        epoch: 12,
    }
}

pub fn list_query(
    catalog: &SemanticHistoryCatalog,
    limit: usize,
) -> sts2_game_mod::SemanticEventListQuery {
    sts2_game_mod::SemanticEventListQuery {
        reference: catalog.binding().clone(),
        scope: sts2_game_mod::SemanticHistoryScope::History,
        event_scope: fixture_scope(),
        limit,
        continuation: None,
        live_fence: None,
    }
}

pub fn binding() -> SemanticCatalogBinding {
    SemanticCatalogBinding {
        manifest: fixture_manifest().cursor_binding(),
        producer_version: SEMANTIC_EVENT_REFERENCE_PRODUCER_VERSION.to_owned(),
    }
}
