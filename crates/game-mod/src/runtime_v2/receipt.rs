// SPDX-License-Identifier: MIT

use super::contract::{
    RuntimeV2Action, RuntimeV2EffectWitness, RuntimeV2Message, RuntimeV2Observation,
    RuntimeV2Status,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct QueuedOperation {
    pub(super) operation_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SettledOutcome {
    pub(super) observation: RuntimeV2Observation,
    pub(super) witness: RuntimeV2EffectWitness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum OperationState {
    Pending {
        accepted: Box<RuntimeV2Message>,
    },
    Settled(SettledOutcome),
    Rejected {
        observation: RuntimeV2Observation,
        error_code: String,
    },
    Unknown {
        error_code: String,
        settled: Option<SettledOutcome>,
    },
    Cancelled {
        observation: RuntimeV2Observation,
        error_code: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct OperationReceipt {
    pub(super) request: RuntimeV2Message,
    pub(super) state: OperationState,
}

impl OperationReceipt {
    pub(super) fn is_pending(&self) -> bool {
        matches!(self.state, OperationState::Pending { .. })
    }

    pub(super) fn action_replay(&self) -> RuntimeV2Message {
        let request = &self.request;
        match &self.state {
            OperationState::Pending { accepted } => accepted.as_ref().clone(),
            OperationState::Settled(outcome) => RuntimeV2Message::action_response(
                request,
                RuntimeV2Status::Settled,
                outcome.observation.generation,
                Some(outcome.observation),
                Some(outcome.witness.clone()),
                None,
            ),
            OperationState::Rejected {
                observation,
                error_code,
            } => RuntimeV2Message::action_response(
                request,
                RuntimeV2Status::Rejected,
                observation.generation,
                Some(*observation),
                None,
                Some(error_code),
            ),
            OperationState::Unknown { error_code, .. } => RuntimeV2Message::action_response(
                request,
                RuntimeV2Status::Unknown,
                request.generation,
                None,
                None,
                Some(error_code),
            ),
            OperationState::Cancelled {
                observation,
                error_code,
            } => RuntimeV2Message::action_response(
                request,
                RuntimeV2Status::Cancelled,
                observation.generation,
                Some(*observation),
                None,
                Some(error_code),
            ),
        }
    }

    pub(super) fn reconcile(&self, request: &RuntimeV2Message) -> RuntimeV2Message {
        let fallback_action = RuntimeV2Action::end_turn();
        let action = self.request.action.as_ref().unwrap_or(&fallback_action);
        match &self.state {
            OperationState::Pending { accepted } => RuntimeV2Message::reconcile_response(
                request,
                action,
                RuntimeV2Status::Accepted,
                accepted.generation,
                accepted.observation,
                None,
                None,
            ),
            OperationState::Settled(outcome) => RuntimeV2Message::reconcile_response(
                request,
                action,
                RuntimeV2Status::Settled,
                outcome.observation.generation,
                Some(outcome.observation),
                Some(outcome.witness.clone()),
                None,
            ),
            OperationState::Rejected {
                observation,
                error_code,
            } => RuntimeV2Message::reconcile_response(
                request,
                action,
                RuntimeV2Status::Rejected,
                observation.generation,
                Some(*observation),
                None,
                Some(error_code),
            ),
            OperationState::Unknown {
                settled: Some(outcome),
                ..
            } => RuntimeV2Message::reconcile_response(
                request,
                action,
                RuntimeV2Status::Settled,
                outcome.observation.generation,
                Some(outcome.observation),
                Some(outcome.witness.clone()),
                None,
            ),
            OperationState::Unknown {
                error_code,
                settled: None,
            } => RuntimeV2Message::reconcile_response(
                request,
                action,
                RuntimeV2Status::Unknown,
                self.request.generation,
                None,
                None,
                Some(error_code),
            ),
            OperationState::Cancelled {
                observation,
                error_code,
            } => RuntimeV2Message::reconcile_response(
                request,
                action,
                RuntimeV2Status::Cancelled,
                observation.generation,
                Some(*observation),
                None,
                Some(error_code),
            ),
        }
    }
}
