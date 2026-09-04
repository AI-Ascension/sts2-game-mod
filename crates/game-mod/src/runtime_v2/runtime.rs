// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_game_mod_host::{MainThreadQueue, QueueError};

use super::artifact::verify_runtime_v2_artifact;
use super::contract::{RuntimeV2Kind, RuntimeV2Message, RuntimeV2Observation, RuntimeV2Status};
use super::fake::RuntimeV2GamePort;
use super::receipt::{OperationReceipt, OperationState, QueuedOperation};
use super::support::{
    legality_error, rejected_action, rejected_reconcile, request_validation_error,
};
use super::types::{RuntimeV2Config, RuntimeV2Error, RuntimeV2Mod};

impl<G: RuntimeV2GamePort> RuntimeV2Mod<G> {
    /// Verifies the copied artifact and creates one isolated fake runtime instance.
    pub fn new(game: G, config: RuntimeV2Config) -> Result<Self, RuntimeV2Error> {
        verify_runtime_v2_artifact().map_err(RuntimeV2Error::Artifact)?;
        config.validate()?;
        Ok(Self {
            game,
            identity: config.identity,
            queue: MainThreadQueue::new(config.queue_capacity),
            receipts: VecDeque::with_capacity(config.receipt_capacity),
            receipt_capacity: config.receipt_capacity,
            max_request_bytes: config.max_request_bytes,
        })
    }

    /// Returns the current owned fake-host observation.
    pub fn snapshot(&self) -> Result<RuntimeV2Observation, RuntimeV2Error> {
        self.checked_snapshot()
    }

    /// Returns the number of operations retained for replay or reconciliation.
    #[must_use]
    pub fn receipt_len(&self) -> usize {
        self.receipts.len()
    }

    /// Returns the number of admitted operations waiting for dispatch.
    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    /// Returns the fake game port after the boundary is no longer needed.
    #[must_use]
    pub fn into_game(self) -> G {
        self.game
    }

    /// Closes admission while retaining queued operations for deterministic draining.
    pub fn close(&mut self) {
        self.queue.close();
    }

    /// Decodes one strict Runtime-v2 request and returns its encoded response.
    pub fn handle(&mut self, body: &[u8]) -> Result<Vec<u8>, RuntimeV2Error> {
        if body.len() > self.max_request_bytes {
            return Err(RuntimeV2Error::RequestTooLarge);
        }
        let request: RuntimeV2Message =
            serde_json::from_slice(body).map_err(|_| RuntimeV2Error::MalformedRequest)?;
        request
            .validate_request()
            .map_err(request_validation_error)?;
        let response = match request.kind {
            RuntimeV2Kind::StateRequest => self.state(&request)?,
            RuntimeV2Kind::ActionRequest => self.admit_action(request)?,
            RuntimeV2Kind::ReconcileRequest => self.reconcile_operation(request)?,
            RuntimeV2Kind::StateResponse
            | RuntimeV2Kind::ActionResponse
            | RuntimeV2Kind::ReconcileResponse => return Err(RuntimeV2Error::MalformedRequest),
        };
        serde_json::to_vec(&response).map_err(|_| RuntimeV2Error::Encoding)
    }

    /// Admits one valid action or returns a protocol-level rejection without mutation.
    pub fn admit_action(
        &mut self,
        request: RuntimeV2Message,
    ) -> Result<RuntimeV2Message, RuntimeV2Error> {
        if request.kind != RuntimeV2Kind::ActionRequest {
            return Err(RuntimeV2Error::MalformedRequest);
        }
        request
            .validate_request()
            .map_err(request_validation_error)?;
        let observation = self.checked_snapshot()?;
        if !self.identity.matches(&request) {
            return Ok(rejected_action(&request, observation, "stale_identity"));
        }
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV2Error::MalformedRequest)?;
        if let Some(index) = self.receipt_index(operation_id) {
            if self.receipts[index].request == request {
                return Ok(self.receipts[index].action_replay());
            }
            return Ok(rejected_action(
                &request,
                observation,
                "idempotency_conflict",
            ));
        }
        if request.generation != observation.generation {
            return Ok(rejected_action(
                &request,
                observation,
                "sts2.game-core/stale_generation",
            ));
        }
        if let Some(error_code) = legality_error(observation) {
            return Ok(rejected_action(&request, observation, error_code));
        }
        if self.receipts.len() >= self.receipt_capacity {
            return Ok(rejected_action(
                &request,
                observation,
                "sts2.runtime/receipt_store_full",
            ));
        }
        let accepted = RuntimeV2Message::action_response(
            &request,
            RuntimeV2Status::Accepted,
            observation.generation,
            Some(observation),
            None,
            None,
        );
        match self.queue.enqueue(QueuedOperation {
            operation_id: operation_id.to_owned(),
        }) {
            Ok(()) => {
                self.receipts.push_back(OperationReceipt {
                    request,
                    state: OperationState::Pending {
                        accepted: Box::new(accepted.clone()),
                    },
                });
                Ok(accepted)
            }
            Err(QueueError::Closed) => Ok(rejected_action(
                &request,
                observation,
                "sts2.runtime/queue_closed",
            )),
            Err(QueueError::Full { .. }) => Ok(rejected_action(
                &request,
                observation,
                "sts2.runtime/queue_full",
            )),
        }
    }

    /// Drains at most `budget` queued operations in FIFO order.
    pub fn pump(&mut self, budget: usize) -> Result<Vec<RuntimeV2Message>, RuntimeV2Error> {
        self.queue
            .drain(budget)
            .into_iter()
            .map(|queued| self.execute_queued(queued, false))
            .collect()
    }

    /// Simulates a client disconnect after a successful in-memory write.
    pub fn simulate_post_write_disconnect(
        &mut self,
        operation_id: &str,
    ) -> Result<RuntimeV2Message, RuntimeV2Error> {
        let queued = self
            .queue
            .remove_matching(|item| item.operation_id == operation_id)
            .ok_or(RuntimeV2Error::OperationNotQueued)?;
        self.execute_queued(queued, true)
    }

    /// Simulates a timeout before dispatch and removes the work so it cannot execute later.
    pub fn simulate_timeout(
        &mut self,
        request: &RuntimeV2Message,
    ) -> Result<RuntimeV2Message, RuntimeV2Error> {
        if request.kind != RuntimeV2Kind::ActionRequest {
            return Err(RuntimeV2Error::MalformedRequest);
        }
        request
            .validate_request()
            .map_err(request_validation_error)?;
        if !self.identity.matches(request) {
            return Err(RuntimeV2Error::StaleIdentity);
        }
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV2Error::MalformedRequest)?;
        let index = self
            .receipt_index(operation_id)
            .ok_or(RuntimeV2Error::OperationNotFound)?;
        if self.receipts[index].request != *request {
            return Ok(rejected_action(
                request,
                self.checked_snapshot()?,
                "idempotency_conflict",
            ));
        }
        if self.receipts[index].is_pending() {
            self.queue
                .remove_matching(|item| item.operation_id == operation_id);
            self.receipts[index].state = OperationState::Unknown {
                error_code: "sts2.runtime/unknown_after_disconnect".to_owned(),
                settled: None,
            };
        }
        Ok(self.receipts[index].action_replay())
    }

    /// Records a cancellation before admission, retaining it so a later retry cannot mutate.
    pub fn cancel_before_admission(
        &mut self,
        request: RuntimeV2Message,
    ) -> Result<RuntimeV2Message, RuntimeV2Error> {
        if request.kind != RuntimeV2Kind::ActionRequest {
            return Err(RuntimeV2Error::MalformedRequest);
        }
        request
            .validate_request()
            .map_err(request_validation_error)?;
        let observation = self.checked_snapshot()?;
        if !self.identity.matches(&request) {
            return Ok(rejected_action(&request, observation, "stale_identity"));
        }
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV2Error::MalformedRequest)?;
        if let Some(index) = self.receipt_index(operation_id) {
            if self.receipts[index].request == request && self.receipts[index].is_pending() {
                return Err(RuntimeV2Error::CancellationAfterAdmission);
            }
            if self.receipts[index].request == request {
                return Ok(self.receipts[index].action_replay());
            }
            return Ok(rejected_action(
                &request,
                observation,
                "idempotency_conflict",
            ));
        }
        if self.receipts.len() >= self.receipt_capacity {
            return Ok(rejected_action(
                &request,
                observation,
                "sts2.runtime/receipt_store_full",
            ));
        }
        let cancelled = RuntimeV2Message::action_response(
            &request,
            RuntimeV2Status::Cancelled,
            observation.generation,
            Some(observation),
            None,
            Some("sts2.runtime/cancelled_before_dispatch"),
        );
        self.receipts.push_back(OperationReceipt {
            request,
            state: OperationState::Cancelled {
                observation,
                error_code: "sts2.runtime/cancelled_before_dispatch".to_owned(),
            },
        });
        Ok(cancelled)
    }

    fn state(&self, request: &RuntimeV2Message) -> Result<RuntimeV2Message, RuntimeV2Error> {
        if !self.identity.matches(request) {
            return Err(RuntimeV2Error::StaleIdentity);
        }
        Ok(RuntimeV2Message::state_response(
            request,
            self.checked_snapshot()?,
        ))
    }

    fn reconcile_operation(
        &self,
        request: RuntimeV2Message,
    ) -> Result<RuntimeV2Message, RuntimeV2Error> {
        let observation = self.checked_snapshot()?;
        if !self.identity.matches(&request) {
            return Ok(rejected_reconcile(&request, observation, "stale_identity"));
        }
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV2Error::MalformedRequest)?;
        let Some(index) = self.receipt_index(operation_id) else {
            return Ok(rejected_reconcile(
                &request,
                observation,
                "sts2.runtime/unknown_operation",
            ));
        };
        let record = &self.receipts[index];
        if request.generation != record.request.generation
            && request.generation != observation.generation
        {
            return Ok(rejected_reconcile(
                &request,
                observation,
                "sts2.game-core/stale_generation",
            ));
        }
        Ok(record.reconcile(&request))
    }
}
