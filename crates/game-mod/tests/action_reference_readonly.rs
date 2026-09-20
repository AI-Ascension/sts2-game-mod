// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/action_reference.rs"]
mod support;

use sts2_game_mod::{
    ActionAvailabilityQuery, ActionCatalog, ActionCatalogSnapshot, ActionDispatchAuthority,
    ActionError, ActionField, ActionPreviewClass, ActionPreviewQuery, ActionPreviewResult,
    ActionSnapshotReference, ActionVisibilityScope, FixtureActionSource,
};
use support::{
    GENERATION, fence, fixture_definitions, fixture_manifest, frame, none, not_observed,
    produce_with, snapshot,
};

/// One catalog bound to an observable source so a test can count the reads it serves.
struct Bound {
    snapshot: ActionCatalogSnapshot,
    source: FixtureActionSource,
    catalog: ActionCatalog,
}

fn bind() -> Bound {
    let manifest = fixture_manifest();
    let snapshot = snapshot(&manifest, fixture_definitions());
    let source = FixtureActionSource::new(snapshot.clone());
    let catalog = produce_with(&manifest, &source).expect("catalog");
    Bound {
        snapshot,
        source,
        catalog,
    }
}

/// Builds one preview query with an explicit fence and instance half.
fn live_query(
    catalog: &ActionCatalog,
    fence: ActionField<ActionSnapshotReference>,
    instance: ActionField<String>,
    target_id: Option<&str>,
) -> ActionPreviewQuery {
    ActionPreviewQuery {
        frame: frame(catalog, "action.alpha", GENERATION),
        fence,
        instance,
        target_id: match target_id {
            Some(target_id) => ActionField::available(target_id.to_owned()),
            None => none(),
        },
        scope: ActionVisibilityScope::Public,
    }
}

fn explanation_query(catalog: &ActionCatalog, generation: u64) -> ActionAvailabilityQuery {
    ActionAvailabilityQuery {
        frame: frame(catalog, "action.alpha", generation),
        scope: ActionVisibilityScope::Public,
    }
}

#[test]
fn repeated_reads_of_one_frame_are_identical_and_read_the_source_once() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let preview_query = live_query(&bound.catalog, none(), none(), Some("target.slime"));
    let explain_query = explanation_query(&bound.catalog, GENERATION);
    let first_preview: ActionPreviewResult = reader.preview(&preview_query).expect("preview");
    let first_explanation = reader.explain(&explain_query).expect("explanation");
    for _ in 0..4 {
        assert_eq!(
            reader.preview(&preview_query).expect("preview"),
            first_preview
        );
        assert_eq!(
            reader.explain(&explain_query).expect("explanation"),
            first_explanation
        );
    }
    assert_eq!(bound.source.reads(), 1);
    assert_eq!(bound.source.snapshot(), &bound.snapshot);
    assert_eq!(first_preview.authority, ActionDispatchAuthority::NotGranted);
    assert_eq!(bound.source.snapshot().definitions.len(), 7);
}

#[test]
fn a_static_query_contradicting_its_own_fence_is_refused() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let query = live_query(
        &bound.catalog,
        ActionField::unavailable(not_observed()),
        none(),
        None,
    );
    assert_eq!(
        reader.preview(&query).expect_err("static fence"),
        ActionError::UnexpectedLiveFence
    );
}

#[test]
fn a_live_fence_without_a_live_instance_is_refused() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let query = live_query(
        &bound.catalog,
        ActionField::available(fence()),
        none(),
        None,
    );
    assert_eq!(
        reader.preview(&query).expect_err("missing instance"),
        ActionError::MissingLiveFence {
            action_id: "action.alpha".to_owned(),
        }
    );
}

#[test]
fn a_live_instance_without_a_live_fence_is_refused() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let query = live_query(
        &bound.catalog,
        none(),
        ActionField::available("action-instance.alpha".to_owned()),
        None,
    );
    assert_eq!(
        reader.preview(&query).expect_err("missing fence"),
        ActionError::MissingLiveFence {
            action_id: "action.alpha".to_owned(),
        }
    );
}

#[test]
fn an_incomplete_live_fence_is_refused_rather_than_reconciled() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let mut incomplete = fence();
    incomplete.run_id = String::new();
    let query = live_query(
        &bound.catalog,
        ActionField::available(incomplete),
        ActionField::available("action-instance.alpha".to_owned()),
        Some("target.slime"),
    );
    assert_eq!(
        reader.preview(&query).expect_err("incomplete fence"),
        ActionError::MissingLiveFence {
            action_id: "action.alpha".to_owned(),
        }
    );
}

#[test]
fn a_fully_live_query_still_grants_no_dispatch_authority() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let query = live_query(
        &bound.catalog,
        ActionField::available(fence()),
        ActionField::available("action-instance.alpha".to_owned()),
        Some("target.slime"),
    );
    let result = reader.preview(&query).expect("preview");
    assert_eq!(result.class, ActionPreviewClass::DeterministicExact);
    assert_eq!(result.consequence_count, 1);
    assert_eq!(result.authority, ActionDispatchAuthority::NotGranted);
    assert!(result.requires_fresh_validation());
}

#[test]
fn a_frame_from_another_generation_is_refused_by_explain_and_preview() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Public);
    let expected = ActionError::StaleActionReference {
        action_id: "action.alpha".to_owned(),
        referenced: GENERATION + 1,
        current: GENERATION,
    };
    let stale = ActionPreviewQuery {
        frame: frame(&bound.catalog, "action.alpha", GENERATION + 1),
        fence: none(),
        instance: none(),
        target_id: ActionField::available("target.slime".to_owned()),
        scope: ActionVisibilityScope::Public,
    };
    assert_eq!(reader.preview(&stale).expect_err("stale frame"), expected);
    assert_eq!(
        reader
            .explain(&explanation_query(&bound.catalog, GENERATION + 1))
            .expect_err("stale frame"),
        expected
    );
}

#[test]
fn a_hidden_action_is_unreachable_and_stays_unreachable_on_retry() {
    let bound = bind();
    let reader = bound.catalog.reader(ActionVisibilityScope::Owner);
    let hidden = ActionPreviewQuery {
        frame: frame(&bound.catalog, "action.eta", GENERATION),
        fence: none(),
        instance: none(),
        target_id: none(),
        scope: ActionVisibilityScope::Owner,
    };
    for _ in 0..3 {
        assert_eq!(
            reader.preview(&hidden).expect_err("hidden action"),
            ActionError::ExcludedByScope
        );
    }
    assert_eq!(
        reader
            .explain(&ActionAvailabilityQuery {
                frame: frame(&bound.catalog, "action.eta", GENERATION),
                scope: ActionVisibilityScope::Owner,
            })
            .expect_err("hidden action"),
        ActionError::ExcludedByScope
    );
}
