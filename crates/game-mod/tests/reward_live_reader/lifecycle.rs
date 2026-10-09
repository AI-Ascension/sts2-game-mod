// SPDX-License-Identifier: MIT

use std::sync::atomic::Ordering;

use super::support::*;
use sts2_game_mod::{
    RewardField, RewardLiveError, RewardLiveInvalidation, RewardLiveRoomId, RewardVisibilityScope,
};

#[test]
fn every_capture_fences_reused_raw_ids_and_source_revision_changes() {
    let catalog = test_catalog();
    let first_input = base_snapshot(&catalog);
    let mut second_input = base_snapshot(&catalog);
    second_input.state_generation = 2;
    second_input.room_id = RewardLiveRoomId::new("room:two").expect("room ID");
    second_input.run_id = sts2_game_mod::RewardLiveRunId::new("run:two").expect("run ID");
    second_input.epoch = 2;
    second_input.source_revision = RewardField::Available("source:revision-two".to_owned());
    let (mut reader, counters) = reader_for(&catalog, [Ok(first_input), Ok(second_input)]);

    let first = reader.capture().expect("first capture");
    let first_offer = reader
        .list_offers(offer_query(first.clone(), 64))
        .expect("first listing")
        .entries[0]
        .reference
        .clone();
    assert_eq!(first.snapshot_id().as_str(), "snapshot:repeat");
    assert_eq!(
        first.source_revision(),
        &RewardField::Available("source:revision-one".to_owned())
    );

    let second = reader.capture().expect("second capture");
    assert_eq!(second.snapshot_id().as_str(), "snapshot:repeat");
    assert_eq!(second.room_id(), "room:two");
    assert_eq!(second.generations(), (2, 2));
    assert_eq!(
        second.source_revision(),
        &RewardField::Available("source:revision-two".to_owned())
    );
    assert!(matches!(
        reader.get_offer(&first_offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    assert_eq!(
        reader
            .list_offers(offer_query(second, 64))
            .expect("new listing")
            .total,
        6
    );
    assert_eq!(counters.reads.load(Ordering::SeqCst), 2);
}

#[test]
fn explicit_claim_room_and_instance_changes_invalidate_all_live_handles() {
    let catalog = test_catalog();
    let first_input = base_snapshot(&catalog);
    let mut second_input = base_snapshot(&catalog);
    second_input.room_id = RewardLiveRoomId::new("room:two").expect("room ID");
    let mut third_input = base_snapshot(&catalog);
    third_input.room_id = RewardLiveRoomId::new("room:three").expect("room ID");
    third_input.state_generation = 3;
    third_input.source_revision = RewardField::Available("source:revision-three".to_owned());
    let mut fourth_input = base_snapshot(&catalog);
    fourth_input.instance_id =
        sts2_game_mod::RewardLiveInstanceId::new("game:alternate").expect("new instance");
    fourth_input.state_generation = 4;
    fourth_input.room_id = RewardLiveRoomId::new("room:alternate").expect("room ID");
    fourth_input.source_revision = RewardField::Available("source:revision-four".to_owned());
    let (mut reader, _) = reader_for(
        &catalog,
        [
            Ok(first_input),
            Ok(second_input),
            Ok(third_input),
            Ok(fourth_input),
        ],
    );

    let first = reader.capture().expect("capture");
    let first_page = reader.list_offers(offer_query(first, 1)).expect("listing");
    let first_offer = first_page.entries[0].reference.clone();
    let first_continuation = first_page.continuation.expect("continuation");
    reader
        .invalidate(RewardLiveInvalidation::ClaimOrSelection)
        .expect("claim invalidation");
    assert!(matches!(
        reader.get_offer(&first_offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    let mut stale_page = offer_query(first_page.snapshot, 1);
    stale_page.continuation = Some(first_continuation);
    assert!(matches!(
        reader.list_offers(stale_page),
        Err(RewardLiveError::StaleReference)
    ));
    assert_eq!(
        reader.last_invalidation(),
        Some(RewardLiveInvalidation::ClaimOrSelection)
    );

    let snapshot = reader.capture().expect("fresh capture");
    let offer = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("fresh listing")
        .entries[0]
        .reference
        .clone();
    reader
        .invalidate(RewardLiveInvalidation::RoomChanged)
        .expect("room invalidation");
    assert!(matches!(
        reader.get_offer(&offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    let snapshot = reader.capture().expect("capture after room transition");
    let offer = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("post-room listing")
        .entries[0]
        .reference
        .clone();
    reader
        .rebind_instance(
            sts2_game_mod::RewardLiveInstanceId::new("game:alternate").expect("new instance"),
        )
        .expect("instance invalidation");
    assert!(matches!(
        reader.get_offer(&offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    assert_eq!(
        reader.last_invalidation(),
        Some(RewardLiveInvalidation::ProfileChanged)
    );
    let alternate = reader.capture().expect("capture selected instance");
    assert_eq!(alternate.instance_id(), "game:alternate");
}

#[test]
fn failed_fresh_capture_clears_prior_snapshot_instead_of_serving_stale_data() {
    let catalog = test_catalog();
    let (mut reader, counters) = reader_for(
        &catalog,
        [
            Ok(base_snapshot(&catalog)),
            Err(sts2_game_mod::RewardSourceError::AccessDenied),
        ],
    );
    let snapshot = reader.capture().expect("initial capture");
    let offer = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("initial listing")
        .entries[0]
        .reference
        .clone();
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::Source(
            sts2_game_mod::RewardSourceError::AccessDenied
        ))
    ));
    assert_eq!(
        reader.last_invalidation(),
        Some(RewardLiveInvalidation::SourceFailure)
    );
    assert_eq!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    );
    assert!(matches!(
        reader.get_offer(&offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    assert_eq!(counters.reads.load(Ordering::SeqCst), 2);
    assert_eq!(counters.claims.load(Ordering::SeqCst), 0);
    assert_eq!(counters.rng_evaluations.load(Ordering::SeqCst), 0);
}

#[test]
fn malformed_fresh_capture_cannot_leave_an_older_snapshot_current() {
    let catalog = test_catalog();
    let mut malformed = base_snapshot(&catalog);
    malformed.catalog.producer_version = "other-producer".to_owned();
    let (mut reader, _) = reader_for(&catalog, [Ok(base_snapshot(&catalog)), Ok(malformed)]);
    let snapshot = reader.capture().expect("initial capture");
    let offer = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("initial listing")
        .entries[0]
        .reference
        .clone();
    assert!(matches!(
        reader.capture(),
        Err(RewardLiveError::BindingMismatch("catalog"))
    ));
    assert!(matches!(
        reader.get_offer(&offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    assert_eq!(
        reader.retained_source_bytes(),
        Err(RewardLiveError::NoCurrentSnapshot)
    );
}

#[test]
fn handles_and_continuations_are_reader_scoped_and_pages_do_not_recapture() {
    let catalog = test_catalog();
    let (mut first_reader, first_counters) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let (mut second_reader, _) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let first_snapshot = first_reader.capture().expect("first reader capture");
    let second_snapshot = second_reader.capture().expect("second reader capture");
    let first_page = first_reader
        .list_offers(offer_query(first_snapshot.clone(), 1))
        .expect("first page");
    let foreign_offer = first_page.entries[0].reference.clone();
    let continuation = first_page.continuation.expect("bounded continuation");
    assert!(matches!(
        second_reader.get_offer(&foreign_offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::WrongReader)
    ));

    let mut foreign_query = offer_query(second_snapshot.clone(), 1);
    foreign_query.continuation = Some(continuation);
    assert_eq!(
        second_reader.list_offers(foreign_query).map(|_| ()),
        Err(RewardLiveError::WrongReader)
    );

    let retry = first_reader
        .list_offers(offer_query(first_page.snapshot.clone(), 1))
        .expect("independent first page");
    let mut next_query = offer_query(first_page.snapshot.clone(), 1);
    next_query.continuation = retry.continuation;
    let next = first_reader.list_offers(next_query).expect("next page");
    assert_eq!(next.entries.len(), 1);
    assert_eq!(first_counters.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn same_reader_continuation_is_bound_to_snapshot_locale_scope_and_limit() {
    let catalog = test_catalog();
    let (mut reader, counters) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let snapshot = reader.capture().expect("capture");
    let first = reader
        .list_offers(offer_query(snapshot.clone(), 2))
        .expect("first page");
    assert_eq!(first.entries.len(), 2);
    let token = first.continuation.expect("continuation");
    let mut second_query = offer_query(snapshot.clone(), 2);
    second_query.continuation = Some(token);
    let second = reader.list_offers(second_query).expect("second page");
    assert_eq!(second.entries.len(), 2);
    assert!(second.continuation.is_some());
    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn continuation_rejects_a_different_page_size_or_visibility_scope() {
    let catalog = test_catalog();
    let (mut reader, counters) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let snapshot = reader.capture().expect("capture");
    let first = reader
        .list_offers(offer_query(snapshot.clone(), 2))
        .expect("first page");
    let mut wrong_limit = offer_query(snapshot.clone(), 1);
    wrong_limit.continuation = first.continuation;
    assert!(matches!(
        reader.list_offers(wrong_limit),
        Err(RewardLiveError::StaleContinuation)
    ));

    let first = reader
        .list_offers(offer_query(snapshot.clone(), 2))
        .expect("fresh first page");
    let mut wrong_scope = offer_query(snapshot, 2);
    wrong_scope.scope = RewardVisibilityScope::Owner;
    wrong_scope.continuation = first.continuation;
    assert!(matches!(
        reader.list_offers(wrong_scope),
        Err(RewardLiveError::StaleContinuation)
    ));
    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn page_size_bounds_reject_empty_and_oversized_requests() {
    let catalog = test_catalog();
    let (mut reader, counters) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let snapshot = reader.capture().expect("capture");
    assert!(matches!(
        reader.list_offers(offer_query(snapshot.clone(), 0)),
        Err(RewardLiveError::InvalidPageSize)
    ));
    assert!(matches!(
        reader.list_offers(offer_query(
            snapshot,
            sts2_game_mod::REWARD_LIVE_MAX_PAGE_ITEMS + 1
        )),
        Err(RewardLiveError::InvalidPageSize)
    ));
    assert_eq!(counters.reads.load(Ordering::SeqCst), 1);
}

#[test]
fn catalog_generation_replacement_invalidates_existing_live_references() {
    let catalog = test_catalog();
    let (mut reader, _) = reader_for(&catalog, [Ok(base_snapshot(&catalog))]);
    let snapshot = reader.capture().expect("capture");
    let offer = reader
        .list_offers(offer_query(snapshot, 64))
        .expect("list")
        .entries[0]
        .reference
        .clone();
    reader
        .replace_catalog(catalog.reader(), RewardLiveInvalidation::CatalogChanged)
        .expect("catalog replacement");
    assert!(matches!(
        reader.get_offer(&offer, RewardVisibilityScope::Public),
        Err(RewardLiveError::StaleReference)
    ));
    assert_eq!(
        reader.last_invalidation(),
        Some(RewardLiveInvalidation::CatalogChanged)
    );
}
