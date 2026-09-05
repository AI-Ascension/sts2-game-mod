// SPDX-License-Identifier: MIT

use super::*;

impl<G: RuntimeV3GameplayGamePort> RuntimeV3GameplayMod<G> {
    pub(super) fn wait(
        &mut self,
        request: &RuntimeV3GameplayMessage,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(request)?;
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        self.refresh_completion(operation_id);
        let Some(receipt) = self.receipts.get(operation_id) else {
            let mut response = self.result_response(
                request,
                RuntimeV3GameplayStatus::Unknown,
                None,
                None,
                None,
                Some("unknown_operation"),
                Some(RuntimeV3GameplayWaitOutcome::RecoveryRequired),
            );
            response.kind = RuntimeV3GameplayMessageKind::WaitResponse;
            return Ok(response);
        };
        if receipt.response.status == Some(RuntimeV3GameplayStatus::Accepted) {
            let mut response = self.result_response(
                request,
                RuntimeV3GameplayStatus::Unknown,
                None,
                None,
                None,
                Some("transition_timeout"),
                Some(RuntimeV3GameplayWaitOutcome::Timeout),
            );
            response.operation_id = Some(operation_id.to_owned());
            response.kind = RuntimeV3GameplayMessageKind::WaitResponse;
            return Ok(response);
        }
        match receipt.response.status {
            Some(RuntimeV3GameplayStatus::Settled) => {
                let mut response = correlated(&receipt.response, request);
                response.kind = RuntimeV3GameplayMessageKind::WaitResponse;
                response.wait_outcome = Some(if response.state_id == receipt.request.state_id {
                    RuntimeV3GameplayWaitOutcome::SameStateMutation
                } else {
                    RuntimeV3GameplayWaitOutcome::Successor
                });
                Ok(response)
            }
            Some(RuntimeV3GameplayStatus::Unknown) => {
                let mut response = correlated(&receipt.response, request);
                response.kind = RuntimeV3GameplayMessageKind::WaitResponse;
                response.wait_outcome = Some(RuntimeV3GameplayWaitOutcome::RecoveryRequired);
                Ok(response)
            }
            Some(RuntimeV3GameplayStatus::Rejected | RuntimeV3GameplayStatus::Cancelled) => {
                let mut response = self.result_response(
                    request,
                    RuntimeV3GameplayStatus::Unknown,
                    None,
                    None,
                    None,
                    Some("settlement_rejected"),
                    Some(RuntimeV3GameplayWaitOutcome::RecoveryRequired),
                );
                response.operation_id = Some(operation_id.to_owned());
                response.kind = RuntimeV3GameplayMessageKind::WaitResponse;
                Ok(response)
            }
            Some(RuntimeV3GameplayStatus::Accepted) | None => {
                Err(RuntimeV3GameplayError::OperationNotFound)
            }
        }
    }

    pub(super) fn recover(
        &mut self,
        request: &RuntimeV3GameplayMessage,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(request)?;
        let recovery = request
            .recovery
            .as_ref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        match recovery.kind {
            RuntimeV3GameplayRecoveryKind::Reobserve => {
                let (observation, legal_actions) = self.checked_snapshot()?;
                let mut response = self.result_response(
                    request,
                    RuntimeV3GameplayStatus::Accepted,
                    Some(observation),
                    Some(legal_actions),
                    None,
                    None,
                    None,
                );
                response.operation_id = Some("reobserve".to_owned());
                response.kind = RuntimeV3GameplayMessageKind::RecoverResponse;
                Ok(response)
            }
            RuntimeV3GameplayRecoveryKind::Reconcile => {
                let operation_id = recovery
                    .operation_id
                    .as_deref()
                    .ok_or(RuntimeV3GameplayError::OperationNotFound)?;
                self.refresh_completion(operation_id);
                let Some(receipt) = self.receipts.get(operation_id) else {
                    let mut response = self.result_response(
                        request,
                        RuntimeV3GameplayStatus::Unknown,
                        None,
                        None,
                        None,
                        Some("unknown_operation"),
                        None,
                    );
                    response.operation_id = Some(operation_id.to_owned());
                    response.kind = RuntimeV3GameplayMessageKind::RecoverResponse;
                    return Ok(response);
                };
                let mut response = correlated(&receipt.response, request);
                response.kind = RuntimeV3GameplayMessageKind::RecoverResponse;
                Ok(response)
            }
            RuntimeV3GameplayRecoveryKind::ReleaseLease
            | RuntimeV3GameplayRecoveryKind::StopEpisode => {
                Err(RuntimeV3GameplayError::UnsupportedRecovery)
            }
        }
    }
}
