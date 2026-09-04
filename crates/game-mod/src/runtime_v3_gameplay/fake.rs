// SPDX-License-Identifier: MIT

use super::contract::{
    RuntimeV3GameplayAction, RuntimeV3GameplayObservation,
};

/// Host-side failures kept separate from queue and protocol failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayGameError {
    NotReady,
    Rejected,
    ProjectionInvalid,
    MutationUncertain,
}

/// Authoritative host capability required by the semantic Runtime-v3 adapter.
pub trait RuntimeV3GameplayGamePort {
    fn snapshot(&self) -> Result<RuntimeV3GameplayObservation, RuntimeV3GameplayGameError>;

    /// Produces the complete legal-action set for exactly this observation.
    fn legal_actions(
        &self,
        observation: &RuntimeV3GameplayObservation,
    ) -> Result<Vec<super::contract::RuntimeV3GameplayLegalAction>, RuntimeV3GameplayGameError>;

    /// Reports modal/input state; false is never converted into a gameplay action.
    fn input_enabled(&self) -> bool;

    /// Applies a previously admitted semantic action on the host thread.
    fn dispatch(
        &mut self,
        action: &RuntimeV3GameplayAction,
    ) -> Result<(), RuntimeV3GameplayGameError>;
}

/// Deterministic host double for contract and queue tests.
#[derive(Debug)]
pub struct FakeRuntimeV3GameplayGame {
    observation: RuntimeV3GameplayObservation,
    legal_actions: Vec<super::contract::RuntimeV3GameplayLegalAction>,
    input_enabled: bool,
    dispatch_calls: usize,
}

impl FakeRuntimeV3GameplayGame {
    pub fn new(
        observation: RuntimeV3GameplayObservation,
        legal_actions: Vec<super::contract::RuntimeV3GameplayLegalAction>,
    ) -> Result<Self, super::contract::RuntimeV3GameplayValidationError> {
        observation.validate()?;
        for action in &legal_actions {
            action.validate()?;
        }
        Ok(Self {
            observation,
            legal_actions,
            input_enabled: true,
            dispatch_calls: 0,
        })
    }

    pub fn set_input_enabled(&mut self, input_enabled: bool) {
        self.input_enabled = input_enabled;
    }

    #[must_use]
    pub const fn dispatch_calls(&self) -> usize {
        self.dispatch_calls
    }

    pub fn advance_generation(&mut self) -> Result<(), RuntimeV3GameplayGameError> {
        self.observation.generation = self
            .observation
            .generation
            .checked_add(1)
            .ok_or(RuntimeV3GameplayGameError::ProjectionInvalid)?;
        Ok(())
    }
}

impl RuntimeV3GameplayGamePort for FakeRuntimeV3GameplayGame {
    fn snapshot(&self) -> Result<RuntimeV3GameplayObservation, RuntimeV3GameplayGameError> {
        Ok(self.observation.clone())
    }

    fn legal_actions(
        &self,
        observation: &RuntimeV3GameplayObservation,
    ) -> Result<Vec<super::contract::RuntimeV3GameplayLegalAction>, RuntimeV3GameplayGameError>
    {
        if observation.state_id != self.observation.state_id
            || observation.generation != self.observation.generation
        {
            return Err(RuntimeV3GameplayGameError::ProjectionInvalid);
        }
        Ok(self.legal_actions.clone())
    }

    fn input_enabled(&self) -> bool {
        self.input_enabled
    }

    fn dispatch(
        &mut self,
        action: &RuntimeV3GameplayAction,
    ) -> Result<(), RuntimeV3GameplayGameError> {
        if !self.input_enabled {
            return Err(RuntimeV3GameplayGameError::NotReady);
        }
        if !self
            .legal_actions
            .iter()
            .any(|candidate| &candidate.action == action)
        {
            return Err(RuntimeV3GameplayGameError::Rejected);
        }
        self.dispatch_calls += 1;
        self.advance_generation()
    }
}
