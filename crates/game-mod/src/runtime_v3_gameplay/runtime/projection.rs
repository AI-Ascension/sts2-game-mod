// SPDX-License-Identifier: MIT

use super::*;

impl<G: RuntimeV3GameplayGamePort> RuntimeV3GameplayMod<G> {
    pub(super) fn checked_snapshot(
        &self,
    ) -> Result<
        (
            RuntimeV3GameplayObservation,
            Vec<RuntimeV3GameplayLegalAction>,
        ),
        RuntimeV3GameplayError,
    > {
        let observation = self.game.snapshot().map_err(RuntimeV3GameplayError::Host)?;
        observation
            .validate()
            .map_err(|_| RuntimeV3GameplayError::InvalidObservation)?;
        let legal_actions = if self.game.input_enabled() {
            self.game
                .legal_actions(&observation)
                .map_err(RuntimeV3GameplayError::Host)?
        } else {
            Vec::new()
        };
        if legal_actions.len() > RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS {
            return Err(RuntimeV3GameplayError::InvalidActions);
        }
        for (index, action) in legal_actions.iter().enumerate() {
            action
                .validate()
                .map_err(|_| RuntimeV3GameplayError::InvalidActions)?;
            if legal_actions[..index]
                .iter()
                .any(|previous| previous.action_id == action.action_id)
            {
                return Err(RuntimeV3GameplayError::InvalidActions);
            }
        }
        Ok((observation, legal_actions))
    }

    pub(super) fn observation_response(
        &self,
        request: &RuntimeV3GameplayMessage,
        observation: RuntimeV3GameplayObservation,
        legal_actions: Vec<RuntimeV3GameplayLegalAction>,
        reobserve: bool,
    ) -> RuntimeV3GameplayMessage {
        let mut response = RuntimeV3GameplayMessage::base(
            context(request),
            observation.generation,
            if reobserve {
                RuntimeV3GameplayMessageKind::ReobserveResponse
            } else {
                RuntimeV3GameplayMessageKind::StateResponse
            },
        );
        response.state_id = Some(observation.state_id.clone());
        response.observation = Some(observation);
        response.legal_actions = Some(legal_actions);
        response
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn result_response(
        &self,
        request: &RuntimeV3GameplayMessage,
        status: RuntimeV3GameplayStatus,
        observation: Option<RuntimeV3GameplayObservation>,
        legal_actions: Option<Vec<RuntimeV3GameplayLegalAction>>,
        transition: Option<RuntimeV3GameplayTransitionWitness>,
        error_code: Option<&str>,
        wait_outcome: Option<RuntimeV3GameplayWaitOutcome>,
    ) -> RuntimeV3GameplayMessage {
        let mut response = RuntimeV3GameplayMessage::base(
            context(request),
            observation
                .as_ref()
                .map_or(request.generation, |value| value.generation),
            RuntimeV3GameplayMessageKind::DispatchActionResponse,
        );
        response.state_id = observation.as_ref().map(|value| value.state_id.clone());
        response.operation_id = request.operation_id.clone();
        response.observation = observation;
        response.legal_actions = legal_actions;
        response.status = Some(status);
        response.transition = transition;
        response.error_code = error_code.map(str::to_owned);
        response.wait_outcome = wait_outcome;
        response
    }
}
