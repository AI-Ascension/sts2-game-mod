// SPDX-License-Identifier: MIT

use super::contract::{
    RUNTIME_V3_GAMEPLAY_MAX_GENERATION, RuntimeV3GameplayIdentity, RuntimeV3GameplayLegalAction,
    RuntimeV3GameplayObservation, RuntimeV3GameplayTransitionWitness,
};

/// Owned evidence supplied by the host's operation-specific completion source.
#[derive(Clone, Debug)]
pub struct RuntimeV3GameplayCompletion {
    pub identity: RuntimeV3GameplayIdentity,
    pub operation_id: String,
    pub action: RuntimeV3GameplayLegalAction,
    pub observation: RuntimeV3GameplayObservation,
    pub legal_actions: Vec<RuntimeV3GameplayLegalAction>,
    pub transition: RuntimeV3GameplayTransitionWitness,
}

/// Host-side failures kept separate from queue and protocol failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayGameError {
    /// The operation was not started and cannot complete later.
    NotReady,
    /// Definitive non-application; use MutationUncertain after possible submission.
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
    /// Completion sources must key by the entire identity plus operation ID.
    fn dispatch(
        &mut self,
        identity: &RuntimeV3GameplayIdentity,
        operation_id: &str,
        action: &RuntimeV3GameplayLegalAction,
    ) -> Result<(), RuntimeV3GameplayGameError>;

    /// Read-only polling; absence of independent proof must never imply settlement.
    fn completion(
        &self,
        _identity: &RuntimeV3GameplayIdentity,
        _operation_id: &str,
    ) -> Option<RuntimeV3GameplayCompletion> {
        None
    }
}

/// Deterministic host double for contract and queue tests.
#[derive(Debug)]
pub struct FakeRuntimeV3GameplayGame {
    observation: RuntimeV3GameplayObservation,
    legal_actions: Vec<super::contract::RuntimeV3GameplayLegalAction>,
    input_enabled: bool,
    dispatch_calls: usize,
    completions: std::collections::BTreeMap<String, RuntimeV3GameplayCompletion>,
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
            completions: std::collections::BTreeMap::new(),
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
            .filter(|generation| *generation <= RUNTIME_V3_GAMEPLAY_MAX_GENERATION)
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
        identity: &RuntimeV3GameplayIdentity,
        operation_id: &str,
        action: &RuntimeV3GameplayLegalAction,
    ) -> Result<(), RuntimeV3GameplayGameError> {
        if !self.input_enabled {
            return Err(RuntimeV3GameplayGameError::NotReady);
        }
        if !self
            .legal_actions
            .iter()
            .any(|candidate| candidate == action)
        {
            return Err(RuntimeV3GameplayGameError::Rejected);
        }
        let from_generation = self.observation.generation;
        self.advance_generation()?;
        self.dispatch_calls += 1;
        // This is an explicit synthetic completion oracle, not STS2 effect evidence.
        self.completions.insert(
            operation_id.to_owned(),
            RuntimeV3GameplayCompletion {
                identity: identity.clone(),
                operation_id: operation_id.to_owned(),
                action: action.clone(),
                observation: self.observation.clone(),
                legal_actions: self.legal_actions.clone(),
                transition: RuntimeV3GameplayTransitionWitness {
                    from_generation,
                    to_generation: self.observation.generation,
                    state_id: self.observation.state_id.clone(),
                    effect_kind: "fake.semantic-completion".to_owned(),
                },
            },
        );
        Ok(())
    }

    fn completion(
        &self,
        identity: &RuntimeV3GameplayIdentity,
        operation_id: &str,
    ) -> Option<RuntimeV3GameplayCompletion> {
        self.completions
            .get(operation_id)
            .filter(|proof| &proof.identity == identity)
            .cloned()
    }
}
