// SPDX-License-Identifier: MIT

use super::message::RuntimeV3GameplayMessage;
use super::*;

pub(super) fn validate_shape(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    match message.kind {
        RuntimeV3GameplayMessageKind::StateRequest
        | RuntimeV3GameplayMessageKind::ReobserveRequest => {
            require_shape(payload_is_empty(message))
        }
        RuntimeV3GameplayMessageKind::StateResponse
        | RuntimeV3GameplayMessageKind::ReobserveResponse => require_shape(
            message.state_id.is_some()
                && message.observation.is_some()
                && message.legal_actions.is_some()
                && no_state_result_fields(message)
                && observation_matches_envelope(message),
        ),
        RuntimeV3GameplayMessageKind::LegalActionsRequest => {
            require_shape(message.state_id.is_some() && only_state_id_fields(message))
        }
        RuntimeV3GameplayMessageKind::LegalActionsResponse => require_shape(
            message.state_id.is_some()
                && message.legal_actions.is_some()
                && no_action_result_fields(message),
        ),
        RuntimeV3GameplayMessageKind::DispatchActionRequest => require_shape(
            message.state_id.is_some()
                && message.operation_id.is_some()
                && message.action.is_some()
                && no_observation_result_fields(message),
        ),
        RuntimeV3GameplayMessageKind::DispatchActionResponse => {
            validate_non_wait_result_shape(message)
        }
        RuntimeV3GameplayMessageKind::WaitRequest => require_shape(
            message.operation_id.is_some()
                && message
                    .wait_for_millis
                    .is_some_and(|value| (1..=120_000).contains(&value))
                && message.state_id.is_none()
                && no_wait_request_fields(message),
        ),
        RuntimeV3GameplayMessageKind::WaitResponse => {
            if message.wait_outcome.is_some() {
                validate_result_shape(message)?;
                validate_wait_outcome(message)
            } else {
                Err(RuntimeV3GameplayValidationError::ResultShape)
            }
        }
        RuntimeV3GameplayMessageKind::RecoverRequest => require_shape(
            message.recovery.is_some()
                && message.state_id.is_none()
                && message.operation_id.is_none()
                && no_recovery_request_fields(message),
        ),
        RuntimeV3GameplayMessageKind::RecoverResponse => validate_non_wait_result_shape(message),
    }
}

fn payload_is_empty(message: &RuntimeV3GameplayMessage) -> bool {
    message.state_id.is_none()
        && message.operation_id.is_none()
        && message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_observation_result_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.observation.is_none()
        && message.legal_actions.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_action_result_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.operation_id.is_none()
        && message.observation.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_state_result_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.operation_id.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn no_recovery_request_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
}

fn no_wait_request_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.state_id.is_none()
        && message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn only_state_id_fields(message: &RuntimeV3GameplayMessage) -> bool {
    message.operation_id.is_none()
        && message.observation.is_none()
        && message.legal_actions.is_none()
        && message.action.is_none()
        && message.status.is_none()
        && message.transition.is_none()
        && message.error_code.is_none()
        && message.wait_for_millis.is_none()
        && message.wait_outcome.is_none()
        && message.recovery.is_none()
}

fn observation_matches_envelope(message: &RuntimeV3GameplayMessage) -> bool {
    let Some(observation) = &message.observation else {
        return false;
    };
    message.state_id.as_deref() == Some(observation.state_id.as_str())
        && observation.generation == message.generation
}

fn validate_result_shape(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    if message.operation_id.is_none()
        || message.action.is_some()
        || message.wait_for_millis.is_some()
        || message.recovery.is_some()
    {
        return Err(RuntimeV3GameplayValidationError::ResultShape);
    }
    match message.status {
        Some(RuntimeV3GameplayStatus::Settled) => require_shape(
            message.observation.is_some()
                && message.legal_actions.is_some()
                && message.transition.is_some()
                && message.error_code.is_none()
                && observation_matches_envelope(message)
                && transition_matches_envelope(message),
        ),
        Some(RuntimeV3GameplayStatus::Accepted) => require_shape(
            message.observation.is_some()
                && message.legal_actions.is_some()
                && message.error_code.is_none()
                && message.transition.is_none()
                && observation_matches_envelope(message),
        ),
        Some(RuntimeV3GameplayStatus::Rejected | RuntimeV3GameplayStatus::Cancelled) => {
            require_shape(
                message.observation.is_some()
                    && message.legal_actions.is_some()
                    && message.error_code.is_some()
                    && message.transition.is_none()
                    && observation_matches_envelope(message),
            )
        }
        Some(RuntimeV3GameplayStatus::Unknown) => require_shape(
            message.observation.is_none()
                && message.legal_actions.is_none()
                && message.transition.is_none()
                && message.error_code.is_some(),
        ),
        None => Err(RuntimeV3GameplayValidationError::ResultShape),
    }
}

fn validate_non_wait_result_shape(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    validate_result_shape(message)?;
    require_shape(message.wait_outcome.is_none())
}

fn transition_matches_envelope(message: &RuntimeV3GameplayMessage) -> bool {
    let Some(transition) = &message.transition else {
        return false;
    };
    transition.to_generation == message.generation
        && message.state_id.as_deref() == Some(transition.state_id.as_str())
}

fn validate_wait_outcome(
    message: &RuntimeV3GameplayMessage,
) -> Result<(), RuntimeV3GameplayValidationError> {
    match message.wait_outcome {
        Some(RuntimeV3GameplayWaitOutcome::Successor)
        | Some(RuntimeV3GameplayWaitOutcome::SameStateMutation) => {
            require_shape(message.status == Some(RuntimeV3GameplayStatus::Settled))
        }
        Some(RuntimeV3GameplayWaitOutcome::Timeout)
        | Some(RuntimeV3GameplayWaitOutcome::RecoveryRequired) => {
            require_shape(message.status == Some(RuntimeV3GameplayStatus::Unknown))
        }
        None => Err(RuntimeV3GameplayValidationError::ResultShape),
    }
}

fn require_shape(valid: bool) -> Result<(), RuntimeV3GameplayValidationError> {
    if valid {
        Ok(())
    } else {
        Err(RuntimeV3GameplayValidationError::ResultShape)
    }
}
