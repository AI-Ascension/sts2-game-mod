// SPDX-License-Identifier: MIT

//! Focused producer-boundary tests for the `continue_run` offer gate
//! (`sts2-game-mod#172`, T2).
//!
//! These are deterministic component tests for the owner-local decision that appends a
//! continuation to a new-run catalog. They open no save, launch no host, and claim no resumed
//! run; native resume evidence is T3 and stays out of scope.

#![allow(clippy::expect_used)]

use std::error::Error;

use serde_json::json;
use sts2_game_mod::{
    RuntimeV3GameplayAction, RuntimeV3GameplayLegalAction, RuntimeV3GameplayResumableRun,
    RuntimeV3GameplayResumeOfferError, RuntimeV3GameplayRunCompatibility, RuntimeV3GameplayState,
    offer_continue_run,
};

/// Host-generated identity sequence used for the offered continuation.
const SEQUENCE: u32 = 2;

fn start_run() -> RuntimeV3GameplayLegalAction {
    RuntimeV3GameplayLegalAction {
        action_id: "start_run:1".to_owned(),
        action: RuntimeV3GameplayAction::StartRun {
            character_id: "ironclad".to_owned(),
        },
    }
}

fn start_run_for(sequence: u32, character: &str) -> RuntimeV3GameplayLegalAction {
    RuntimeV3GameplayLegalAction {
        action_id: format!("start_run:{sequence}:{character}"),
        action: RuntimeV3GameplayAction::StartRun {
            character_id: character.to_owned(),
        },
    }
}

fn continuation(run_id: Option<&str>) -> RuntimeV3GameplayLegalAction {
    let run_id = run_id.map(str::to_owned);
    RuntimeV3GameplayLegalAction {
        action_id: match &run_id {
            Some(run_id) => format!("continue_run:{SEQUENCE}:{run_id}"),
            None => format!("continue_run:{SEQUENCE}"),
        },
        action: RuntimeV3GameplayAction::ContinueRun { run_id },
    }
}

fn compatible(run_id: Option<&str>) -> RuntimeV3GameplayResumableRun {
    RuntimeV3GameplayResumableRun::Compatible {
        run_id: run_id.map(str::to_owned),
    }
}

fn setup_state(characters: &[&str]) -> RuntimeV3GameplayState {
    RuntimeV3GameplayState::Setup {
        characters: characters.iter().map(|value| (*value).to_owned()).collect(),
    }
}

#[test]
fn a_compatible_run_is_offered_beside_start_run_in_the_admitted_shapes()
-> Result<(), Box<dyn Error>> {
    let state = setup_state(&["ironclad"]);

    let mut catalog = vec![start_run()];
    assert!(offer_continue_run(
        &state,
        &mut catalog,
        &compatible(Some("profile1")),
        SEQUENCE
    )?);
    assert_eq!(catalog, vec![start_run(), continuation(Some("profile1"))]);
    assert_eq!(catalog[1].action_id, "continue_run:2:profile1");
    assert_eq!(
        serde_json::to_value(&catalog[1].action)?,
        json!({ "kind": "continue_run", "run_id": "profile1" })
    );

    let mut undiscriminated = vec![start_run()];
    assert!(offer_continue_run(
        &state,
        &mut undiscriminated,
        &compatible(None),
        SEQUENCE
    )?);
    assert_eq!(undiscriminated[1].action_id, "continue_run:2");
    assert_eq!(
        serde_json::to_value(&undiscriminated[1].action)?,
        json!({ "kind": "continue_run" })
    );

    // The continuation keeps its own identity and is never aliased onto `start_run`.
    assert_ne!(catalog[1].action_id, catalog[0].action_id);
    assert_ne!(catalog[1].action, catalog[0].action);
    Ok(())
}

#[test]
fn only_a_present_matching_run_is_compatible() -> Result<(), Box<dyn Error>> {
    let state = setup_state(&["ironclad"]);

    assert_eq!(
        RuntimeV3GameplayResumableRun::detect(
            false,
            RuntimeV3GameplayRunCompatibility::Matches,
            Some("profile1".to_owned())
        ),
        RuntimeV3GameplayResumableRun::Absent
    );
    assert_eq!(
        RuntimeV3GameplayResumableRun::detect(
            true,
            RuntimeV3GameplayRunCompatibility::Mismatches,
            Some("profile1".to_owned())
        ),
        RuntimeV3GameplayResumableRun::Incompatible
    );
    let detected = RuntimeV3GameplayResumableRun::detect(
        true,
        RuntimeV3GameplayRunCompatibility::Matches,
        Some("profile1".to_owned()),
    );
    assert_eq!(detected, compatible(Some("profile1")));
    assert_eq!(
        detected.compatible_run_id(),
        Some(&Some("profile1".to_owned()))
    );

    // An absent or incompatible run adds nothing and never errors.
    for unofferable in [
        RuntimeV3GameplayResumableRun::Absent,
        RuntimeV3GameplayResumableRun::Incompatible,
    ] {
        let mut catalog = vec![start_run()];
        assert!(!offer_continue_run(
            &state,
            &mut catalog,
            &unofferable,
            SEQUENCE
        )?);
        assert_eq!(catalog, vec![start_run()]);
    }
    Ok(())
}

#[test]
fn a_multi_character_setup_catalog_still_offers_the_continuation() -> Result<(), Box<dyn Error>> {
    // The real setup screen offers one `start_run` per unlocked character. A gate that required
    // exactly one would refuse every multi-character catalog and silently restore the #172 symptom
    // ("every episode starts over"), so the check is that a `start_run` is present at all.
    let state = setup_state(&["ironclad", "silent"]);
    let mut catalog = vec![start_run_for(2, "ironclad"), start_run_for(2, "silent")];
    assert!(offer_continue_run(
        &state,
        &mut catalog,
        &compatible(None),
        SEQUENCE
    )?);
    assert_eq!(catalog.len(), 3);
    assert_eq!(catalog[2], continuation(None));
    assert_eq!(catalog[2].action_id, "continue_run:2");
    // The continuation is added beside every `start_run` and never replaces one.
    assert_eq!(catalog[0], start_run_for(2, "ironclad"));
    assert_eq!(catalog[1], start_run_for(2, "silent"));

    // A catalog with no `start_run` is not a new-run catalog, so the gate still refuses.
    let mut map_catalog = vec![continuation(None)];
    assert_eq!(
        offer_continue_run(&state, &mut map_catalog, &compatible(None), SEQUENCE).err(),
        Some(RuntimeV3GameplayResumeOfferError::StartRunNotOffered)
    );
    Ok(())
}
