// SPDX-License-Identifier: MIT

use super::contract::{RuntimeV2CombatPhase, RuntimeV2Observation, RuntimeV2ValidationError};

/// Failures a host-specific implementation may report after dispatch.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV2GameError {
    /// The fake host was not ready when dispatch reached it.
    NotReady,
    /// The host rejected the action without changing its observation.
    Rejected,
}

/// Host-facing capability required by the Runtime-v2 fake boundary.
pub trait RuntimeV2GamePort {
    /// Returns an owned observation; no host reference crosses this boundary.
    fn snapshot(&self) -> RuntimeV2Observation;

    /// Applies one already-admitted `end_turn` on the host thread.
    fn end_turn(&mut self) -> Result<(), RuntimeV2GameError>;
}

/// Deterministic in-memory host used by Runtime-v2 tests.
#[derive(Debug)]
pub struct FakeRuntimeV2Game {
    observation: RuntimeV2Observation,
    end_turn_calls: usize,
}

impl FakeRuntimeV2Game {
    /// Creates a fake host from a validated observation.
    pub fn new(observation: RuntimeV2Observation) -> Result<Self, RuntimeV2ValidationError> {
        observation.validate()?;
        Ok(Self {
            observation,
            end_turn_calls: 0,
        })
    }

    /// Returns how many times the fake host was asked to apply `end_turn`.
    #[must_use]
    pub const fn end_turn_calls(&self) -> usize {
        self.end_turn_calls
    }
}

impl RuntimeV2GamePort for FakeRuntimeV2Game {
    fn snapshot(&self) -> RuntimeV2Observation {
        self.observation
    }

    fn end_turn(&mut self) -> Result<(), RuntimeV2GameError> {
        if !self.observation.host_ready {
            return Err(RuntimeV2GameError::NotReady);
        }
        if self.observation.combat_phase != RuntimeV2CombatPhase::PlayerTurn {
            return Err(RuntimeV2GameError::Rejected);
        }
        let generation = self.observation.generation.checked_add(1);
        let turn_index = self.observation.turn_index.checked_add(1);
        let (Some(generation), Some(turn_index)) = (generation, turn_index) else {
            return Err(RuntimeV2GameError::Rejected);
        };
        if turn_index > 1024 {
            return Err(RuntimeV2GameError::Rejected);
        }
        self.observation.generation = generation;
        self.observation.turn_index = turn_index;
        self.end_turn_calls += 1;
        Ok(())
    }
}
