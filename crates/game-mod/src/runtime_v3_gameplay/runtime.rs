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

mod admission;
mod dispatch;
mod projection;
mod receipt;
mod recovery;

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
    admitted_observation: RuntimeV3GameplayObservation,
    admitted_actions: Vec<RuntimeV3GameplayLegalAction>,
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

fn correlated(
    receipt: &RuntimeV3GameplayMessage,
    request: &RuntimeV3GameplayMessage,
) -> RuntimeV3GameplayMessage {
    let mut response = receipt.clone();
    response.correlation_id.clone_from(&request.correlation_id);
    response
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
