// SPDX-License-Identifier: MIT

use std::error::Error;

use sts2_game_mod::{
    POC_MAX_EVIDENCE_RECORDS, POC_MAX_GENERATION, POC_MAX_REQUEST_BYTES, PocAction, PocCoreError,
    PocCorePort, PocCoreState, PocMessage, PocMessageKind, PocMod, PocModError, PocObservation,
    PocRoute, PocStatus,
};

const STATE_GOLDEN: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/state-response.json");
const ACCEPTED_GOLDEN: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-accepted.json");
const REJECTED_GOLDEN: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-rejected.json");
const INVALID_ACTION: &str =
    include_str!("../../../protocol-artifact/poc-v1/fixtures/invalid-action.json");

#[derive(Debug)]
struct FakeCore {
    state: PocCoreState,
}

impl PocCorePort for FakeCore {
    fn snapshot(&self) -> PocCoreState {
        self.state
    }

    fn apply(
        &mut self,
        expected_generation: u64,
        action: &PocAction,
    ) -> Result<PocCoreState, PocCoreError> {
        if expected_generation != self.state.generation {
            return Err(PocCoreError::StaleGeneration);
        }
        if action.action_id != "use_budget" {
            return Err(PocCoreError::InsufficientUnits);
        }
        if action.units == 0 {
            return Err(PocCoreError::ZeroUnits);
        }
        if action.units > self.state.available_units {
            return Err(PocCoreError::InsufficientUnits);
        }
        self.state = PocCoreState {
            generation: self.state.generation.saturating_add(1),
            available_units: self.state.available_units - action.units,
            settled_effects: self.state.settled_effects.saturating_add(1),
        };
        Ok(self.state)
    }
}

fn bytes(message: &PocMessage) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(serde_json::to_vec(message)?)
}

fn message(bytes: &[u8]) -> Result<PocMessage, Box<dyn Error>> {
    Ok(serde_json::from_slice(bytes)?)
}

fn mod_runtime() -> Result<PocMod<FakeCore>, Box<dyn Error>> {
    Ok(PocMod::new(FakeCore {
        state: PocCoreState {
            generation: 0,
            available_units: 3,
            settled_effects: 0,
        },
    })?)
}

#[test]
fn translates_state_valid_action_and_invalid_action_with_one_witness() -> Result<(), Box<dyn Error>>
{
    let mut runtime = mod_runtime()?;
    let state = runtime.handle(
        PocRoute::State,
        &bytes(&PocMessage::state_request("corr-0001", "instance-1"))?,
    )?;
    assert_eq!(state, STATE_GOLDEN.trim().as_bytes());
    let state = message(&state)?;
    assert_eq!(state.kind, PocMessageKind::StateResponse);
    assert_eq!(state.generation, 0);
    assert_eq!(
        state.observation.map(|value| value.available_units),
        Some(3)
    );

    let accepted = runtime.handle(
        PocRoute::Action,
        &bytes(&PocMessage::action_request(
            "corr-0002",
            "instance-1",
            0,
            PocAction::new("use_budget", 1),
        ))?,
    )?;
    assert_eq!(accepted, ACCEPTED_GOLDEN.trim().as_bytes());
    let accepted = message(&accepted)?;
    assert_eq!(accepted.status, Some(PocStatus::Accepted));
    assert_eq!(accepted.generation, 1);
    assert_eq!(accepted.error_code, None);
    assert_eq!(runtime.snapshot()?.available_units, 2);
    assert_eq!(runtime.snapshot()?.settled_effects, 1);

    let rejected = runtime.handle(PocRoute::Action, INVALID_ACTION.as_bytes())?;
    let rejected = message(&rejected)?;
    assert_eq!(rejected.status, Some(PocStatus::Rejected));
    assert_eq!(
        rejected.error_code.as_deref(),
        Some("sts2.game-core/zero_units")
    );
    assert_eq!(runtime.snapshot()?.available_units, 2);
    assert_eq!(runtime.snapshot()?.settled_effects, 1);
    assert_eq!(runtime.witnesses().len(), 1);
    let witness = runtime
        .witnesses()
        .first()
        .ok_or("missing effect witness")?;
    assert_eq!(witness.correlation_id, "corr-0002");
    assert_eq!(witness.instance_id, "instance-1");
    assert_eq!(witness.previous_generation, 0);
    assert_eq!(witness.generation, 1);
    assert_eq!(witness.available_units_before, 3);
    assert_eq!(witness.available_units_after, 2);
    assert_eq!(witness.settled_effects, 1);
    assert!(witness.settled);
    assert_eq!(runtime.records().len(), 3);

    let mut rejection_runtime = PocMod::new(FakeCore {
        state: PocCoreState {
            generation: 1,
            available_units: 3,
            settled_effects: 0,
        },
    })?;
    let rejected_golden = rejection_runtime.handle(
        PocRoute::Action,
        &bytes(&PocMessage::action_request(
            "corr-0005",
            "instance-1",
            1,
            PocAction::new("use_budget", 0),
        ))?,
    )?;
    assert_eq!(rejected_golden, REJECTED_GOLDEN.trim().as_bytes());
    Ok(())
}

#[test]
fn stale_generation_and_artifact_mismatch_fail_closed_without_forwarding()
-> Result<(), Box<dyn Error>> {
    let mut runtime = mod_runtime()?;
    let stale = PocMessage::action_request(
        "corr-stale",
        "instance-1",
        9,
        PocAction::new("use_budget", 1),
    );
    let rejected = runtime.handle(PocRoute::Action, &bytes(&stale)?)?;
    let rejected = message(&rejected)?;
    assert_eq!(rejected.status, Some(PocStatus::Rejected));
    assert_eq!(
        rejected.error_code.as_deref(),
        Some("sts2.game-core/stale_generation")
    );
    assert_eq!(runtime.snapshot()?.generation, 0);
    assert!(runtime.witnesses().is_empty());

    let mut malformed = PocMessage::state_request("corr-bad", "instance-1");
    malformed.schema_digest = String::from("bad");
    assert_eq!(
        runtime.handle(PocRoute::State, &bytes(&malformed)?),
        Err(PocModError::ArtifactMismatch)
    );
    assert_eq!(runtime.records().len(), 1);
    Ok(())
}

#[derive(Debug)]
struct InvalidSnapshotCore;

impl PocCorePort for InvalidSnapshotCore {
    fn snapshot(&self) -> PocCoreState {
        PocCoreState {
            generation: 0,
            available_units: 9,
            settled_effects: 5,
        }
    }

    fn apply(
        &mut self,
        _expected_generation: u64,
        _action: &PocAction,
    ) -> Result<PocCoreState, PocCoreError> {
        Err(PocCoreError::InsufficientUnits)
    }
}

#[derive(Debug)]
struct InvalidOutputCore {
    state: PocCoreState,
}

impl PocCorePort for InvalidOutputCore {
    fn snapshot(&self) -> PocCoreState {
        self.state
    }

    fn apply(
        &mut self,
        _expected_generation: u64,
        _action: &PocAction,
    ) -> Result<PocCoreState, PocCoreError> {
        Ok(PocCoreState {
            generation: 1,
            available_units: 9,
            settled_effects: 5,
        })
    }
}

#[derive(Debug)]
struct NoOpCore {
    state: PocCoreState,
}

impl PocCorePort for NoOpCore {
    fn snapshot(&self) -> PocCoreState {
        self.state
    }

    fn apply(
        &mut self,
        _expected_generation: u64,
        _action: &PocAction,
    ) -> Result<PocCoreState, PocCoreError> {
        Ok(self.state)
    }
}

#[derive(Debug)]
struct MutatingRejectCore {
    state: PocCoreState,
}

impl PocCorePort for MutatingRejectCore {
    fn snapshot(&self) -> PocCoreState {
        self.state
    }

    fn apply(
        &mut self,
        _expected_generation: u64,
        _action: &PocAction,
    ) -> Result<PocCoreState, PocCoreError> {
        self.state = PocCoreState {
            generation: 1,
            available_units: 2,
            settled_effects: 1,
        };
        Err(PocCoreError::ZeroUnits)
    }
}

fn initial_state() -> PocCoreState {
    PocCoreState {
        generation: 0,
        available_units: 3,
        settled_effects: 0,
    }
}

fn action_request() -> PocMessage {
    PocMessage::action_request(
        "corr-action",
        "instance-1",
        0,
        PocAction::new("use_budget", 1),
    )
}

#[test]
fn invalid_core_projection_is_rejected_before_wire_or_evidence() -> Result<(), Box<dyn Error>> {
    let mut runtime = PocMod::new(InvalidSnapshotCore)?;
    assert_eq!(
        runtime.handle(
            PocRoute::State,
            &bytes(&PocMessage::state_request("corr-invalid", "instance-1"))?,
        ),
        Err(PocModError::CoreStateBounds)
    );
    assert_eq!(runtime.snapshot(), Err(PocModError::CoreStateBounds));
    assert!(runtime.records().is_empty());
    Ok(())
}

#[test]
fn invalid_output_noop_and_post_error_mutation_fail_closed() -> Result<(), Box<dyn Error>> {
    let mut invalid_output = PocMod::new(InvalidOutputCore {
        state: initial_state(),
    })?;
    assert_eq!(
        invalid_output.handle(PocRoute::Action, &bytes(&action_request())?),
        Err(PocModError::CoreStateBounds)
    );
    assert!(invalid_output.witnesses().is_empty());

    let mut no_op = PocMod::new(NoOpCore {
        state: initial_state(),
    })?;
    assert_eq!(
        no_op.handle(PocRoute::Action, &bytes(&action_request())?),
        Err(PocModError::CoreTransition)
    );
    assert!(no_op.witnesses().is_empty());
    assert!(no_op.records().is_empty());

    let mut mutated_rejection = PocMod::new(MutatingRejectCore {
        state: initial_state(),
    })?;
    let mut invalid = action_request();
    invalid.action = Some(PocAction::new("use_budget", 0));
    assert_eq!(
        mutated_rejection.handle(PocRoute::Action, &bytes(&invalid)?),
        Err(PocModError::CoreTransition)
    );
    assert!(mutated_rejection.witnesses().is_empty());
    assert!(mutated_rejection.records().is_empty());
    Ok(())
}

#[test]
fn unknown_fields_and_oversized_requests_fail_before_forwarding() -> Result<(), Box<dyn Error>> {
    let mut runtime = mod_runtime()?;
    let oversized = vec![b' '; POC_MAX_REQUEST_BYTES + 1];
    assert_eq!(
        runtime.handle(PocRoute::State, &oversized),
        Err(PocModError::RequestTooLarge)
    );

    let mut outer = bytes(&PocMessage::state_request("corr-extra", "instance-1"))?;
    outer.pop();
    outer.extend_from_slice(br#", "extra": true}"#);
    assert_eq!(
        runtime.handle(PocRoute::State, &outer),
        Err(PocModError::MalformedRequest)
    );

    let mut nested = serde_json::to_value(action_request())?;
    let action = nested
        .get_mut("action")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("missing action object")?;
    action.insert("extra".to_owned(), serde_json::Value::Bool(true));
    assert_eq!(
        runtime.handle(PocRoute::Action, &serde_json::to_vec(&nested)?),
        Err(PocModError::MalformedRequest)
    );
    assert!(runtime.records().is_empty());
    Ok(())
}

#[test]
fn evidence_retention_has_an_explicit_bound() -> Result<(), Box<dyn Error>> {
    let mut runtime = mod_runtime()?;
    for index in 0..POC_MAX_EVIDENCE_RECORDS {
        let correlation = format!("corr-{index}");
        runtime.handle(
            PocRoute::State,
            &bytes(&PocMessage::state_request(&correlation, "instance-1"))?,
        )?;
    }
    assert_eq!(runtime.records().len(), POC_MAX_EVIDENCE_RECORDS);
    assert_eq!(
        runtime.handle(
            PocRoute::State,
            &bytes(&PocMessage::state_request("corr-overflow", "instance-1"))?,
        ),
        Err(PocModError::EvidenceLimit)
    );
    Ok(())
}

#[test]
fn observation_validation_rejects_out_of_range_values() {
    assert_eq!(
        PocObservation {
            available_units: 9,
            settled_effects: 0,
        }
        .validate(),
        Err(sts2_game_mod::PocValidationError::ObservationBounds)
    );
    assert_eq!(
        PocCoreState {
            generation: POC_MAX_GENERATION + 1,
            available_units: 0,
            settled_effects: 0,
        }
        .validate(),
        Err(sts2_game_mod::PocValidationError::GenerationBounds)
    );
}
