// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use sts2_game_mod_host::{MainThreadQueue, QueueError};

use super::contract::{
    RUNTIME_V3_GAMEPLAY_MAX_GENERATION, RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS,
    RuntimeV3GameplayContext, RuntimeV3GameplayIdentity, RuntimeV3GameplayLegalAction,
    RuntimeV3GameplayMessage, RuntimeV3GameplayMessageKind, RuntimeV3GameplayObservation,
    RuntimeV3GameplayRecoveryKind, RuntimeV3GameplayStatus, RuntimeV3GameplayTransitionWitness,
    RuntimeV3GameplayWaitOutcome,
};
use super::fake::{RuntimeV3GameplayGameError, RuntimeV3GameplayGamePort};

const MAX_QUEUE_CAPACITY: usize = 1024;
const MAX_RECEIPTS: usize = 1024;
const MAX_REQUEST_BYTES: usize = 128 * 1024;

#[derive(Clone, Debug)]
pub struct RuntimeV3GameplayConfig {
    pub identity: RuntimeV3GameplayIdentity,
    pub queue_capacity: usize,
    pub receipt_capacity: usize,
    pub max_request_bytes: usize,
}

impl Default for RuntimeV3GameplayConfig {
    fn default() -> Self {
        Self {
            identity: RuntimeV3GameplayIdentity::new("instance-1", "session-1", "lease-1", 1),
            queue_capacity: 8,
            receipt_capacity: 32,
            max_request_bytes: MAX_REQUEST_BYTES,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeV3GameplayError {
    RequestTooLarge,
    MalformedRequest,
    Encoding,
    InvalidConfig,
    InvalidObservation,
    InvalidActions,
    StaleIdentity,
    StaleGeneration,
    OperationNotFound,
    OperationConflict,
    QueueClosed,
    QueueFull,
    ReceiptStoreFull,
    Host(RuntimeV3GameplayGameError),
    UnsupportedRecovery,
}

impl std::fmt::Display for RuntimeV3GameplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::RequestTooLarge => "runtime-v3 request exceeds its byte bound",
            Self::MalformedRequest => "runtime-v3 request is malformed",
            Self::Encoding => "runtime-v3 response could not be encoded",
            Self::InvalidConfig => "runtime-v3 configuration is invalid",
            Self::InvalidObservation => "host returned an invalid fair-play observation",
            Self::InvalidActions => "host returned an invalid legal-action catalog",
            Self::StaleIdentity => "runtime-v3 request has stale identity or lease",
            Self::StaleGeneration => "runtime-v3 request has a stale generation",
            Self::OperationNotFound => "runtime-v3 operation is not retained",
            Self::OperationConflict => {
                "runtime-v3 operation identity conflicts with a prior request"
            }
            Self::QueueClosed => "runtime-v3 admission queue is closed",
            Self::QueueFull => "runtime-v3 admission queue is full",
            Self::ReceiptStoreFull => "runtime-v3 receipt store is full",
            Self::Host(_) => "host rejected or could not prove the semantic operation",
            Self::UnsupportedRecovery => "recovery operation belongs to the gateway or harness",
        })
    }
}

impl std::error::Error for RuntimeV3GameplayError {}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueuedOperation {
    operation_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OperationReceipt {
    request: RuntimeV3GameplayMessage,
    response: RuntimeV3GameplayMessage,
}

#[derive(Debug)]
pub struct RuntimeV3GameplayMod<G> {
    pub(super) game: G,
    pub(super) identity: RuntimeV3GameplayIdentity,
    queue: MainThreadQueue<QueuedOperation>,
    receipts: BTreeMap<String, OperationReceipt>,
    receipt_capacity: usize,
    max_request_bytes: usize,
}

impl<G: RuntimeV3GameplayGamePort> RuntimeV3GameplayMod<G> {
    pub fn new(game: G, config: RuntimeV3GameplayConfig) -> Result<Self, RuntimeV3GameplayError> {
        if !valid_identity(&config.identity.instance_id)
            || !valid_identity(&config.identity.session_id)
            || !valid_identity(&config.identity.lease_id)
            || config.identity.lease_epoch > RUNTIME_V3_GAMEPLAY_MAX_GENERATION
            || config.queue_capacity == 0
            || config.queue_capacity > MAX_QUEUE_CAPACITY
            || config.receipt_capacity == 0
            || config.receipt_capacity > MAX_RECEIPTS
            || config.max_request_bytes == 0
            || config.max_request_bytes > MAX_REQUEST_BYTES
        {
            return Err(RuntimeV3GameplayError::InvalidConfig);
        }
        Ok(Self {
            game,
            identity: config.identity,
            queue: MainThreadQueue::new(config.queue_capacity),
            receipts: BTreeMap::new(),
            receipt_capacity: config.receipt_capacity,
            max_request_bytes: config.max_request_bytes,
        })
    }

    pub fn handle(&mut self, body: &[u8]) -> Result<Vec<u8>, RuntimeV3GameplayError> {
        if body.len() > self.max_request_bytes {
            return Err(RuntimeV3GameplayError::RequestTooLarge);
        }
        let request: RuntimeV3GameplayMessage =
            serde_json::from_slice(body).map_err(|_| RuntimeV3GameplayError::MalformedRequest)?;
        request
            .validate_request()
            .map_err(|_| RuntimeV3GameplayError::MalformedRequest)?;
        let response = match request.kind {
            RuntimeV3GameplayMessageKind::StateRequest => self.state(&request, false)?,
            RuntimeV3GameplayMessageKind::ReobserveRequest => self.state(&request, true)?,
            RuntimeV3GameplayMessageKind::LegalActionsRequest => self.legal_actions(&request)?,
            RuntimeV3GameplayMessageKind::DispatchActionRequest => self.admit_action(request)?,
            RuntimeV3GameplayMessageKind::WaitRequest => self.wait(&request)?,
            RuntimeV3GameplayMessageKind::RecoverRequest => self.recover(&request)?,
            RuntimeV3GameplayMessageKind::StateResponse
            | RuntimeV3GameplayMessageKind::ReobserveResponse
            | RuntimeV3GameplayMessageKind::LegalActionsResponse
            | RuntimeV3GameplayMessageKind::DispatchActionResponse
            | RuntimeV3GameplayMessageKind::WaitResponse
            | RuntimeV3GameplayMessageKind::RecoverResponse => {
                return Err(RuntimeV3GameplayError::MalformedRequest);
            }
        };
        serde_json::to_vec(&response).map_err(|_| RuntimeV3GameplayError::Encoding)
    }

    pub fn snapshot(
        &self,
    ) -> Result<
        (
            RuntimeV3GameplayObservation,
            Vec<RuntimeV3GameplayLegalAction>,
        ),
        RuntimeV3GameplayError,
    > {
        self.checked_snapshot()
    }

    pub fn pump(
        &mut self,
        budget: usize,
    ) -> Result<Vec<RuntimeV3GameplayMessage>, RuntimeV3GameplayError> {
        self.queue
            .drain(budget)
            .into_iter()
            .map(|queued| self.execute_queued(queued))
            .collect()
    }

    #[must_use]
    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    #[must_use]
    pub fn receipt_len(&self) -> usize {
        self.receipts.len()
    }

    pub fn close(&mut self) {
        self.queue.close();
    }

    #[must_use]
    pub fn into_game(self) -> G {
        self.game
    }

    fn state(
        &self,
        request: &RuntimeV3GameplayMessage,
        reobserve: bool,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(request)?;
        let (observation, legal_actions) = self.checked_snapshot()?;
        Ok(self.observation_response(request, observation, legal_actions, reobserve))
    }

    fn legal_actions(
        &self,
        request: &RuntimeV3GameplayMessage,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(request)?;
        let (observation, legal_actions) = self.checked_snapshot()?;
        if request.state_id.as_deref() != Some(observation.state_id.as_str())
            || request.generation != observation.generation
        {
            return Err(RuntimeV3GameplayError::StaleGeneration);
        }
        let mut response = RuntimeV3GameplayMessage::base(
            context(request),
            observation.generation,
            RuntimeV3GameplayMessageKind::LegalActionsResponse,
        );
        response.state_id = Some(observation.state_id);
        response.legal_actions = Some(legal_actions);
        Ok(response)
    }

    fn admit_action(
        &mut self,
        request: RuntimeV3GameplayMessage,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(&request)?;
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
        let (observation, legal_actions) = self.checked_snapshot()?;
        if let Some(receipt) = self.receipts.get(operation_id) {
            if receipt.request == request {
                return Ok(receipt.response.clone());
            }
            return Ok(self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(observation),
                Some(legal_actions),
                None,
                Some("idempotency_conflict"),
                None,
            ));
        }
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
            Some(observation),
            Some(legal_actions),
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

    fn execute_queued(
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
        let dispatch_result = self.game.dispatch(&action.action);
        let after_result = self.checked_snapshot();
        let (after, after_actions) = match after_result {
            Ok(value) => value,
            Err(_) => {
                return Ok(self.finish_unknown(
                    &queued.operation_id,
                    &request,
                    "settlement_unproven",
                ));
            }
        };
        let response = match dispatch_result {
            Ok(()) if after.generation > before.generation => {
                let transition = RuntimeV3GameplayTransitionWitness {
                    from_generation: before.generation,
                    to_generation: after.generation,
                    state_id: after.state_id.clone(),
                    effect_kind: format!("{}.settled", action.action_id),
                };
                self.result_response(
                    &request,
                    RuntimeV3GameplayStatus::Settled,
                    Some(after),
                    Some(after_actions),
                    Some(transition),
                    None,
                    None,
                )
            }
            Err(error) if after == before => self.result_response(
                &request,
                RuntimeV3GameplayStatus::Rejected,
                Some(after),
                Some(after_actions),
                None,
                Some(game_error_code(error)),
                None,
            ),
            Ok(()) | Err(_) => self.result_response(
                &request,
                RuntimeV3GameplayStatus::Unknown,
                None,
                None,
                None,
                Some("settlement_unproven"),
                None,
            ),
        };
        Ok(self.finish(&queued.operation_id, response))
    }

    fn wait(
        &self,
        request: &RuntimeV3GameplayMessage,
    ) -> Result<RuntimeV3GameplayMessage, RuntimeV3GameplayError> {
        self.ensure_identity(request)?;
        let operation_id = request
            .operation_id
            .as_deref()
            .ok_or(RuntimeV3GameplayError::MalformedRequest)?;
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
                let mut response = receipt.response.clone();
                response.kind = RuntimeV3GameplayMessageKind::WaitResponse;
                response.wait_outcome = Some(RuntimeV3GameplayWaitOutcome::Successor);
                Ok(response)
            }
            Some(RuntimeV3GameplayStatus::Unknown) => {
                let mut response = receipt.response.clone();
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

    fn recover(
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
                let mut response = receipt.response.clone();
                response.kind = RuntimeV3GameplayMessageKind::RecoverResponse;
                Ok(response)
            }
            RuntimeV3GameplayRecoveryKind::ReleaseLease
            | RuntimeV3GameplayRecoveryKind::StopEpisode => {
                Err(RuntimeV3GameplayError::UnsupportedRecovery)
            }
        }
    }

    fn checked_snapshot(
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
        let legal_actions = self
            .game
            .legal_actions(&observation)
            .map_err(RuntimeV3GameplayError::Host)?;
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

    fn ensure_identity(
        &self,
        message: &RuntimeV3GameplayMessage,
    ) -> Result<(), RuntimeV3GameplayError> {
        if message.instance_id == self.identity.instance_id
            && message.session_id == self.identity.session_id
            && message.lease_id == self.identity.lease_id
            && message.lease_epoch == self.identity.lease_epoch
        {
            Ok(())
        } else {
            Err(RuntimeV3GameplayError::StaleIdentity)
        }
    }

    fn observation_response(
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

    fn result_response(
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

    fn finish(
        &mut self,
        operation_id: &str,
        response: RuntimeV3GameplayMessage,
    ) -> RuntimeV3GameplayMessage {
        if let Some(receipt) = self.receipts.get_mut(operation_id) {
            receipt.response = response.clone();
        }
        response
    }

    fn finish_unknown(
        &mut self,
        operation_id: &str,
        request: &RuntimeV3GameplayMessage,
        error_code: &str,
    ) -> RuntimeV3GameplayMessage {
        let response = self.result_response(
            request,
            RuntimeV3GameplayStatus::Unknown,
            None,
            None,
            None,
            Some(error_code),
            None,
        );
        self.finish(operation_id, response)
    }
}

fn context(message: &RuntimeV3GameplayMessage) -> RuntimeV3GameplayContext {
    RuntimeV3GameplayContext::new(
        message.correlation_id.clone(),
        message.instance_id.clone(),
        message.session_id.clone(),
        message.lease_id.clone(),
        message.lease_epoch,
    )
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn game_error_code(error: RuntimeV3GameplayGameError) -> &'static str {
    match error {
        RuntimeV3GameplayGameError::NotReady => "host_not_ready",
        RuntimeV3GameplayGameError::Rejected => "action_rejected",
        RuntimeV3GameplayGameError::ProjectionInvalid => "projection_invalid",
        RuntimeV3GameplayGameError::MutationUncertain => "settlement_unproven",
    }
}
