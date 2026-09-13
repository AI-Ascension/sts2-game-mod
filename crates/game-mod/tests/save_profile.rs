// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use sts2_game_mod::{
    BaselineFence, FakeSaveProfileHost, HostCompatibility, InstanceIdentity, ProfileDiscoveryError,
    ProfileDiscoveryRequest, ProfileIdentityError, ProfileReadPort, ProfileSelectionAvailability,
    ProfileSelectionPort, ProfileSelectionRejection, ProfileSelectionRequest, ProviderProfileId,
    SaveProfileBaseline, SaveProfileStatus, SaveSlotId, SaveSlotSummary, SelectionAuthority,
    SelectionIdempotencyKey, UnavailableSaveProfileHost, UserDataIdentity, WorkflowProfileId,
};

fn identity(value: &str) -> SaveSlotId {
    SaveSlotId::new(value).expect("fixture identity")
}

fn fence(digest: &str) -> BaselineFence {
    let user_data = UserDataIdentity::new("fixture-user-data").expect("user data");
    let baseline = SaveProfileBaseline::new(user_data, digest, 7).expect("baseline");
    BaselineFence::new(
        InstanceIdentity::new("fixture-instance").expect("instance"),
        baseline,
    )
    .expect("fence")
}

fn slot(value: &str, status: SaveProfileStatus, baseline: &SaveProfileBaseline) -> SaveSlotSummary {
    SaveSlotSummary::new(
        identity(value),
        HostCompatibility::new("fixture-host-v1").expect("compatibility"),
        status,
        baseline.clone(),
    )
}

fn fixture(statuses: &[(&str, SaveProfileStatus)]) -> FakeSaveProfileHost {
    let fence = fence("sha256:baseline-a");
    let slots = statuses
        .iter()
        .map(|(id, status)| slot(id, *status, fence.baseline()))
        .collect::<Vec<_>>();
    FakeSaveProfileHost::new(fence, slots, None).expect("fixture")
}

fn selected_fixture(status: SaveProfileStatus) -> FakeSaveProfileHost {
    let fence = fence("sha256:baseline-a");
    let slots = vec![
        slot("selected", status, fence.baseline()),
        slot(
            "destination",
            SaveProfileStatus::Available,
            fence.baseline(),
        ),
    ];
    FakeSaveProfileHost::new(fence, slots, Some(identity("selected"))).expect("selected fixture")
}

fn request(fence: BaselineFence, slot: &str, key: &str) -> ProfileSelectionRequest {
    ProfileSelectionRequest::new(
        fence,
        SelectionAuthority::new("fixture-authority").expect("authority"),
        identity(slot),
        SelectionIdempotencyKey::new(key).expect("idempotency key"),
    )
}

#[test]
fn identity_types_are_distinct_and_path_like_values_are_rejected() {
    let workflow = WorkflowProfileId::new("workflow-default").expect("workflow identity");
    let provider = ProviderProfileId::new("provider-default").expect("provider identity");
    assert_ne!(workflow.as_str(), provider.as_str());
    assert_eq!(
        UserDataIdentity::new("/unknown/existing/profile"),
        Err(ProfileIdentityError::PathLike {
            kind: "user_data_id"
        })
    );
    assert_eq!(
        SaveSlotId::new("../valued-profile"),
        Err(ProfileIdentityError::PathLike {
            kind: "save_slot_id"
        })
    );
}

#[test]
fn discovery_is_bounded_sorted_redacted_and_has_no_implicit_selection() {
    let store = fixture(&[
        ("slot-b", SaveProfileStatus::Available),
        ("slot-a", SaveProfileStatus::Unsupported),
    ]);
    let read = store
        .discover(&ProfileDiscoveryRequest::new(store.fence().expect("fence")))
        .expect("discovery");
    assert_eq!(
        read.slots()
            .iter()
            .map(|slot| slot.slot_id().as_str())
            .collect::<Vec<_>>(),
        vec!["slot-a", "slot-b"]
    );
    assert!(read.current_selection().is_none());
    assert_eq!(read.slots()[0].status(), SaveProfileStatus::Unsupported);
    assert_eq!(read.slots()[0].baseline().digest(), "sha256:baseline-a");
    assert!(!format!("{read:?}").contains("path"));
    assert!(!format!("{read:?}").contains("payload"));
}

#[test]
fn selection_requires_explicit_slot_and_is_idempotent_after_lost_response() {
    let mut store = fixture(&[("slot-a", SaveProfileStatus::Available)]);
    assert_eq!(
        store.selection_availability(),
        ProfileSelectionAvailability::SyntheticFixtureOnly
    );
    let first = store
        .select(request(
            store.fence().expect("fence"),
            "slot-a",
            "operation-1",
        ))
        .expect("first selection");
    assert_eq!(
        first.readback().current_selection().map(SaveSlotId::as_str),
        Some("slot-a")
    );
    assert_eq!(store.mutation_count(), 1);

    let second = store
        .select(request(
            store.fence().expect("fence"),
            "slot-a",
            "operation-1",
        ))
        .expect("idempotent retry");
    assert_eq!(first, second);
    assert_eq!(store.mutation_count(), 1);
    assert_eq!(
        store
            .reconcile(&SelectionIdempotencyKey::new("operation-1").expect("key"))
            .expect("reconciliation"),
        first
    );
}

#[test]
fn failed_requests_never_mutate_selection() {
    let mut store = fixture(&[
        ("active", SaveProfileStatus::ActiveRun),
        ("busy", SaveProfileStatus::InUse),
        ("pending", SaveProfileStatus::PendingSave),
        ("failed", SaveProfileStatus::FailedSave),
        ("unsupported", SaveProfileStatus::Unsupported),
        ("available", SaveProfileStatus::Available),
    ]);
    let cases = [
        ("active", ProfileSelectionRejection::ActiveRun),
        ("busy", ProfileSelectionRejection::InUse),
        ("pending", ProfileSelectionRejection::PendingSave),
        ("failed", ProfileSelectionRejection::FailedSave),
        ("unsupported", ProfileSelectionRejection::UnsupportedSlot),
        ("missing", ProfileSelectionRejection::UnknownSlot),
    ];
    for (slot, expected) in cases {
        assert_eq!(
            store.select(request(store.fence().expect("fence"), slot, slot)),
            Err(expected)
        );
    }
    store.set_selection_busy(true);
    assert_eq!(
        store.select(request(
            store.fence().expect("fence"),
            "available",
            "busy-operation"
        )),
        Err(ProfileSelectionRejection::ConcurrentSelection)
    );
    assert_eq!(store.mutation_count(), 0);
}

#[test]
fn switching_away_from_an_unsettled_current_slot_never_mutates() {
    let cases = [
        (
            SaveProfileStatus::ActiveRun,
            ProfileSelectionRejection::ActiveRun,
        ),
        (SaveProfileStatus::InUse, ProfileSelectionRejection::InUse),
        (
            SaveProfileStatus::PendingSave,
            ProfileSelectionRejection::PendingSave,
        ),
        (
            SaveProfileStatus::FailedSave,
            ProfileSelectionRejection::FailedSave,
        ),
    ];
    for (status, expected) in cases {
        let mut store = selected_fixture(status);
        assert_eq!(
            store.select(request(
                store.fence().expect("fence"),
                "destination",
                "switch-operation"
            )),
            Err(expected)
        );
        let readback = store
            .discover(&ProfileDiscoveryRequest::new(store.fence().expect("fence")))
            .expect("readback");
        assert_eq!(
            readback.current_selection().map(SaveSlotId::as_str),
            Some("selected")
        );
        assert_eq!(store.mutation_count(), 0);
    }
}

#[test]
fn stale_and_wrong_instance_or_user_data_reject_without_mutation() {
    let mut store = fixture(&[("slot-a", SaveProfileStatus::Available)]);
    let original = store.fence().expect("fence");

    let stale = fence("sha256:baseline-b");
    assert_eq!(
        store.select(request(stale, "slot-a", "stale")),
        Err(ProfileSelectionRejection::StaleBaseline)
    );

    let wrong_instance = BaselineFence::new(
        InstanceIdentity::new("other-instance").expect("instance"),
        original.baseline().clone(),
    )
    .expect("fence");
    assert_eq!(
        store.select(request(wrong_instance, "slot-a", "wrong-instance")),
        Err(ProfileSelectionRejection::WrongInstance)
    );

    let other_user = UserDataIdentity::new("other-user-data").expect("user data");
    let wrong_user_data_baseline =
        SaveProfileBaseline::new(other_user, "sha256:baseline-a", 7).expect("baseline");
    let wrong_user_data =
        BaselineFence::new(original.instance_id().clone(), wrong_user_data_baseline)
            .expect("fence");
    assert_eq!(
        store.select(request(wrong_user_data, "slot-a", "wrong-user-data")),
        Err(ProfileSelectionRejection::WrongUserData)
    );
    assert_eq!(store.mutation_count(), 0);
}

#[test]
fn idempotency_key_cannot_switch_target_and_unavailable_host_is_explicit() {
    let mut store = fixture(&[
        ("slot-a", SaveProfileStatus::Available),
        ("slot-b", SaveProfileStatus::Available),
    ]);
    store
        .select(request(store.fence().expect("fence"), "slot-a", "same-key"))
        .expect("first selection");
    assert_eq!(
        store.select(request(store.fence().expect("fence"), "slot-b", "same-key")),
        Err(ProfileSelectionRejection::IdempotencyConflict)
    );

    let mut unavailable = UnavailableSaveProfileHost;
    assert_eq!(
        unavailable.discover(&ProfileDiscoveryRequest::new(fence("sha256:baseline-a"))),
        Err(ProfileDiscoveryError::UnavailableHost)
    );
    assert_eq!(
        unavailable.selection_availability(),
        ProfileSelectionAvailability::UnavailableHost
    );
    assert_eq!(
        unavailable.select(request(
            fence("sha256:baseline-a"),
            "slot-a",
            "host-unavailable"
        )),
        Err(ProfileSelectionRejection::UnavailableHost)
    );
}
