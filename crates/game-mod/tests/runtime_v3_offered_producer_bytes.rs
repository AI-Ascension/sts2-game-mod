// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

//! The producer side of the described offer: what an unchanged host must still emit.
//!
//! `runtime_v3_offered_entry_mirror.rs` already proves the mirror agrees with the schema. This file
//! covers the half a producer can silently break on its own — that widening the contract did not
//! change a single byte for a host that described nothing.
//!
//! Every expectation below is a *recorded* legacy document, captured from base commit `18a8e5c`
//! before any offered entry existed. Asserting against a recorded string rather than a rebuilt
//! expectation is what makes this a compatibility check rather than a tautology: a test that
//! re-derives its own expectation would pass while the bytes changed.

use std::error::Error;

use sts2_game_mod::{
    RuntimeV3GameplayChoice, RuntimeV3GameplayMessage, RuntimeV3GameplayObservation,
    RuntimeV3GameplayPlayer, RuntimeV3GameplayState,
};

const BASE: &str =
    include_str!("../../../protocol-artifact/runtime-v3-gameplay/golden/state-response.json");

/// The exact bytes a host emitted before the observation was widened, recorded from base `18a8e5c`.
const LEGACY_REWARD: &str = r#"{"state_id":"state-1","generation":7,"visible_seed":"seed","player":{"hp":50,"max_hp":50,"energy":3,"gold":99,"hand":[],"deck":[],"discard":[],"exhaust":[]},"state":{"state":"reward","options":["card:21:Setup-Strike","card:22:Tremble"]}}"#;

/// The envelope prefix every legacy state document shares, including the `"state"` wrapper the
/// serialized state itself is nested inside.
const LEGACY_PREFIX: &str = r#"{"state_id":"state-1","generation":7,"visible_seed":"seed","player":{"hp":50,"max_hp":50,"energy":3,"gold":99,"hand":[],"deck":[],"discard":[],"exhaust":[]},"state":"#;

fn player() -> RuntimeV3GameplayPlayer {
    RuntimeV3GameplayPlayer {
        hp: 50,
        max_hp: 50,
        energy: 3,
        gold: 99,
        hand: Vec::new(),
        deck: Vec::new(),
        discard: Vec::new(),
        exhaust: Vec::new(),
    }
}

fn observation(state: RuntimeV3GameplayState) -> RuntimeV3GameplayObservation {
    RuntimeV3GameplayObservation {
        state,
        state_id: "state-1".to_owned(),
        generation: 7,
        visible_seed: Some("seed".to_owned()),
        player: player(),
    }
}

fn reward(options: Vec<RuntimeV3GameplayChoice>) -> RuntimeV3GameplayObservation {
    observation(RuntimeV3GameplayState::Reward { options })
}

fn named(choice_id: &str) -> RuntimeV3GameplayChoice {
    RuntimeV3GameplayChoice::Named(choice_id.to_owned())
}

#[test]
fn legacy_bare_identifiers_keep_their_exact_bytes() -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string(&reward(vec![
        named("card:21:Setup-Strike"),
        named("card:22:Tremble"),
    ]))?;
    assert_eq!(
        json, LEGACY_REWARD,
        "an undescribed host must keep emitting byte-identical JSON"
    );

    let decoded: RuntimeV3GameplayObservation = serde_json::from_str(LEGACY_REWARD)?;
    assert_eq!(
        serde_json::to_string(&decoded)?,
        LEGACY_REWARD,
        "the legacy form must round trip to the same bytes"
    );

    Ok(())
}

#[test]
fn every_widened_state_keeps_its_legacy_bytes() -> Result<(), Box<dyn Error>> {
    for (state, expected) in [
        (
            RuntimeV3GameplayState::Event {
                choices: vec![named("event:Fight")],
            },
            r#"{"state":"event","choices":["event:Fight"]}"#,
        ),
        (
            RuntimeV3GameplayState::Selection {
                choices: vec![named("card:21:A"), named("card:22:B")],
            },
            r#"{"state":"selection","choices":["card:21:A","card:22:B"]}"#,
        ),
    ] {
        let json = serde_json::to_string(&observation(state))?;
        assert_eq!(
            json,
            format!("{LEGACY_PREFIX}{expected}}}"),
            "legacy state bytes changed for {expected}"
        );
        let decoded: RuntimeV3GameplayObservation = serde_json::from_str(&json)?;
        assert_eq!(
            serde_json::to_string(&decoded)?,
            json,
            "the legacy state form must round trip to the same bytes"
        );
    }

    Ok(())
}

#[test]
fn the_unchanged_states_keep_their_bare_identifier_shape() -> Result<(), Box<dyn Error>> {
    // `Map` and `Rest` are deliberately *not* described states: the merged contract keeps both
    // identity-only because no producer discloses them, and widening a member nobody fills is
    // capacity without a producer. These are the two states whose bytes this change must not touch,
    // and they are asserted in full rather than by substring so a sibling key changing shape fails.
    for (state, expected) in [
        (
            RuntimeV3GameplayState::Rest {
                options: vec!["rest.heal".to_owned()],
            },
            r#"{"state":"rest","options":["rest.heal"]}"#,
        ),
        (
            RuntimeV3GameplayState::Map {
                node_id: Some("map.1".to_owned()),
                options: vec!["node:1".to_owned(), "node:2".to_owned()],
            },
            r#"{"state":"map","node_id":"map.1","options":["node:1","node:2"]}"#,
        ),
    ] {
        let json = serde_json::to_string(&observation(state))?;
        assert_eq!(
            json,
            format!("{LEGACY_PREFIX}{expected}}}"),
            "an identity-only state changed shape: {expected}"
        );
    }

    Ok(())
}

#[test]
fn a_described_entry_round_trips_through_the_object_form() -> Result<(), Box<dyn Error>> {
    let entry = sts2_game_mod::RuntimeV3GameplayOfferedEntry {
        choice_id: "card:21:Setup-Strike".to_owned(),
        name: Some("Setup Strike".to_owned()),
        cost: Some(1),
        upgraded: Some(false),
        rarity: None,
        description: None,
        contents: None,
    };
    let json = serde_json::to_string(&reward(vec![RuntimeV3GameplayChoice::Described(
        entry.clone(),
    )]))?;

    // An attribute the host withheld must not appear at all, and a supplied one must round trip.
    assert!(
        !json.contains("\"rarity\""),
        "an absent rarity must be omitted, not defaulted"
    );
    assert!(
        !json.contains("\"description\""),
        "an absent description must be omitted, not defaulted"
    );
    assert!(
        json.contains("\"cost\":1"),
        "the supplied cost must survive"
    );

    let decoded: RuntimeV3GameplayObservation = serde_json::from_str(&json)?;
    assert_eq!(
        serde_json::to_string(&decoded)?,
        json,
        "the described form must round trip to the same bytes"
    );

    let RuntimeV3GameplayState::Reward { options } = &decoded.state else {
        return Err("the decoded state is not a reward".into());
    };
    assert_eq!(
        options[0].described(),
        Some(&entry),
        "the decoded entry must carry exactly what the host published"
    );

    Ok(())
}

#[test]
fn a_withheld_attribute_is_omitted_entirely_and_not_written_as_null() -> Result<(), Box<dyn Error>>
{
    // Only the identity is supplied, so every optional attribute must be absent from the emitted
    // object. This is the assertion that pins `skip_serializing_if` on every attribute: without it
    // serde writes `"name": null`, which a consumer reading "the host said nothing" cannot
    // distinguish from a null the contract refuses, and which the schema rejects outright.
    let json = serde_json::to_string(&reward(vec![RuntimeV3GameplayChoice::Described(
        sts2_game_mod::RuntimeV3GameplayOfferedEntry::named("card:21:Setup-Strike"),
    )]))?;

    for attribute in [
        "name",
        "cost",
        "upgraded",
        "rarity",
        "description",
        "contents",
    ] {
        assert!(
            !json.contains(&format!("\"{attribute}\"")),
            "a withheld `{attribute}` must be omitted entirely, not emitted as null"
        );
    }
    assert!(
        !json.contains("null"),
        "no withheld attribute may appear as an explicit null: {json}"
    );

    Ok(())
}

#[test]
fn an_undisclosed_attribute_must_be_omitted_rather_than_null() -> Result<(), Box<dyn Error>> {
    let mut document: serde_json::Value = serde_json::from_str(BASE)?;
    document["observation"]["state"] = serde_json::json!({
        "state": "reward",
        "options": [{ "choice_id": "card:21:Setup-Strike", "name": null }],
    });
    assert!(
        serde_json::from_value::<RuntimeV3GameplayMessage>(document).is_err(),
        "an explicit null must be refused rather than read as 'the host said nothing'"
    );

    Ok(())
}
