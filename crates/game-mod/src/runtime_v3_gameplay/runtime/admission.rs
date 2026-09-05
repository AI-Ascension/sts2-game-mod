// SPDX-License-Identifier: MIT

use super::*;

impl<G: RuntimeV3GameplayGamePort> RuntimeV3GameplayMod<G> {
    pub(super) fn admit_action(
        &mut self,
        request: RuntimeV3GameplayMessage,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(&request)?;
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        if let Some(receipt) = self.receipts.get(operation_id) {
            return Ok(self.replay_admission(&request, receipt));
        }
        let (observation, legal_actions) = self.checked_snapshot()?;
        if request.state_id.as_deref() != Some(observation.state_id.as_str())
            || request.generation != observation.generation
        {
            return Ok(self.rejected_admission(
                &request,
                observation,
                legal_actions,
                "stale_generation",
            ));
        }
        let action = request
            .action
            .as_ref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        if !self.game.input_enabled() {
            return Ok(self.rejected_admission(
                &request,
                observation,
                legal_actions,
                "input_disabled",
            ));
        }
        if !legal_actions.iter().any(|candidate| candidate == action) {
            return Ok(self.rejected_admission(
                &request,
                observation,
                legal_actions,
                "action_not_current",
            ));
        }
        if self.receipts.len() >= self.receipt_capacity {
            return Ok(self.rejected_admission(
                &request,
                observation,
                legal_actions,
                "receipt_store_full",
            ));
        }
        self.enqueue_action(request, observation, legal_actions)
    }

    fn replay_admission(
        &self,
        request: &RuntimeV3GameplayMessage,
        receipt: &OperationReceipt,
    ) -> RuntimeV3GameplayMessage {
        let mut original = receipt.request.clone();
        original.correlation_id.clone_from(&request.correlation_id);
        if original == *request {
            return correlated(&receipt.response, request);
        }
        self.rejected_admission(
            request,
            receipt.admitted_observation.clone(),
            receipt.admitted_actions.clone(),
            "idempotency_conflict",
        )
    }

    fn enqueue_action(
        &mut self,
        request: RuntimeV3GameplayMessage,
        observation: RuntimeV3GameplayObservation,
        legal_actions: Vec<RuntimeV3GameplayLegalAction>,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        let accepted = self.result_response(
            &request,
            RuntimeV3GameplayStatus::Accepted,
            Some(observation.clone()),
            Some(legal_actions.clone()),
            None,
            None,
            None,
        );
        match self.queue.enqueue(QueuedOperation {
            operation_id: operation_id.to_owned(),
        }) {
            Ok(()) => {
                self.receipts.insert(
                    operation_id.to_owned(),
                    OperationReceipt {
                        request,
                        response: accepted.clone(),
                        admitted_observation: observation,
                        admitted_actions: legal_actions,
                    },
                );
                Ok(accepted)
            }
            Err(QueueError::Closed) => {
                Ok(self.rejected_admission(&request, observation, legal_actions, "queue_closed"))
            }
            Err(QueueError::Full { .. }) => {
                Ok(self.rejected_admission(&request, observation, legal_actions, "queue_full"))
            }
        }
    }

    fn rejected_admission(
        &self,
        request: &RuntimeV3GameplayMessage,
        observation: RuntimeV3GameplayObservation,
        legal_actions: Vec<RuntimeV3GameplayLegalAction>,
        error_code: &str,
    ) -> RuntimeV3GameplayMessage {
        self.result_response(
            request,
            RuntimeV3GameplayStatus::Rejected,
            Some(observation),
            Some(legal_actions),
            None,
            Some(error_code),
            None,
        )
    }
}
