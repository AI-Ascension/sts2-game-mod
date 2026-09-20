// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use manifest_fixture::fixture_manifest;
use sts2_game_mod::{
    CoopCatalogProducer, CoopError, CoopLiveFence, CoopPeerListQuery, CoopReadAuthority,
    CoopReadScope,
};
use support::*;

#[test]
fn producing_reads_the_source_once_and_every_later_read_is_local() {
    let manifest = fixture_manifest();
    let source = fixture_source();
    let catalog = CoopCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");
    assert_eq!(source.reads.get(), 1);
    let reference = peer_reference(&catalog, "peer.local");
    for _ in 0..5 {
        catalog.peer(&reference, CoopReadScope::Peer).expect("peer");
        catalog.party().expect("party");
        catalog
            .list_peers(&CoopPeerListQuery {
                locale: catalog.locale().to_owned(),
                party: party_reference(&catalog),
                scope: CoopReadScope::Peer,
                limit: 8,
                continuation: None,
                live_fence: None,
            })
            .expect("page");
    }
    assert_eq!(
        source.reads.get(),
        1,
        "a retained catalog must not re-read the source"
    );
}

#[test]
fn a_party_read_states_the_capability_it_withholds() {
    let (_manifest, catalog) = fixture_catalog();
    assert_eq!(catalog.authority(), CoopReadAuthority::NotGranted);
    let reader = catalog.reader();
    assert_eq!(reader.authority(), CoopReadAuthority::NotGranted);
    let view = catalog.party().expect("party").viewed(CoopReadScope::Party);
    assert_eq!(view.authority, CoopReadAuthority::NotGranted);
    assert_eq!(catalog.party().expect("party").party.run_id, "run.alpha.1");
}

#[test]
fn a_reference_produced_for_another_binding_is_refused_rather_than_resolved() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reference = peer_reference(&catalog, "peer.local");
    reference.catalog.locale = "fr-FR".to_owned();
    assert!(matches!(
        catalog.peer(&reference, CoopReadScope::Peer),
        Err(CoopError::StaleReference)
    ));
}

#[test]
fn a_party_of_another_identity_is_not_answered_with_this_one() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reference = peer_reference(&catalog, "peer.local");
    reference.party_id = "party.beta".to_owned();
    assert!(matches!(
        catalog.peer(&reference, CoopReadScope::Peer),
        Err(CoopError::NotFound)
    ));
    let mut party = party_reference(&catalog);
    party.party_id = "party.beta".to_owned();
    assert!(matches!(
        catalog.list_peers(&CoopPeerListQuery {
            locale: catalog.locale().to_owned(),
            party,
            scope: CoopReadScope::Peer,
            limit: 8,
            continuation: None,
            live_fence: None,
        }),
        Err(CoopError::NotFound)
    ));
}

#[test]
fn a_scope_that_may_not_observe_a_member_is_refused_rather_than_answered() {
    let (_manifest, catalog) = fixture_catalog();
    let reference = peer_reference(&catalog, "peer.local");
    assert!(matches!(
        catalog.peer(&reference, CoopReadScope::Party),
        Err(CoopError::ExcludedByScope)
    ));
    assert!(
        catalog.peer(&reference, CoopReadScope::Peer).is_ok(),
        "the local member's own view is observable"
    );
}

#[test]
fn a_live_read_requires_a_fence_naming_this_instance_and_party() {
    let (_manifest, catalog) = fixture_catalog();
    let scope = CoopReadScope::Party;
    assert!(matches!(
        catalog.current(
            &CoopLiveFence {
                instance_id: String::new(),
                party_id: "party.alpha".to_owned(),
                epoch: 1,
            },
            scope,
        ),
        Err(CoopError::MissingLiveFence)
    ));
    assert!(matches!(
        catalog.current(
            &CoopLiveFence {
                instance_id: "../escape".to_owned(),
                party_id: "party.alpha".to_owned(),
                epoch: 1,
            },
            scope,
        ),
        Err(CoopError::NonOpaqueIdentity("live_fence"))
    ));
    assert!(matches!(
        catalog.current(
            &CoopLiveFence {
                instance_id: "instance.alpha".to_owned(),
                party_id: "party.beta".to_owned(),
                epoch: 1,
            },
            scope,
        ),
        Err(CoopError::StaleLiveFence)
    ));
    assert!(
        catalog
            .current(
                &CoopLiveFence {
                    instance_id: "instance.alpha".to_owned(),
                    party_id: "party.alpha".to_owned(),
                    epoch: fixture_epoch(),
                },
                scope,
            )
            .is_ok()
    );
}

#[test]
fn a_live_fence_from_another_epoch_is_refused_rather_than_answered() {
    let (_manifest, catalog) = fixture_catalog();
    let error = catalog
        .current(
            &CoopLiveFence {
                instance_id: "instance.alpha".to_owned(),
                party_id: "party.alpha".to_owned(),
                epoch: fixture_epoch() + 1,
            },
            CoopReadScope::Party,
        )
        .expect_err("another episode's fence");
    assert_eq!(
        error,
        CoopError::EpochMismatch {
            expected: fixture_epoch() + 1,
            actual: fixture_epoch(),
        },
        "an episode's record must not satisfy another episode's fence"
    );
    assert!(
        catalog
            .current(
                &CoopLiveFence {
                    instance_id: "instance.alpha".to_owned(),
                    party_id: "party.alpha".to_owned(),
                    epoch: 0,
                },
                CoopReadScope::Party,
            )
            .is_err(),
        "an unstated epoch is not this party's epoch either"
    );
}

#[test]
fn a_retained_list_holds_no_live_fence_and_refuses_one_that_is_supplied() {
    let (_manifest, catalog) = fixture_catalog();
    assert!(matches!(
        catalog.list_peers(&CoopPeerListQuery {
            locale: catalog.locale().to_owned(),
            party: party_reference(&catalog),
            scope: CoopReadScope::Party,
            limit: 8,
            continuation: None,
            live_fence: Some(CoopLiveFence {
                instance_id: "instance.alpha".to_owned(),
                party_id: "party.alpha".to_owned(),
                epoch: 1,
            }),
        }),
        Err(CoopError::UnexpectedLiveFence)
    ));
}

#[test]
fn a_page_size_outside_its_bound_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    for limit in [0, sts2_game_mod::COOP_MAX_PAGE_ITEMS + 1] {
        assert!(matches!(
            catalog.list_peers(&CoopPeerListQuery {
                locale: catalog.locale().to_owned(),
                party: party_reference(&catalog),
                scope: CoopReadScope::Party,
                limit,
                continuation: None,
                live_fence: None,
            }),
            Err(CoopError::InvalidPageSize)
        ));
    }
}

#[test]
fn a_list_locale_that_is_not_this_catalog_locale_is_refused() {
    let (_manifest, catalog) = fixture_catalog();
    assert!(matches!(
        catalog.list_peers(&CoopPeerListQuery {
            locale: "fr-FR".to_owned(),
            party: party_reference(&catalog),
            scope: CoopReadScope::Party,
            limit: 8,
            continuation: None,
            live_fence: None,
        }),
        Err(CoopError::LocaleMismatch)
    ));
}

#[test]
fn a_continuation_is_single_use_and_bound_to_its_own_query() {
    let (_manifest, catalog) = fixture_catalog();
    let mut reader = catalog.reader();
    let query = CoopPeerListQuery {
        locale: catalog.locale().to_owned(),
        party: party_reference(&catalog),
        scope: CoopReadScope::Peer,
        limit: 1,
        continuation: None,
        live_fence: None,
    };
    let first = reader.list_peers(&query).expect("first page");
    assert_eq!(first.total, 2);
    assert_eq!(first.entries.len(), 1);
    assert!(!first.complete);
    let continuation = first.continuation.expect("continuation");
    let reused = CoopPeerListQuery {
        locale: catalog.locale().to_owned(),
        party: party_reference(&catalog),
        scope: CoopReadScope::Peer,
        limit: 1,
        continuation: Some(continuation.clone()),
        live_fence: None,
    };
    let second = reader.list_peers(&reused).expect("second page");
    assert_eq!(second.entries.len(), 1);
    assert!(second.complete);
    let again = CoopPeerListQuery {
        continuation: Some(continuation),
        ..reused
    };
    assert!(matches!(
        reader.list_peers(&again),
        Err(CoopError::InvalidContinuation)
    ));
}

#[test]
fn a_continuation_from_one_reader_is_refused_by_another() {
    let (_manifest, catalog) = fixture_catalog();
    let mut first_reader = catalog.reader();
    let mut second_reader = catalog.reader();
    let first = first_reader
        .list_peers(&CoopPeerListQuery {
            locale: catalog.locale().to_owned(),
            party: party_reference(&catalog),
            scope: CoopReadScope::Peer,
            limit: 1,
            continuation: None,
            live_fence: None,
        })
        .expect("first page");
    let continuation = first.continuation.expect("continuation");
    assert!(matches!(
        second_reader.list_peers(&CoopPeerListQuery {
            locale: catalog.locale().to_owned(),
            party: party_reference(&catalog),
            scope: CoopReadScope::Peer,
            limit: 1,
            continuation: Some(continuation),
            live_fence: None,
        }),
        Err(CoopError::InvalidContinuation)
    ));
}

#[test]
fn the_local_view_is_the_only_projection_that_carries_local_only_fields() {
    let (_manifest, catalog) = fixture_catalog();
    let view = catalog.reader().local_view().expect("local view");
    assert_eq!(view.authority, CoopReadAuthority::NotGranted);
    assert_eq!(view.local.role, sts2_game_mod::CoopPeerRole::Local);
    assert!(view.local.potions.is_present());
    assert_eq!(view.allies.len(), 1);
    for ally in &view.allies {
        assert!(ally.potions.is_not_permitted());
    }
    assert_eq!(view.reference.party_id, "party.alpha");
}
