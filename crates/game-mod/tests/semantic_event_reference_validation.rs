// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/semantic_event_manifest.rs"]
mod manifest_fixture;
#[path = "support/semantic_event.rs"]
mod support;

use sts2_game_mod::{
    SemanticEventError, SemanticEventKind, SemanticEventSubject, SemanticIdentityNamespace,
    SemanticSubjectRole,
};
use support::*;

fn subject(
    role: SemanticSubjectRole,
    namespace: SemanticIdentityNamespace,
    id: &str,
) -> SemanticEventSubject {
    SemanticEventSubject {
        role,
        namespace,
        subject_id: id.to_owned(),
    }
}

#[test]
fn a_targeted_kind_without_a_target_is_refused_not_left_unknown() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![actor()];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::MissingSubject("target"))
    ));
}

#[test]
fn an_effect_without_an_actor_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![target("peer.local")];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::MissingSubject("actor"))
    ));
}

#[test]
fn a_kind_that_acts_on_no_target_refuses_one() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[4].subjects = vec![actor(), target("peer.local")];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::UnexpectedSubjectRole("target"))
    ));
}

#[test]
fn one_role_named_twice_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![actor(), actor()];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::DuplicateSubjectRole("actor"))
    ));
}

#[test]
fn a_subject_minted_in_the_wrong_namespace_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![
        actor(),
        subject(
            SemanticSubjectRole::Target,
            SemanticIdentityNamespace::Definition,
            "enemy.slime",
        ),
    ];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::WrongSubjectNamespace("target"))
    ));
    assert!(SemanticIdentityNamespace::LiveInstance.admits_subject_role());
    for namespace in [
        SemanticIdentityNamespace::Definition,
        SemanticIdentityNamespace::Action,
        SemanticIdentityNamespace::Event,
    ] {
        assert!(!namespace.admits_subject_role());
    }
}

#[test]
fn a_subject_that_aliases_the_event_identity_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![
        actor(),
        subject(
            SemanticSubjectRole::Target,
            SemanticIdentityNamespace::LiveInstance,
            "event.3",
        ),
    ];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::IdentityNamespaceCollision("event_id"))
    ));
}

#[test]
fn a_subject_that_aliases_the_run_it_belongs_to_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![
        actor(),
        subject(
            SemanticSubjectRole::Target,
            SemanticIdentityNamespace::LiveInstance,
            "run.alpha",
        ),
    ];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::IdentityNamespaceCollision("scope"))
    ));
}

#[test]
fn a_subject_identity_that_looks_like_a_path_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].subjects = vec![
        actor(),
        subject(
            SemanticSubjectRole::Target,
            SemanticIdentityNamespace::LiveInstance,
            "enemy/../../etc/passwd",
        ),
    ];
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::NonOpaqueIdentity("subject_id"))
    ));
}

#[test]
fn a_quantity_kind_without_a_quantity_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[1].value = None;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::KindDetailMismatch(id)) if id == "event.2"
    ));
}

#[test]
fn a_kind_that_reports_no_quantity_refuses_one() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[4].value = Some(sts2_game_mod::SemanticQuantity {
        amount: 1,
        unit: "health".to_owned(),
    });
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::KindDetailMismatch(id)) if id == "event.5"
    ));
}

#[test]
fn a_quantity_unit_that_is_not_opaque_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[2].value = Some(sts2_game_mod::SemanticQuantity {
        amount: 5,
        unit: "c:/temp".to_owned(),
    });
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::NonOpaqueIdentity("quantity.unit"))
    ));
}

#[test]
fn a_content_reference_the_manifest_does_not_carry_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[0].reference = Some(sts2_game_mod::SemanticReference {
        entity_kind: "card".to_owned(),
        namespaced_id: "card.absent".to_owned(),
    });
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::UnknownManifestReference { namespaced_id, .. })
            if namespaced_id == "card.absent"
    ));
}

#[test]
fn a_kind_that_names_content_must_name_it() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events[0].reference = None;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::KindDetailMismatch(id)) if id == "event.1"
    ));
}

#[test]
fn an_observed_record_without_a_kind_or_origin_is_refused() {
    let mut without_kind = fixture_snapshot();
    without_kind.batch.events[2].kind = None;
    assert!(matches!(
        produce_snapshot(without_kind),
        Err(SemanticEventError::MissingKind(id)) if id == "event.3"
    ));
    let mut without_origin = fixture_snapshot();
    without_origin.batch.events[2].origin = None;
    assert!(matches!(
        produce_snapshot(without_origin),
        Err(SemanticEventError::MissingOrigin(id)) if id == "event.3"
    ));
}

#[test]
fn a_gap_that_carries_an_observed_field_is_refused() {
    let mut snapshot = fixture_snapshot();
    let mut events = fixture_events();
    let mut disclosed = gap("event.3", 3, sts2_game_mod::SemanticCoverageStatus::Dropped);
    disclosed.kind = Some(SemanticEventKind::Block);
    events[2] = disclosed;
    snapshot.batch.events = events;
    snapshot.batch.window.intervals = vec![sts2_game_mod::SemanticCoverageInterval {
        status: sts2_game_mod::SemanticCoverageStatus::Dropped,
        first_sequence: 3,
        last_sequence: 3,
    }];
    snapshot.family = fixture_family(&snapshot.batch);
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::UnexpectedKind(id)) if id == "event.3"
    ));
}

#[test]
fn a_declared_count_that_disagrees_with_the_history_is_refused() {
    let mut snapshot = fixture_snapshot();
    snapshot.family.event_count = 4;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::FamilyCountMismatch)
    ));
}

#[test]
fn another_manifest_or_producer_is_refused_as_stale_or_foreign() {
    let mut wrong_manifest = fixture_snapshot();
    wrong_manifest.manifest =
        manifest_fixture::manifest(&[("card", "card.strike")]).cursor_binding();
    assert!(matches!(
        produce_snapshot(wrong_manifest),
        Err(SemanticEventError::ManifestMismatch)
    ));
    let mut wrong_producer = fixture_snapshot();
    wrong_producer.producer_version = "someone-else-v1".to_owned();
    assert!(matches!(
        produce_snapshot(wrong_producer),
        Err(SemanticEventError::ProducerVersionMismatch)
    ));
}

#[test]
fn an_empty_history_is_refused_rather_than_published_as_observed_nothing() {
    let mut snapshot = fixture_snapshot();
    snapshot.batch.events = Vec::new();
    snapshot.family.event_count = 0;
    assert!(matches!(
        produce_snapshot(snapshot),
        Err(SemanticEventError::EmptyPresentCollection("events"))
    ));
}
