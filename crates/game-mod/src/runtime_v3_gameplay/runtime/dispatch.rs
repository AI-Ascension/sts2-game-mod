// SPDX-License-Identifier: MIT

use super::*;

impl<G: RuntimeV3GameplayGamePort> RuntimeV3GameplayMod<G> {
    pub(super) fn execute_queued(
        &mut self,
        queued: QueuedOperation,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        let receipt = self
            .receipts
            .get(&queued.operation_id)
            .ok_or(RuntimeV3GameplayError::OperationNotFound)?;
        if receipt.response.status != Some(RuntimeV3GameplayStatus::Accepted) {
            return Ok(receipt.response.clone());
        }
        let request = receipt.request.clone();
        let (before, legal_actions) = match self.checked_snapshot() {
            Ok(value) => value,
            Err(_) => {
                return Ok(self.finish_unknown(
                    &queued.operation_id,
                    &request,
                    "settlement_unproven",
                ));
            }
        };
        let action = request
            .action
            .as_ref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        if request.generation != before.generation
            || request.state_id.as_deref() != Some(before.state_id.as_str())
            || !self.game.input_enabled()
            || !legal_actions.iter().any(|candidate| candidate == action)
        {
            let response = self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(before),
                Some(legal_actions),
                None,
                Some("action_not_current"),
                None,
            );
            return Ok(self.finish(&queued.operation_id, response));
        }
        let dispatch_result = self
            .game
            .dispatch(&self.identity, &queued.operation_id, action);
        self.finish_dispatch(&queued.operation_id, &request, before, dispatch_result)
    }

    fn finish_dispatch(
        &mut self,
        operation_id: &str,
        request: &RuntimeV3GameplayMessage,
        before: RuntimeV3GameplayObservation,
        dispatch_result: Result<(), RuntimeV3GameplayGameError>,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        if let Some(response) = self.proven_completion(request) {
            return Ok(self.finish(operation_id, response));
        }
        let after_result = self.checked_snapshot();
        let (after, after_actions) = match after_result {
            Ok(value) => value,
            Err(_) => {
                return Ok(self.finish_unknown(operation_id, request, "settlement_unproven"));
            }
        };
        let response = match dispatch_result {
            Err(
                error @ (RuntimeV3GameplayGameError::Rejected
                | RuntimeV3GameplayGameError::NotReady),
            ) if after == before => self.result_response(
                request,
                RuntimeV3GameplayStatus::Rejected,
                Some(after),
                Some(after_actions),
                None,
                Some(game_error_code(error)),
                None,
            ),
            Ok(()) | Err(_) => self.result_response(
                request,
                RuntimeV3GameplayStatus::Unknown,
                None,
                None,
                None,
                Some("settlement_unproven"),
                None,
            ),
        };
        Ok(self.finish(operation_id, response))
    }

    pub(super) fn proven_completion(
        &self,
        request: &RuntimeV3GameplayMessage,
    ) -> Option<RuntimeV3GameplayMessage> {
        let operation_id = request.operation_id.as_deref()?;
        let proof = self.game.completion(&self.identity, operation_id)?;
        if proof.identity != self.identity
            || proof.operation_id != operation_id
            || request.action.as_ref() != Some(&proof.action)
            || proof.transition.from_generation != request.generation
        {
            return None;
        }
        let response = self.result_response(
            request,
            RuntimeV3GameplayStatus::Settled,
            Some(proof.observation),
            Some(proof.legal_actions),
            Some(proof.transition),
            None,
            None,
        );
        response.validate().ok()?;
        Some(response)
    }

    pub(super) fn refresh_completion(&mut self, operation_id: &str) {
        let Some(receipt) = self.receipts.get(operation_id) else {
            return;
        };
        // Accepted work has not been dispatched; polling must never execute it.
        if receipt.response.status != Some(RuntimeV3GameplayStatus::Unknown) {
            return;
        }
        if let Some(response) = self.proven_completion(&receipt.request) {
            self.finish(operation_id, response);
        }
    }
}
