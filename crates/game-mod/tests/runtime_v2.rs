// SPDX-License-Identifier: MIT

use std::error::Error;

use sts2_game_mod::{
    FakeRuntimeV2Game, RuntimeV2CombatPhase, RuntimeV2Config, RuntimeV2Context, RuntimeV2Error,
    RuntimeV2Identity, RuntimeV2Message, RuntimeV2Mod, RuntimeV2Observation, RuntimeV2Status,
    RuntimeV2ValidationError, verify_runtime_v2_artifact,
};

fn context(correlation_id: &str) -> RuntimeV2Context {
    RuntimeV2Context::new(correlation_id, "instance-1", "session-1", "lease-1", 1)
}

fn observation(phase: RuntimeV2CombatPhase) -> RuntimeV2Observation {
    RuntimeV2Observation {
        combat_phase: phase,
        turn_index: 2,
        host_ready: true,
        generation: 4,
    }
}

fn runtime(
    state: RuntimeV2Observation,
    queue_capacity: usize,
    receipt_capacity: usize,
) -> Result<RuntimeV2Mod<FakeRuntimeV2Game>, Box<dyn Error>> {
    let config = RuntimeV2Config {
        identity: RuntimeV2Identity::new("instance-1", "session-1", "lease-1", 1),
        queue_capacity,
        receipt_capacity,
        ..RuntimeV2Config::default()
    };
    Ok(RuntimeV2Mod::new(FakeRuntimeV2Game::new(state)?, config)?)
}

fn golden(name: &str) -> &'static [u8] {
    let bytes: &'static [u8] = match name {
        "state-request" => {
            include_bytes!("../../../protocol-artifact/runtime-v2/golden/state-request.json")
        }
        "state-response" => {
            include_bytes!("../../../protocol-artifact/runtime-v2/golden/state-response.json")
        }
        "legal-action-accepted" => {
            include_bytes!(
                "../../../protocol-artifact/runtime-v2/golden/legal-action-accepted.json"
            )
        }
        "legal-action-settled" => {
            include_bytes!("../../../protocol-artifact/runtime-v2/golden/legal-action-settled.json")
        }
        "timeout-unknown-response" => {
            include_bytes!(
                "../../../protocol-artifact/runtime-v2/golden/timeout-unknown-response.json"
            )
        }
        "reconcile-settled-response" => {
            include_bytes!(
                "../../../protocol-artifact/runtime-v2/golden/reconcile-settled-response.json"
            )
        }
        "cancelled-before-dispatch" => {
            include_bytes!(
                "../../../protocol-artifact/runtime-v2/golden/cancelled-before-dispatch.json"
            )
        }
        _ => &[],
    };
    if bytes.last().copied() == Some(b'\n') {
        &bytes[..bytes.len() - 1]
    } else {
        bytes
    }
}

fn message(bytes: &[u8]) -> Result<RuntimeV2Message, Box<dyn Error>> {
    let message: RuntimeV2Message = serde_json::from_slice(bytes)?;
    message.validate()?;
    Ok(message)
}

fn one<T>(mut values: Vec<T>) -> Result<T, Box<dyn Error>> {
    values
        .pop()
        .ok_or_else(|| std::io::Error::other("expected one Runtime-v2 pump result").into())
}

#[test]
fn copied_artifact_and_wire_order_are_verified() -> Result<(), Box<dyn Error>> {
    assert_eq!(verify_runtime_v2_artifact(), Ok(()));
    let request = RuntimeV2Message::state_request(&context("corr-0001"), 4);
    assert_eq!(serde_json::to_vec(&request)?, golden("state-request"));
    assert_eq!(request.validate(), Ok(()));

    let mut runtime = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let response = runtime.handle(&serde_json::to_vec(&request)?)?;
    assert_eq!(response, golden("state-response"));
    assert_eq!(
        message(&response)?.kind,
        sts2_game_mod::RuntimeV2Kind::StateResponse
    );
    Ok(())
}

#[test]
fn valid_end_turn_is_admitted_settled_once_and_replayed() -> Result<(), Box<dyn Error>> {
    let mut runtime = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let request = RuntimeV2Message::action_request(&context("corr-0002"), 4, "op-1");
    let accepted = runtime.admit_action(request.clone())?;
    assert_eq!(
        serde_json::to_vec(&accepted)?,
        golden("legal-action-accepted")
    );
    assert_eq!(accepted.status, Some(RuntimeV2Status::Accepted));
    assert_eq!(
        accepted.observation,
        Some(observation(RuntimeV2CombatPhase::PlayerTurn))
    );
    assert_eq!(runtime.queue_len(), 1);
    assert_eq!(runtime.receipt_len(), 1);

    assert_eq!(runtime.admit_action(request.clone())?, accepted);
    let settled = one(runtime.pump(1)?)?;
    assert_eq!(
        serde_json::to_vec(&settled)?,
        golden("legal-action-settled")
    );
    assert_eq!(settled.status, Some(RuntimeV2Status::Settled));
    assert_eq!(
        settled.effect_witness.as_ref().map(|w| w.kind.as_str()),
        Some("turn_end_settled")
    );
    assert_eq!(
        settled.effect_witness.as_ref().map(|w| w.generation),
        Some(5)
    );
    assert_eq!(settled.observation.map(|value| value.turn_index), Some(3));
    assert_eq!(runtime.admit_action(request)?, settled);
    assert_eq!(runtime.into_game().end_turn_calls(), 1);
    Ok(())
}

#[test]
fn stale_identity_generation_and_illegal_phases_reject_before_mutation()
-> Result<(), Box<dyn Error>> {
    let cases = [
        (
            RuntimeV2CombatPhase::OutsideCombat,
            "sts2.game-core/outside_combat",
        ),
        (
            RuntimeV2CombatPhase::EnemyTurn,
            "sts2.game-core/not_player_turn",
        ),
    ];
    for (phase, error_code) in cases {
        let mut runtime = runtime(observation(phase), 2, 4)?;
        let response = runtime.admit_action(RuntimeV2Message::action_request(
            &context("corr-illegal"),
            4,
            "op-illegal",
        ))?;
        assert_eq!(response.status, Some(RuntimeV2Status::Rejected));
        assert_eq!(response.error_code.as_deref(), Some(error_code));
        assert_eq!(runtime.queue_len(), 0);
        assert_eq!(runtime.into_game().end_turn_calls(), 0);
    }

    let mut stale = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let response = stale.admit_action(RuntimeV2Message::action_request(
        &context("corr-stale"),
        3,
        "op-stale",
    ))?;
    assert_eq!(
        response.error_code.as_deref(),
        Some("sts2.game-core/stale_generation")
    );
    assert_eq!(stale.into_game().end_turn_calls(), 0);

    let mut identity = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let wrong_context = RuntimeV2Context::new("corr-identity", "other", "session-1", "lease-1", 1);
    let response = identity.admit_action(RuntimeV2Message::action_request(
        &wrong_context,
        4,
        "op-identity",
    ))?;
    assert_eq!(response.error_code.as_deref(), Some("stale_identity"));
    assert_eq!(identity.into_game().end_turn_calls(), 0);

    let mut stale_lease = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let wrong_lease = RuntimeV2Context::new("corr-lease", "instance-1", "session-1", "lease-1", 2);
    let response = stale_lease.admit_action(RuntimeV2Message::action_request(
        &wrong_lease,
        4,
        "op-lease",
    ))?;
    assert_eq!(response.error_code.as_deref(), Some("stale_identity"));
    assert_eq!(stale_lease.into_game().end_turn_calls(), 0);
    Ok(())
}

#[test]
fn duplicate_conflict_and_queue_or_store_bounds_fail_closed() -> Result<(), Box<dyn Error>> {
    let mut queued = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 1, 4)?;
    let first = RuntimeV2Message::action_request(&context("corr-1"), 4, "op-1");
    assert_eq!(
        queued.admit_action(first.clone())?.status,
        Some(RuntimeV2Status::Accepted)
    );
    let second = RuntimeV2Message::action_request(&context("corr-2"), 4, "op-2");
    let full = queued.admit_action(second)?;
    assert_eq!(full.error_code.as_deref(), Some("sts2.runtime/queue_full"));
    assert_eq!(queued.receipt_len(), 1);
    let conflict = queued.admit_action(RuntimeV2Message::action_request(
        &context("corr-conflict"),
        4,
        "op-1",
    ))?;
    assert_eq!(conflict.error_code.as_deref(), Some("idempotency_conflict"));
    assert_eq!(queued.queue_len(), 1);

    let mut stored = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 4, 1)?;
    assert_eq!(
        stored.admit_action(first)?.status,
        Some(RuntimeV2Status::Accepted)
    );
    let full_store = stored.admit_action(RuntimeV2Message::action_request(
        &context("corr-store"),
        4,
        "op-store",
    ))?;
    assert_eq!(
        full_store.error_code.as_deref(),
        Some("sts2.runtime/receipt_store_full")
    );
    assert_eq!(stored.queue_len(), 1);
    Ok(())
}

#[test]
fn post_write_disconnect_is_unknown_then_same_operation_reconciles() -> Result<(), Box<dyn Error>> {
    let mut runtime = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let request = RuntimeV2Message::action_request(&context("corr-0010"), 4, "op-timeout");
    assert_eq!(
        runtime.admit_action(request.clone())?.status,
        Some(RuntimeV2Status::Accepted)
    );
    let unknown = runtime.simulate_post_write_disconnect("op-timeout")?;
    assert_eq!(
        serde_json::to_vec(&unknown)?,
        golden("timeout-unknown-response")
    );
    assert_eq!(unknown.status, Some(RuntimeV2Status::Unknown));
    assert_eq!(runtime.snapshot()?.generation, 5);
    assert_eq!(runtime.admit_action(request)?, unknown);

    let reconcile = RuntimeV2Message::reconcile_request(&context("corr-0011"), 4, "op-timeout");
    let response = message(&runtime.handle(&serde_json::to_vec(&reconcile)?)?)?;
    assert_eq!(
        serde_json::to_vec(&response)?,
        golden("reconcile-settled-response")
    );
    assert_eq!(response.status, Some(RuntimeV2Status::Settled));
    assert_eq!(response.operation_id.as_deref(), Some("op-timeout"));
    assert_eq!(runtime.into_game().end_turn_calls(), 1);
    Ok(())
}

#[test]
fn timeout_removes_work_and_prevents_late_execution_without_outcome() -> Result<(), Box<dyn Error>>
{
    let mut runtime = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let request = RuntimeV2Message::action_request(&context("corr-0010"), 4, "op-timeout");
    assert_eq!(
        runtime.admit_action(request.clone())?.status,
        Some(RuntimeV2Status::Accepted)
    );
    let unknown = runtime.simulate_timeout(&request)?;
    assert_eq!(
        serde_json::to_vec(&unknown)?,
        golden("timeout-unknown-response")
    );
    assert_eq!(runtime.queue_len(), 0);
    assert!(runtime.pump(1)?.is_empty());
    let reconcile = RuntimeV2Message::reconcile_request(&context("corr-0011"), 4, "op-timeout");
    let response = message(&runtime.handle(&serde_json::to_vec(&reconcile)?)?)?;
    assert_eq!(response.status, Some(RuntimeV2Status::Unknown));
    assert_eq!(runtime.into_game().end_turn_calls(), 0);
    Ok(())
}

#[test]
fn cancellation_is_only_before_admission_and_cannot_undo_admitted_work()
-> Result<(), Box<dyn Error>> {
    let request = RuntimeV2Message::action_request(&context("corr-0009"), 4, "op-cancel");
    let mut cancelled = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let response = cancelled.cancel_before_admission(request.clone())?;
    assert_eq!(
        serde_json::to_vec(&response)?,
        golden("cancelled-before-dispatch")
    );
    assert_eq!(response.status, Some(RuntimeV2Status::Cancelled));
    assert_eq!(cancelled.admit_action(request.clone())?, response);
    assert_eq!(cancelled.queue_len(), 0);
    assert_eq!(cancelled.into_game().end_turn_calls(), 0);

    let mut admitted = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    assert_eq!(
        admitted.admit_action(request.clone())?.status,
        Some(RuntimeV2Status::Accepted)
    );
    assert_eq!(
        admitted.cancel_before_admission(request),
        Err(RuntimeV2Error::CancellationAfterAdmission)
    );
    assert_eq!(
        one(admitted.pump(1)?)?.status,
        Some(RuntimeV2Status::Settled)
    );
    assert_eq!(admitted.into_game().end_turn_calls(), 1);
    Ok(())
}

#[test]
fn observation_and_wire_validation_remain_bounded() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        RuntimeV2Observation {
            turn_index: 1025,
            ..observation(RuntimeV2CombatPhase::PlayerTurn)
        }
        .validate(),
        Err(RuntimeV2ValidationError::TurnIndexBounds)
    );
    let mut message = RuntimeV2Message::state_request(&context("corr-extra"), 4);
    message.schema_digest = "wrong".to_owned();
    assert_eq!(message.validate(), Err(RuntimeV2ValidationError::Metadata));
    let mut runtime = runtime(observation(RuntimeV2CombatPhase::PlayerTurn), 2, 4)?;
    let oversized = vec![b' '; 4097];
    assert_eq!(
        runtime.handle(&oversized),
        Err(RuntimeV2Error::RequestTooLarge)
    );
    Ok(())
}
