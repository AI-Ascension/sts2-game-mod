// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/action_reference.rs"]
mod support;

use sts2_game_mod::{
    ActionFieldStatus, ActionListQuery, ActionRefusalReason, ActionTargetListQuery,
    ActionVisibilityScope,
};
use support::{
    FIXTURE_IDS, GENERATION, fixture, frame, manifest, manifest_without_actions, produce, snapshot,
};

#[test]
fn catalog_binds_the_fixture_and_reports_its_family_coverage() {
    let (manifest, catalog) = fixture();
    assert_eq!(catalog.binding.manifest, manifest.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(catalog.len(), FIXTURE_IDS.len());
    assert!(!catalog.is_empty());
    assert_eq!(catalog.family.definition_count, FIXTURE_IDS.len());
}

#[test]
fn inventorying_the_action_family_without_a_definition_produces_an_empty_catalog() {
    let manifest = manifest_without_actions();
    let catalog = produce(&manifest, snapshot(&manifest, Vec::new())).expect("catalog");
    assert!(catalog.is_empty());
}

#[test]
fn lists_visible_actions_in_identity_order_without_the_owner_only_or_hidden_ones() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(ActionVisibilityScope::Public);
    let page = reader
        .list(&ActionListQuery {
            locale: "en-US".to_owned(),
            scope: ActionVisibilityScope::Public,
            limit: 64,
            continuation: None,
        })
        .expect("page");
    assert!(page.complete);
    assert_eq!(page.total, 5);
    let ids = page
        .entries
        .iter()
        .map(|entry| entry.reference.action_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "action.alpha",
            "action.beta",
            "action.delta",
            "action.epsilon",
            "action.gamma"
        ]
    );
}

#[test]
fn an_owner_only_action_is_reachable_from_the_locked_reference_scope_only() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(ActionVisibilityScope::Reference);
    let page = reader
        .list(&ActionListQuery {
            locale: "en-US".to_owned(),
            scope: ActionVisibilityScope::Reference,
            limit: 64,
            continuation: None,
        })
        .expect("page");
    let ids = page
        .entries
        .iter()
        .map(|entry| entry.reference.action_id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"action.zeta"));
    assert!(!ids.contains(&"action.eta"));
}

#[test]
fn pages_actions_by_single_use_continuation_and_refuses_a_reused_one() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(ActionVisibilityScope::Public);
    let first = reader
        .list(&ActionListQuery {
            locale: "en-US".to_owned(),
            scope: ActionVisibilityScope::Public,
            limit: 2,
            continuation: None,
        })
        .expect("first");
    assert_eq!(first.entries.len(), 2);
    assert!(!first.complete);
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&ActionListQuery {
            locale: "en-US".to_owned(),
            scope: ActionVisibilityScope::Public,
            limit: 2,
            continuation: Some(continuation.clone()),
        })
        .expect("second");
    assert_eq!(second.entries.len(), 2);
    assert_eq!(
        second.entries[0].reference.action_id.as_str(),
        "action.delta"
    );
    let reused = reader.list(&ActionListQuery {
        locale: "en-US".to_owned(),
        scope: ActionVisibilityScope::Public,
        limit: 2,
        continuation: Some(continuation),
    });
    assert_eq!(
        reused.expect_err("reused continuation"),
        sts2_game_mod::ActionError::InvalidContinuation
    );
}

#[test]
fn lists_targets_of_one_generation_with_live_and_refused_eligibility() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(ActionVisibilityScope::Public);
    let page = reader
        .list_targets(&ActionTargetListQuery {
            frame: frame(&catalog, "action.alpha", GENERATION),
            scope: ActionVisibilityScope::Public,
            limit: 64,
            continuation: None,
        })
        .expect("targets");
    assert!(page.complete);
    assert_eq!(page.total, 4);
    assert_eq!(page.available, 2);
    assert_eq!(page.targets_status, ActionFieldStatus::Available);
    let corpse = page
        .entries
        .iter()
        .find(|entry| entry.reference.target_id == "target.corpse")
        .expect("corpse");
    assert!(!corpse.eligibility.is_available());
    assert_eq!(
        corpse.eligibility.refusal(),
        Some(&ActionRefusalReason::DeadTarget)
    );
    assert_eq!(
        corpse.eligibility.reason_text.value(),
        Some("that enemy is no longer alive")
    );
}

#[test]
fn refuses_a_frame_from_another_generation_and_a_reference_from_another_manifest() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(ActionVisibilityScope::Public);
    let stale = reader.list_targets(&ActionTargetListQuery {
        frame: frame(&catalog, "action.alpha", GENERATION + 1),
        scope: ActionVisibilityScope::Public,
        limit: 64,
        continuation: None,
    });
    assert_eq!(
        stale.expect_err("stale frame"),
        sts2_game_mod::ActionError::StaleActionReference {
            action_id: "action.alpha".to_owned(),
            referenced: GENERATION + 1,
            current: GENERATION,
        }
    );
    let other = produce(
        &manifest(&["action.alpha"]),
        snapshot(&support::fixture_manifest(), support::fixture_definitions()),
    )
    .expect_err("fence mismatch");
    assert_eq!(other, sts2_game_mod::ActionError::ManifestMismatch);
    let mut reader = catalog.reader(ActionVisibilityScope::Public);
    let page = reader
        .list(&ActionListQuery {
            locale: "fr-FR".to_owned(),
            scope: ActionVisibilityScope::Public,
            limit: 64,
            continuation: None,
        })
        .expect_err("locale");
    assert_eq!(page, sts2_game_mod::ActionError::LocaleMismatch);
}

#[test]
fn a_hidden_action_is_unreachable_at_every_scope() {
    let (_, catalog) = fixture();
    for scope in [
        ActionVisibilityScope::Public,
        ActionVisibilityScope::Reference,
        ActionVisibilityScope::Owner,
    ] {
        let reader = catalog.reader(scope);
        let error = reader
            .get(&frame(&catalog, "action.eta", GENERATION).action, scope)
            .expect_err("hidden");
        assert_eq!(error, sts2_game_mod::ActionError::ExcludedByScope);
    }
}

#[test]
fn an_unknown_action_identity_is_not_found_rather_than_invented() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(ActionVisibilityScope::Public);
    let error = reader
        .get(
            &frame(&catalog, "action.missing", GENERATION).action,
            ActionVisibilityScope::Public,
        )
        .expect_err("missing");
    assert_eq!(error, sts2_game_mod::ActionError::NotFound);
}
