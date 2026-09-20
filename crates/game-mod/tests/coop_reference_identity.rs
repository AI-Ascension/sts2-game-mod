// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/coop_reference_manifest.rs"]
mod manifest_fixture;
#[path = "support/coop_reference.rs"]
mod support;

use sts2_game_mod::{
    COOP_MAX_IDENTITY_BYTES, CoopCatalogBinding, CoopError, CoopPartyReference, CoopPeerReference,
    CoopReadScope, is_opaque_coop_identity,
};
use support::*;

#[test]
fn an_opaque_identity_carries_no_path_and_no_control_byte() {
    for accepted in [
        "party.alpha",
        "peer.local",
        "run.alpha.1",
        "instance.alpha",
        "effect.rage",
        "rule.shared",
        "resource.energy",
        "a#1",
        "A_b-9",
    ] {
        assert!(is_opaque_coop_identity(accepted), "{accepted} is opaque");
    }
    for refused in [
        "",
        "/",
        "\\",
        "..",
        "../party",
        "C:",
        "party alpha",
        "party\talpha",
        "party\nalpha",
        "party/alpha",
        "party\\alpha",
        "parté",
    ] {
        assert!(
            !is_opaque_coop_identity(refused),
            "{refused:?} is not opaque"
        );
    }
}

#[test]
fn an_identity_past_its_byte_bound_is_refused() {
    assert!(is_opaque_coop_identity(
        &"a".repeat(COOP_MAX_IDENTITY_BYTES)
    ));
    assert!(!is_opaque_coop_identity(
        &"a".repeat(COOP_MAX_IDENTITY_BYTES + 1)
    ));
}

#[test]
fn a_reference_carries_the_binding_it_was_produced_for_and_not_another() {
    let (_manifest, catalog) = fixture_catalog();
    let reference = peer_reference(&catalog, "peer.local");
    assert_eq!(reference.catalog, *catalog.binding());
    assert_eq!(reference.party_id, "party.alpha");
    assert_eq!(reference.peer_id, "peer.local");
    let other = CoopCatalogBinding {
        manifest: catalog.binding().manifest.clone(),
        locale: "fr-FR".to_owned(),
        producer_version: catalog.binding().producer_version.clone(),
    };
    assert_ne!(other, *catalog.binding());
    assert_ne!(
        CoopPeerReference {
            catalog: other.clone(),
            party_id: "party.alpha".to_owned(),
            peer_id: "peer.local".to_owned(),
            generation: reference.generation,
        },
        reference
    );
    assert_ne!(
        CoopPartyReference {
            catalog: other,
            party_id: "party.alpha".to_owned(),
        },
        party_reference(&catalog)
    );
}

#[test]
fn a_reference_naming_an_unknown_member_is_refused_rather_than_resolved() {
    let (_manifest, catalog) = fixture_catalog();
    assert!(matches!(
        catalog.peer(
            &peer_reference(&catalog, "peer.absent"),
            CoopReadScope::Peer
        ),
        Err(CoopError::NotFound)
    ));
}

#[test]
fn a_party_identity_is_never_the_instance_or_the_run_it_belongs_to() {
    let (_manifest, catalog) = fixture_catalog();
    let record = catalog.party().expect("party");
    assert_ne!(record.party.party_id, record.party.instance_id);
    assert_ne!(record.party.party_id, record.party.run_id);
    assert_ne!(record.party.instance_id, record.party.run_id);
}

#[test]
fn a_member_reference_and_a_party_reference_disagree_about_what_they_name() {
    let (_manifest, catalog) = fixture_catalog();
    let party = party_reference(&catalog);
    let peer = peer_reference(&catalog, "peer.ally");
    assert_eq!(party.party_id, peer.party_id);
    assert_eq!(party.catalog, peer.catalog);
    assert_ne!(party.party_id, peer.peer_id);
}
