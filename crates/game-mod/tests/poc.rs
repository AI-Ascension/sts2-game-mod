// SPDX-License-Identifier: MIT

use std::error::Error;

use sts2_game_mod::{
    PocAction, PocCoreError, PocCorePort, PocCoreState, PocMessage, PocMessageKind, PocMod,
    PocModError, PocRoute, PocStatus,
};

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
    let accepted = message(&accepted)?;
    assert_eq!(accepted.status, Some(PocStatus::Accepted));
    assert_eq!(accepted.generation, 1);
    assert_eq!(accepted.error_code, None);
    assert_eq!(runtime.snapshot().available_units, 2);
    assert_eq!(runtime.snapshot().settled_effects, 1);

    let rejected = runtime.handle(
        PocRoute::Action,
        &bytes(&PocMessage::action_request(
            "corr-0003",
            "instance-1",
            1,
            PocAction::new("use_budget", 0),
        ))?,
    )?;
    let rejected = message(&rejected)?;
    assert_eq!(rejected.status, Some(PocStatus::Rejected));
    assert_eq!(
        rejected.error_code.as_deref(),
        Some("sts2.game-core/zero_units")
    );
    assert_eq!(runtime.snapshot().available_units, 2);
    assert_eq!(runtime.snapshot().settled_effects, 1);
    assert_eq!(runtime.witnesses().len(), 1);
    let witness = runtime
        .witnesses()
        .first()
        .ok_or("missing effect witness")?;
    assert_eq!(witness.correlation_id, "corr-0002");
    assert_eq!(witness.instance_id, "instance-1");
    assert_eq!(witness.generation, 1);
    assert!(witness.settled);
    assert_eq!(runtime.records().len(), 3);
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
    assert_eq!(runtime.snapshot().generation, 0);
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
