// SPDX-License-Identifier: MIT

use std::error::Error;

use sts2_game_mod::{
    FakeRuntimeV2Game, RUNTIME_V2_MAX_GENERATION, RuntimeV2CombatPhase, RuntimeV2Config,
    RuntimeV2Context, RuntimeV2Error, RuntimeV2GamePort, RuntimeV2Message, RuntimeV2Mod,
    RuntimeV2Observation,
};

fn observation() -> RuntimeV2Observation {
    RuntimeV2Observation {
        combat_phase: RuntimeV2CombatPhase::PlayerTurn,
        turn_index: 1,
        host_ready: true,
        generation: 4,
    }
}

#[test]
fn action_only_methods_reject_read_requests_without_retaining_work() -> Result<(), Box<dyn Error>> {
    let context = RuntimeV2Context::new("correlation", "instance-1", "session-1", "lease-1", 1);
    for request in [
        RuntimeV2Message::state_request(&context, 4),
        RuntimeV2Message::reconcile_request(&context, 4, "operation"),
    ] {
        let mut runtime = RuntimeV2Mod::new(
            FakeRuntimeV2Game::new(observation())?,
            RuntimeV2Config::default(),
        )?;
        assert_eq!(
            runtime.admit_action(request.clone()),
            Err(RuntimeV2Error::MalformedRequest)
        );
        assert_eq!(
            runtime.cancel_before_admission(request.clone()),
            Err(RuntimeV2Error::MalformedRequest)
        );
        assert_eq!(
            runtime.simulate_timeout(&request),
            Err(RuntimeV2Error::MalformedRequest)
        );
        assert_eq!(runtime.queue_len(), 0);
        assert_eq!(runtime.receipt_len(), 0);
        assert!(runtime.pump(8)?.is_empty());
        assert_eq!(runtime.into_game().end_turn_calls(), 0);
    }
    Ok(())
}

#[test]
fn fake_end_turn_rejects_generation_bound_without_mutation() -> Result<(), Box<dyn Error>> {
    let before = RuntimeV2Observation {
        generation: RUNTIME_V2_MAX_GENERATION,
        ..observation()
    };
    let mut game = FakeRuntimeV2Game::new(before)?;
    assert!(game.end_turn().is_err());
    assert_eq!(game.snapshot(), before);
    assert_eq!(game.end_turn_calls(), 0);
    Ok(())
}
