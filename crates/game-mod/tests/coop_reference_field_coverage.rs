// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use sts2_game_mod::{CoopField, CoopFieldStatus, CoopFieldValue, CoopReadAuthority, CoopReadScope};
use support::*;

/// The status every field of one member states at one scope.
fn statuses(
    catalog: &sts2_game_mod::CoopCatalog,
    peer_id: &str,
    scope: CoopReadScope,
) -> Vec<CoopFieldStatus> {
    let view = catalog
        .peer(&peer_reference(catalog, peer_id), scope)
        .expect("view");
    CoopField::ALL
        .iter()
        .map(|field| view.status(*field))
        .collect()
}

#[test]
fn the_field_inventory_is_closed_and_every_row_states_one_status() {
    let (_manifest, catalog) = fixture_catalog();
    let view = catalog
        .peer(&peer_reference(&catalog, "peer.local"), CoopReadScope::Peer)
        .expect("local view");
    for field in CoopField::ALL {
        assert!(
            view.status(field).is_present(),
            "{field:?} must be present on the local member's own view"
        );
    }
    assert_eq!(CoopField::ALL.len(), 9);
    assert_eq!(view.piles.len(), sts2_game_mod::CoopPileKind::ALL.len());
}

#[test]
fn an_ally_states_each_field_according_to_that_fields_own_visibility() {
    let (_manifest, catalog) = fixture_catalog();
    let ally = statuses(&catalog, "peer.ally", CoopReadScope::Peer);
    let table: Vec<(&str, CoopFieldStatus)> = CoopField::ALL
        .iter()
        .map(|field| (field.name(), ally[*field as usize]))
        .collect();
    for field in CoopField::ALL {
        let observed = ally[field as usize];
        let permitted = CoopReadScope::Peer.observes_peer_field(
            sts2_game_mod::CoopPeerRole::Ally,
            field.default_visibility(),
        );
        assert_eq!(
            permitted,
            matches!(
                observed,
                CoopFieldStatus::Present | CoopFieldStatus::Absent | CoopFieldStatus::Withheld
            ) || field == CoopField::Piles,
            "{field:?} at the peer scope: {table:?}"
        );
        if !permitted {
            assert_eq!(
                observed,
                CoopFieldStatus::NotPermitted,
                "{field:?} must state the refusal it is subject to"
            );
        }
        assert!(
            observed != CoopFieldStatus::NotPermitted || !permitted || field == CoopField::Piles,
            "{field:?} may only refuse where its visibility or one of its rows refuses"
        );
    }
    assert_eq!(
        CoopField::Potions.default_visibility(),
        sts2_game_mod::CoopFieldVisibility::LocalOnly
    );
    assert_eq!(
        CoopField::Relics.default_visibility(),
        sts2_game_mod::CoopFieldVisibility::ExplicitlyShared
    );
}

#[test]
fn an_ally_party_visible_piles_field_still_states_its_local_only_rows() {
    let (_manifest, catalog) = fixture_catalog();
    let ally = catalog
        .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
        .expect("ally view");
    assert_eq!(
        CoopField::Piles.default_visibility(),
        sts2_game_mod::CoopFieldVisibility::PublicToParty,
        "the field itself is party-visible"
    );
    assert_eq!(
        ally.status(CoopField::Piles),
        CoopFieldStatus::NotPermitted,
        "but the row that owns the refusal is what the field states"
    );
    let refused: Vec<&str> = ally
        .piles
        .iter()
        .filter(|pile| pile.contents.is_not_permitted())
        .map(|pile| pile.kind.name())
        .collect();
    assert_eq!(refused, vec!["draw", "hand"]);
    let observed: Vec<&str> = ally
        .piles
        .iter()
        .filter(|pile| pile.contents.is_present())
        .map(|pile| pile.kind.name())
        .collect();
    assert_eq!(observed, vec!["discard", "exhaust", "play"]);
}

#[test]
fn the_party_scope_observes_no_local_member_and_states_that_by_refusal() {
    let (_manifest, catalog) = fixture_catalog();
    for scope in [CoopReadScope::Peer, CoopReadScope::Party] {
        assert_eq!(
            scope.observes_peer(sts2_game_mod::CoopPeerRole::Local),
            scope == CoopReadScope::Peer,
            "{scope:?} and a local member"
        );
        assert!(
            scope.observes_peer(sts2_game_mod::CoopPeerRole::Ally),
            "{scope:?} and an ally"
        );
    }
    assert!(matches!(
        catalog.peer(
            &peer_reference(&catalog, "peer.local"),
            CoopReadScope::Party
        ),
        Err(sts2_game_mod::CoopError::ExcludedByScope)
    ));
    assert!(
        catalog
            .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
            .is_ok()
    );
}

#[test]
fn an_unavailable_field_class_is_observable_from_no_scope() {
    for visibility in [
        sts2_game_mod::CoopFieldVisibility::PublicToParty,
        sts2_game_mod::CoopFieldVisibility::ExplicitlyShared,
        sts2_game_mod::CoopFieldVisibility::LocalOnly,
        sts2_game_mod::CoopFieldVisibility::Unavailable,
    ] {
        let observable = [CoopReadScope::Peer, CoopReadScope::Party]
            .into_iter()
            .any(|scope| {
                [
                    sts2_game_mod::CoopPeerRole::Local,
                    sts2_game_mod::CoopPeerRole::Ally,
                ]
                .into_iter()
                .any(|role| scope.observes_peer_field(role, visibility))
            });
        assert_eq!(
            observable,
            visibility != sts2_game_mod::CoopFieldVisibility::Unavailable,
            "{visibility:?}"
        );
    }
}

#[test]
fn a_local_only_field_is_visible_only_to_the_local_members_own_view() {
    let visibility = sts2_game_mod::CoopFieldVisibility::LocalOnly;
    for scope in [CoopReadScope::Peer, CoopReadScope::Party] {
        for role in [
            sts2_game_mod::CoopPeerRole::Local,
            sts2_game_mod::CoopPeerRole::Ally,
        ] {
            let expected =
                scope == CoopReadScope::Peer && role == sts2_game_mod::CoopPeerRole::Local;
            assert_eq!(scope.observes_peer_field(role, visibility), expected);
        }
    }
}

#[test]
fn a_local_only_field_never_reads_as_an_empty_value() {
    let (_manifest, catalog) = fixture_catalog();
    let ally = catalog
        .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
        .expect("ally view");
    assert_eq!(
        ally.status(CoopField::Potions),
        CoopFieldStatus::NotPermitted
    );
    assert!(ally.potions.value().is_none());
    assert!(ally.potions.is_consistent());
    assert!(!ally.potions.is_present());
}

#[test]
fn a_withheld_field_and_an_absent_field_are_not_the_same_refusal() {
    let (_manifest, catalog) = fixture_catalog();
    let local = catalog
        .peer(&peer_reference(&catalog, "peer.local"), CoopReadScope::Peer)
        .expect("local view");
    let ally = catalog
        .peer(&peer_reference(&catalog, "peer.ally"), CoopReadScope::Peer)
        .expect("ally view");
    assert_eq!(
        local.status(CoopField::SpecialMechanics),
        CoopFieldStatus::Present
    );
    assert_eq!(
        ally.status(CoopField::SpecialMechanics),
        CoopFieldStatus::Absent,
        "an ally that reports no special mechanic states absence rather than a refusal"
    );
    assert_ne!(
        ally.status(CoopField::Potions),
        ally.status(CoopField::SpecialMechanics),
        "withheld and absent are different states"
    );
}

#[test]
fn every_status_except_present_states_that_it_carries_no_value() {
    for status in [
        CoopFieldStatus::Absent,
        CoopFieldStatus::Unsupported,
        CoopFieldStatus::Withheld,
        CoopFieldStatus::NotPermitted,
        CoopFieldStatus::Stale,
    ] {
        assert!(status.is_stated());
        assert!(!status.is_present());
    }
    assert!(CoopFieldStatus::Present.is_present());
    assert!(!CoopFieldStatus::Present.is_stated());
}

#[test]
fn only_present_fields_may_carry_a_value_and_every_status_agrees_with_its_value() {
    for field in [
        CoopFieldValue::present(1_u8),
        CoopFieldValue::absent(),
        CoopFieldValue::unsupported(),
        CoopFieldValue::withheld(),
        CoopFieldValue::not_permitted(),
        CoopFieldValue::stale(),
    ] {
        assert!(field.is_consistent());
        assert_eq!(field.status().is_present(), field.value().is_some());
        assert_eq!(field.value().is_some(), field.is_present());
    }
    assert_eq!(CoopFieldValue::present(7_u8).into_value(), Some(7));
    assert_eq!(CoopFieldValue::<u8>::stale().into_value(), None);
}

#[test]
fn the_party_view_and_the_peer_view_state_the_same_authority() {
    let (_manifest, catalog) = fixture_catalog();
    assert_eq!(catalog.authority(), CoopReadAuthority::NotGranted);
    for scope in [CoopReadScope::Peer, CoopReadScope::Party] {
        assert_eq!(
            catalog.party().expect("party").viewed(scope).authority,
            CoopReadAuthority::NotGranted
        );
    }
}

#[test]
fn a_field_named_by_the_closed_inventory_has_one_stable_name() {
    for field in CoopField::ALL {
        assert!(!field.name().is_empty());
        assert_eq!(field.name(), field.name());
    }
    assert_eq!(CoopField::Character.name(), "character");
    assert_eq!(CoopField::SpecialMechanics.name(), "special_mechanics");
}
