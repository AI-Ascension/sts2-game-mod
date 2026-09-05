// SPDX-License-Identifier: MIT
#![allow(clippy::expect_used)]

use std::{cell::Cell, rc::Rc};
use sts2_game_mod::*;

struct Host {
    fake: FakeRuntimeV3GameplayGame,
    proof: Rc<Cell<u8>>,
    reads_fail: Rc<Cell<bool>>,
}
impl RuntimeV3GameplayGamePort for Host {
    fn snapshot(&self) -> Result<RuntimeV3GameplayObservation, RuntimeV3GameplayGameError> {
        if self.reads_fail.get() {
            return Err(RuntimeV3GameplayGameError::NotReady);
        }
        self.fake.snapshot()
    }
    fn legal_actions(
        &self,
        observation: &RuntimeV3GameplayObservation,
    ) -> Result<Vec<RuntimeV3GameplayLegalAction>, RuntimeV3GameplayGameError> {
        self.fake.legal_actions(observation)
    }
    fn input_enabled(&self) -> bool {
        true
    }
    fn dispatch(
        &mut self,
        identity: &RuntimeV3GameplayIdentity,
        operation_id: &str,
        action: &RuntimeV3GameplayLegalAction,
    ) -> Result<(), RuntimeV3GameplayGameError> {
        if self.proof.get() == 7 {
            return Err(RuntimeV3GameplayGameError::MutationUncertain);
        }
        self.fake.dispatch(identity, operation_id, action)
    }
    fn completion(
        &self,
        identity: &RuntimeV3GameplayIdentity,
        operation_id: &str,
    ) -> Option<RuntimeV3GameplayCompletion> {
        let mut completion = self.fake.completion(identity, operation_id)?;
        match self.proof.get() {
            0 => return None,
            2 => completion.operation_id = "foreign-operation".to_owned(),
            3 => completion.action.action_id = "other-action".to_owned(),
            4 => completion.transition.from_generation = 999,
            5 => completion.transition.to_generation = 999,
            6 => completion.identity.lease_epoch += 1,
            _ => (),
        }
        Some(completion)
    }
}

fn context(correlation: &str) -> RuntimeV3GameplayContext {
    RuntimeV3GameplayContext::new(correlation, "instance-1", "session-1", "lease-1", 1)
}
fn action() -> RuntimeV3GameplayLegalAction {
    RuntimeV3GameplayLegalAction {
        action_id: "end-turn".to_owned(),
        action: RuntimeV3GameplayAction::EndTurn,
    }
}
fn runtime(proof: Rc<Cell<u8>>, reads_fail: Rc<Cell<bool>>) -> RuntimeV3GameplayMod<Host> {
    let observation = RuntimeV3GameplayObservation {
        state_id: "combat-1".to_owned(),
        generation: 0,
        visible_seed: None,
        player: RuntimeV3GameplayPlayer {
            hp: 10,
            max_hp: 10,
            energy: 3,
            gold: 0,
            hand: vec![],
            deck: vec![],
            discard: vec![],
            exhaust: vec![],
        },
        state: RuntimeV3GameplayState::Combat {
            turn_index: 1,
            enemies: vec![],
        },
    };
    RuntimeV3GameplayMod::new(
        Host {
            fake: FakeRuntimeV3GameplayGame::new(observation, vec![action()]).expect("valid fake"),
            proof,
            reads_fail,
        },
        RuntimeV3GameplayConfig::default(),
    )
    .expect("valid config")
}
fn request(correlation: &str) -> RuntimeV3GameplayMessage {
    RuntimeV3GameplayMessage::dispatch_action_request(
        context(correlation),
        0,
        "combat-1",
        "op-1",
        action(),
    )
}
fn send(
    runtime: &mut RuntimeV3GameplayMod<Host>,
    request: RuntimeV3GameplayMessage,
) -> RuntimeV3GameplayMessage {
    let bytes = runtime
        .handle(&serde_json::to_vec(&request).expect("encode"))
        .expect("handle");
    let response: RuntimeV3GameplayMessage = serde_json::from_slice(&bytes).expect("decode");
    response.validate().expect("valid response");
    assert_eq!(response.correlation_id, request.correlation_id);
    response
}

#[test]
fn unrelated_or_mismatched_completion_never_settles_and_late_proof_reconciles() {
    for mode in [0, 2, 3, 4, 5, 6] {
        let proof = Rc::new(Cell::new(mode));
        let mut runtime = runtime(proof.clone(), Rc::new(Cell::new(false)));
        assert_eq!(
            send(&mut runtime, request("dispatch")).status,
            Some(RuntimeV3GameplayStatus::Accepted)
        );
        assert_eq!(
            runtime.pump(1).expect("pump")[0].status,
            Some(RuntimeV3GameplayStatus::Unknown)
        );
        assert_eq!(
            send(
                &mut runtime,
                RuntimeV3GameplayMessage::wait_request(context("pending"), 1, "op-1", 1)
            )
            .status,
            Some(RuntimeV3GameplayStatus::Unknown)
        );
        proof.set(1);
        let settled = send(
            &mut runtime,
            RuntimeV3GameplayMessage::wait_request(context("late"), 1, "op-1", 1),
        );
        assert_eq!(settled.status, Some(RuntimeV3GameplayStatus::Settled));
        assert_eq!(
            settled.wait_outcome,
            Some(RuntimeV3GameplayWaitOutcome::SameStateMutation)
        );
        assert_eq!(runtime.into_game().fake.dispatch_calls(), 1);
    }
}

#[test]
fn retry_changes_transport_correlation_not_operation_and_does_not_read_host() {
    let reads_fail = Rc::new(Cell::new(false));
    let mut runtime = runtime(Rc::new(Cell::new(1)), reads_fail.clone());
    send(&mut runtime, request("first"));
    send(&mut runtime, request("queued-retry"));
    assert_eq!(runtime.queue_len(), 1);
    runtime.pump(1).expect("pump");
    reads_fail.set(true);
    assert_eq!(
        send(&mut runtime, request("settled-retry")).status,
        Some(RuntimeV3GameplayStatus::Settled)
    );
    let mut conflicting = request("conflict");
    conflicting.generation = 1;
    assert_eq!(
        send(&mut runtime, conflicting).error_code.as_deref(),
        Some("idempotency_conflict")
    );
    let mut foreign = request("foreign");
    foreign.session_id = "other-session".to_owned();
    assert!(
        runtime
            .handle(&serde_json::to_vec(&foreign).expect("encode"))
            .is_err()
    );
    assert_eq!(runtime.into_game().fake.dispatch_calls(), 1);
}

#[test]
fn reconcile_replies_to_current_request_correlation() {
    let mut runtime = runtime(Rc::new(Cell::new(1)), Rc::new(Cell::new(false)));
    send(&mut runtime, request("dispatch"));
    runtime.pump(1).expect("pump");
    let mut request = RuntimeV3GameplayMessage::base(
        context("reconcile"),
        1,
        RuntimeV3GameplayMessageKind::RecoverRequest,
    );
    request.recovery = Some(RuntimeV3GameplayRecovery {
        kind: RuntimeV3GameplayRecoveryKind::Reconcile,
        operation_id: Some("op-1".to_owned()),
    });
    assert_eq!(
        send(&mut runtime, request).status,
        Some(RuntimeV3GameplayStatus::Settled)
    );
}

#[test]
fn unchanged_snapshot_does_not_convert_uncertain_dispatch_into_rejection() {
    let mut runtime = runtime(Rc::new(Cell::new(7)), Rc::new(Cell::new(false)));
    send(&mut runtime, request("dispatch"));
    assert_eq!(
        runtime.pump(1).expect("pump")[0].status,
        Some(RuntimeV3GameplayStatus::Unknown)
    );
}

#[test]
fn exhausted_generation_cannot_mutate_fake_or_create_completion() {
    let original = runtime(Rc::new(Cell::new(1)), Rc::new(Cell::new(false)))
        .into_game()
        .fake
        .snapshot()
        .expect("snapshot");
    let mut observation = original;
    observation.generation = RUNTIME_V3_GAMEPLAY_MAX_GENERATION;
    let mut fake = FakeRuntimeV3GameplayGame::new(observation.clone(), vec![action()])
        .expect("valid max generation");
    let identity = RuntimeV3GameplayIdentity::new("instance-1", "session-1", "lease-1", 1);
    assert_eq!(
        fake.advance_generation(),
        Err(RuntimeV3GameplayGameError::ProjectionInvalid)
    );
    assert_eq!(fake.snapshot().expect("unchanged snapshot"), observation);
    assert_eq!(
        fake.dispatch(&identity, "max-operation", &action()),
        Err(RuntimeV3GameplayGameError::ProjectionInvalid)
    );
    assert_eq!(fake.snapshot().expect("unchanged snapshot"), observation);
    assert_eq!(fake.dispatch_calls(), 0);
    assert!(fake.completion(&identity, "max-operation").is_none());
}
