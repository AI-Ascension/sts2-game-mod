// SPDX-License-Identifier: MIT

use std::error::Error;

use sts2_game_mod::{
    FakeRuntimeV3GameplayGame, RuntimeV3GameplayAction, RuntimeV3GameplayCard,
    RuntimeV3GameplayConfig, RuntimeV3GameplayContext, RuntimeV3GameplayIdentity,
    RuntimeV3GameplayLegalAction, RuntimeV3GameplayMessage, RuntimeV3GameplayMessageKind,
    RuntimeV3GameplayMod, RuntimeV3GameplayObservation, RuntimeV3GameplayPlayer,
    RuntimeV3GameplayState, RuntimeV3GameplayStatus, RuntimeV3GameplayWaitOutcome,
};

fn observation() -> RuntimeV3GameplayObservation {
    RuntimeV3GameplayObservation {
        state_id: "combat-1".to_owned(),
        generation: 0,
        visible_seed: Some("visible-seed-only".to_owned()),
        player: RuntimeV3GameplayPlayer {
            hp: 50,
            max_hp: 50,
            energy: 3,
            gold: 99,
            hand: Vec::<RuntimeV3GameplayCard>::new(),
            deck: Vec::new(),
            discard: Vec::new(),
            exhaust: Vec::new(),
        },
        state: RuntimeV3GameplayState::Combat {
            turn_index: 1,
            enemies: Vec::new(),
        },
    }
}

fn action() -> RuntimeV3GameplayLegalAction {
    RuntimeV3GameplayLegalAction {
        action_id: "combat.end-turn".to_owned(),
        action: RuntimeV3GameplayAction::EndTurn,
    }
}

fn context() -> RuntimeV3GameplayContext {
    RuntimeV3GameplayContext::new("corr-1", "instance-1", "session-1", "lease-1", 1)
}

fn runtime() -> RuntimeV3GameplayMod<FakeRuntimeV3GameplayGame> {
    let game = FakeRuntimeV3GameplayGame::new(observation(), vec![action()])
        .expect("fake observation and catalog are valid");
    RuntimeV3GameplayMod::new(
        game,
        RuntimeV3GameplayConfig {
            identity: RuntimeV3GameplayIdentity::new("instance-1", "session-1", "lease-1", 1),
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
) -> RuntimeV3GameplayMessage {
    let body = serde_json::to_vec(&message).expect("request encodes");
    let response = runtime
        .handle(&body)
        .expect("request is accepted by the boundary");
    serde_json::from_slice(&response).expect("response decodes")
}

#[test]
fn state_and_legal_actions_are_host_generated() -> Result<(), Box<dyn Error>> {
    let mut runtime = runtime();
    let response = round_trip(
        &mut runtime,
        RuntimeV3GameplayMessage::state_request(context(), 0),
    );
    assert_eq!(response.kind, RuntimeV3GameplayMessageKind::StateResponse);
    assert_eq!(response.generation, 0);
    assert_eq!(response.legal_actions.as_ref().map(Vec::len), Some(1));
    response.validate()?;
    Ok(())
}

#[test]
fn accepted_action_requires_main_thread_pump_and_settles_once() -> Result<(), Box<dyn Error>> {
    let mut runtime = runtime();
    let request = RuntimeV3GameplayMessage::dispatch_action_request(
        context(),
        0,
        "combat-1",
        "op-1",
        action(),
    );
    let accepted = round_trip(&mut runtime, request.clone());
    assert_eq!(accepted.status, Some(RuntimeV3GameplayStatus::Accepted));
    assert_eq!(runtime.queue_len(), 1);

    let pumped = runtime.pump(1)?;
    assert_eq!(pumped.len(), 1);
    assert_eq!(pumped[0].status, Some(RuntimeV3GameplayStatus::Settled));
    assert_eq!(pumped[0].generation, 1);
    assert!(pumped[0].transition.is_some());

    let duplicate = round_trip(&mut runtime, request);
    assert_eq!(duplicate.status, Some(RuntimeV3GameplayStatus::Settled));
    assert_eq!(runtime.into_game().dispatch_calls(), 1);
    Ok(())
}

#[test]
fn wait_and_stale_action_are_fail_closed() -> Result<(), Box<dyn Error>> {
    let mut runtime = runtime();
    let request = RuntimeV3GameplayMessage::dispatch_action_request(
        context(),
        0,
        "combat-1",
        "op-1",
        action(),
    );
    let _ = round_trip(&mut runtime, request);
    let _ = runtime.pump(1)?;

    let wait = round_trip(
        &mut runtime,
        RuntimeV3GameplayMessage::wait_request(context(), 1, "op-1", 100),
    );
    assert_eq!(wait.kind, RuntimeV3GameplayMessageKind::WaitResponse);
    assert_eq!(
        wait.wait_outcome,
        Some(RuntimeV3GameplayWaitOutcome::Successor)
    );

    let stale = round_trip(
        &mut runtime,
        RuntimeV3GameplayMessage::dispatch_action_request(
            context(),
            0,
            "combat-1",
            "op-stale",
            action(),
        ),
    );
    assert_eq!(stale.status, Some(RuntimeV3GameplayStatus::Rejected));
    assert_eq!(stale.error_code.as_deref(), Some("stale_generation"));
    Ok(())
}
