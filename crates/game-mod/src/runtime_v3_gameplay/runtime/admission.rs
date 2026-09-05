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
            let mut original = receipt.request.clone();
            original.correlation_id.clone_from(&request.correlation_id);
            if original == request {
                return Ok(correlated(&receipt.response, &request));
            }
            return Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(receipt.admitted_observation.clone()),
                Some(receipt.admitted_actions.clone()),
                None,
                Some("idempotency_conflict"),
                None,
            ));
        }
        let (observation, legal_actions) = self.checked_snapshot()?;
        if request.state_id.as_deref() != Some(observation.state_id.as_str())
            || request.generation != observation.generation
        {
            return Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("stale_generation"),
                None,
            ));
        }
        let action = request
            .action
            .as_ref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        if !self.game.input_enabled() {
            return Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("input_disabled"),
                None,
            ));
        }
        if !legal_actions.iter().any(|candidate| candidate == action) {
            return Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("action_not_current"),
                None,
            ));
        }
        if self.receipts.len() >= self.receipt_capacity {
            return Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("receipt_store_full"),
                None,
            ));
        }
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
            Err(QueueError::Closed) => Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("queue_closed"),
                None,
            )),
            Err(QueueError::Full { .. }) => Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("queue_full"),
                None,
            )),
        }
    }
}
