// SPDX-License-Identifier: MIT

//! Focused producer-boundary tests for the host-offered `continue_run` action
//! (`sts2-game-mod#172`, T2).
//!
//! These are deterministic component tests. They exercise the Rust producer catalog offer, the
//! existing admission/receipt dispatch lane, and the harness-admitted JSON shapes. No native save
//! is opened, no game host is launched, and no resumed run is claimed; native resume evidence is
//! T3 and remains out of scope.

#![allow(clippy::expect_used)]

use std::error::Error;

use serde_json::{Value, json};
use sts2_game_mod::{
    FakeRuntimeV3GameplayGame, RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS, RuntimeV3GameplayAction,
    RuntimeV3GameplayConfig, RuntimeV3GameplayContext, RuntimeV3GameplayGamePort,
    RuntimeV3GameplayIdentity, RuntimeV3GameplayLegalAction, RuntimeV3GameplayMessage,
    RuntimeV3GameplayMessageKind, RuntimeV3GameplayMod, RuntimeV3GameplayObservation,
    RuntimeV3GameplayPlayer, RuntimeV3GameplayResumableRun, RuntimeV3GameplayResumeOfferError,
    RuntimeV3GameplayRunCompatibility, RuntimeV3GameplayState, RuntimeV3GameplayStatus,
    offer_continue_run,
};

/// Host-generated identity sequence used for the offered continuation.
const SEQUENCE: u32 = 2;

fn setup_observation() -> RuntimeV3GameplayObservation {
    RuntimeV3GameplayObservation {
        state_id: "setup-1".to_owned(),
        generation: 0,
        visible_seed: None,
        player: RuntimeV3GameplayPlayer {
            hp: 75,
            max_hp: 75,
            energy: 0,
            gold: 0,
            hand: Vec::new(),
            deck: Vec::new(),
            discard: Vec::new(),
            exhaust: Vec::new(),
        },
        state: RuntimeV3GameplayState::Setup {
            characters: vec!["ironclad".to_owned()],
        },
    }
}

fn start_run() -> RuntimeV3GameplayLegalAction {
    RuntimeV3GameplayLegalAction {
        action_id: "start_run:1".to_owned(),
        action: RuntimeV3GameplayAction::StartRun {
            character_id: "ironclad".to_owned(),
        },
    }
}

fn continuation(run_id: Option<&str>) -> RuntimeV3GameplayLegalAction {
    let run_id = run_id.map(str::to_owned);
    let action_id = match &run_id {
        Some(run_id) => format!("continue_run:{SEQUENCE}:{run_id}"),
        None => format!("continue_run:{SEQUENCE}"),
    };
    RuntimeV3GameplayLegalAction {
        action_id,
        action: RuntimeV3GameplayAction::ContinueRun { run_id },
    }
}

fn context() -> RuntimeV3GameplayContext {
    RuntimeV3GameplayContext::new("corr-1", "instance-1", "session-1", "lease-1", 1)
}

fn identity() -> RuntimeV3GameplayIdentity {
    RuntimeV3GameplayIdentity::new("instance-1", "session-1", "lease-1", 1)
}

fn runtime_with(
    actions: Vec<RuntimeV3GameplayLegalAction>,
) -> RuntimeV3GameplayMod<FakeRuntimeV3GameplayGame> {
    let game = FakeRuntimeV3GameplayGame::new(setup_observation(), actions)
        .expect("setup observation and catalog are valid");
    RuntimeV3GameplayMod::new(
        game,
        RuntimeV3GameplayConfig {
            identity: identity(),
            queue_capacity: 2,
            receipt_capacity: 8,
            max_request_bytes: 4096,
        },
    )
    .expect("runtime configuration is valid")
}

fn round_trip(
    runtime: &mut RuntimeV3GameplayMod<FakeRuntimeV3GameplayGame>,
    message: RuntimeV3GameplayMessage,
) -> Result<RuntimeV3GameplayMessage, Box<dyn Error>> {
    let body = serde_json::to_vec(&message)?;
    let response = runtime.handle(&body)?;
    Ok(serde_json::from_slice(&response)?)
}

fn compatible(run_id: Option<&str>) -> RuntimeV3GameplayResumableRun {
    RuntimeV3GameplayResumableRun::Compatible {
        run_id: run_id.map(str::to_owned),
    }
}

#[test]
fn a_compatible_run_is_offered_beside_start_run_in_the_admitted_shapes()
-> Result<(), Box<dyn Error>> {
    let state = setup_observation().state;

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
    let state = setup_observation().state;

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
fn an_offered_continuation_dispatches_through_admission_and_settles() -> Result<(), Box<dyn Error>>
{
    let offered = continuation(Some("profile1"));
    let mut runtime = runtime_with(vec![start_run(), offered.clone()]);
    let request = RuntimeV3GameplayMessage::dispatch_action_request(
        context(),
        0,
        "setup-1",
        "op-continue",
        offered.clone(),
    );
    let accepted = round_trip(&mut runtime, request)?;
    assert_eq!(accepted.status, Some(RuntimeV3GameplayStatus::Accepted));
    assert_eq!(
        accepted.kind,
        RuntimeV3GameplayMessageKind::DispatchActionResponse
    );
    assert_eq!(runtime.queue_len(), 1);

    let settled = runtime.pump(1)?;
    assert_eq!(settled.len(), 1);
    assert_eq!(settled[0].status, Some(RuntimeV3GameplayStatus::Settled));
    assert!(settled[0].transition.is_some());

    // The host saw exactly the continuation; the lane never substituted `start_run`.
    let game = runtime.into_game();
    assert_eq!(game.dispatch_calls(), 1);
    let proof = game
        .completion(&identity(), "op-continue")
        .expect("the synthetic host settled the dispatched continuation");
    assert_eq!(proof.action, offered);
    assert!(matches!(
        proof.action.action,
        RuntimeV3GameplayAction::ContinueRun {
            run_id: Some(ref run_id)
        } if run_id == "profile1"
    ));
    Ok(())
}

#[test]
fn malformed_continuations_are_refused_before_the_host() -> Result<(), Box<dyn Error>> {
    // A null discriminator is errored rather than folded into an absent one.
    assert!(
        serde_json::from_str::<RuntimeV3GameplayAction>(r#"{"kind":"continue_run","run_id":null}"#)
            .is_err()
    );
    // An extra field, a save path, or any unknown member is refused by the closed shape.
    for payload in [
        r#"{"kind":"continue_run","run_id":"profile1","seed":"1"}"#,
        r#"{"kind":"continue_run","save_path":"profile1/saves/current_run.save"}"#,
        r#"{"kind":"continue_run","run_id":"profile1","run_id":"profile2"}"#,
    ] {
        assert!(
            serde_json::from_str::<RuntimeV3GameplayAction>(payload).is_err(),
            "{payload}"
        );
    }
    // A blank or non-identity discriminator parses as a string but fails validation.
    // A path separator is *not* used here: `/` is a legal identity byte in this contract.
    for run_id in ["", "profile 1", "profile#1", "profile\n1"] {
        let action = RuntimeV3GameplayLegalAction {
            action_id: "continue_run:2".to_owned(),
            action: RuntimeV3GameplayAction::ContinueRun {
                run_id: Some(run_id.to_owned()),
            },
        };
        assert!(action.validate().is_err(), "{run_id}");
    }
    // An identity longer than the bounded field is refused too.
    let oversized = RuntimeV3GameplayLegalAction {
        action_id: "continue_run:2".to_owned(),
        action: RuntimeV3GameplayAction::ContinueRun {
            run_id: Some("p".repeat(513)),
        },
    };
    assert!(oversized.validate().is_err());

    // A malformed continuation never reaches the host dispatch lane.
    let mut runtime = runtime_with(vec![start_run()]);
    let body = serde_json::to_vec(&json!({
        "protocol_version": "runtime-v3-gameplay",
        "schema_digest": sts2_game_mod::RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST,
        "provenance": {
            "artifact": "sts2-protocol/runtime-v3-gameplay",
            "source": "schemas/runtime-v3-gameplay.schema.json",
            "generator": "hand-authored"
        },
        "correlation_id": "corr-1",
        "instance_id": "instance-1",
        "session_id": "session-1",
        "lease_id": "lease-1",
        "lease_epoch": 1,
        "generation": 0,
        "kind": "dispatch_action_request",
        "state_id": "setup-1",
        "operation_id": "op-malformed",
        "observation": null,
        "legal_actions": null,
        "action": {"action_id": "continue_run:2", "action": {"kind": "continue_run", "run_id": null}},
        "status": null,
        "transition": null,
        "error_code": null,
        "wait_for_millis": null,
        "wait_outcome": null,
        "recovery": null
    }))?;
    assert!(runtime.handle(&body).is_err());
    assert_eq!(runtime.into_game().dispatch_calls(), 0);
    Ok(())
}

#[test]
fn stale_and_unoffered_continuations_are_rejected_by_the_authority_fences()
-> Result<(), Box<dyn Error>> {
    // A generation mismatch is refused before the catalog is even consulted.
    let mut stale = runtime_with(vec![start_run(), continuation(Some("profile1"))]);
    let stale_response = round_trip(
        &mut stale,
        RuntimeV3GameplayMessage::dispatch_action_request(
            context(),
            1,
            "setup-1",
            "op-stale",
            continuation(Some("profile1")),
        ),
    )?;
    assert_eq!(
        stale_response.status,
        Some(RuntimeV3GameplayStatus::Rejected)
    );
    assert_eq!(
        stale_response.error_code.as_deref(),
        Some("stale_generation")
    );
    assert_eq!(stale.into_game().dispatch_calls(), 0);

    // An action the current catalog does not offer is refused as not current.
    let mut unoffered = runtime_with(vec![start_run()]);
    let unoffered_response = round_trip(
        &mut unoffered,
        RuntimeV3GameplayMessage::dispatch_action_request(
            context(),
            0,
            "setup-1",
            "op-unoffered",
            continuation(Some("profile1")),
        ),
    )?;
    assert_eq!(
        unoffered_response.status,
        Some(RuntimeV3GameplayStatus::Rejected)
    );
    assert_eq!(
        unoffered_response.error_code.as_deref(),
        Some("action_not_current")
    );
    assert_eq!(unoffered.into_game().dispatch_calls(), 0);
    Ok(())
}

#[test]
fn the_offer_gate_refuses_everything_but_a_real_offered_continuation() -> Result<(), Box<dyn Error>>
{
    let combat = RuntimeV3GameplayState::Combat {
        turn_index: 1,
        enemies: Vec::new(),
    };
    let mut catalog = vec![start_run()];
    assert_eq!(
        offer_continue_run(&combat, &mut catalog, &compatible(None), SEQUENCE).err(),
        Some(RuntimeV3GameplayResumeOfferError::NotOnSetupScreen)
    );

    let state = setup_observation().state;
    let mut without_start = Vec::new();
    assert_eq!(
        offer_continue_run(&state, &mut without_start, &compatible(None), SEQUENCE).err(),
        Some(RuntimeV3GameplayResumeOfferError::StartRunNotOffered)
    );

    let mut duplicate = vec![start_run(), continuation(None)];
    assert_eq!(
        offer_continue_run(&state, &mut duplicate, &compatible(None), SEQUENCE).err(),
        Some(RuntimeV3GameplayResumeOfferError::DuplicateActionId)
    );

    let mut invalid = vec![start_run()];
    assert_eq!(
        offer_continue_run(
            &state,
            &mut invalid,
            &compatible(Some("profile 1")),
            SEQUENCE
        )
        .err(),
        Some(RuntimeV3GameplayResumeOfferError::InvalidContinuation)
    );

    let mut full = vec![start_run()];
    while full.len() < RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS {
        full.push(RuntimeV3GameplayLegalAction {
            action_id: format!("end_turn:{}", full.len()),
            action: RuntimeV3GameplayAction::EndTurn,
        });
    }
    assert_eq!(
        offer_continue_run(&state, &mut full, &compatible(None), SEQUENCE).err(),
        Some(RuntimeV3GameplayResumeOfferError::CatalogFull)
    );
    Ok(())
}

#[test]
fn both_admitted_shapes_round_trip_as_a_dispatch_request() -> Result<(), Box<dyn Error>> {
    for offered in [continuation(Some("profile1")), continuation(None)] {
        let request = RuntimeV3GameplayMessage::dispatch_action_request(
            context(),
            0,
            "setup-1",
            "op-round-trip",
            offered.clone(),
        );
        request.validate()?;
        let encoded = serde_json::to_vec(&request)?;
        let decoded: RuntimeV3GameplayMessage = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, request);
        let encoded_value: Value = serde_json::from_slice(&encoded)?;
        assert_eq!(
            encoded_value["action"]["action_id"],
            json!(offered.action_id)
        );
    }
    Ok(())
}
