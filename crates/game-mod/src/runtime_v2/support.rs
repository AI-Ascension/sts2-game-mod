// SPDX-License-Identifier: MIT

use super::artifact::{RUNTIME_V2_MAX_GENERATION, RUNTIME_V2_MAX_TURN_INDEX};
use super::contract::{
    RuntimeV2Action, RuntimeV2CombatPhase, RuntimeV2Message, RuntimeV2Observation, RuntimeV2Status,
    RuntimeV2ValidationError,
};
use super::fake::RuntimeV2GameError;
use super::types::RuntimeV2Error;

pub(super) fn request_validation_error(error: RuntimeV2ValidationError) -> RuntimeV2Error {
    match error {
        RuntimeV2ValidationError::Metadata | RuntimeV2ValidationError::Provenance => {
            RuntimeV2Error::ArtifactMismatch
        }
        _ => RuntimeV2Error::MalformedRequest,
    }
}

pub(super) fn legality_error(observation: RuntimeV2Observation) -> Option<&'static str> {
    if !observation.host_ready {
        return Some("sts2.game-mod/host_not_ready");
    }
    match observation.combat_phase {
        RuntimeV2CombatPhase::OutsideCombat => Some("sts2.game-core/outside_combat"),
        RuntimeV2CombatPhase::EnemyTurn => Some("sts2.game-core/not_player_turn"),
        RuntimeV2CombatPhase::PlayerTurn => None,
    }
}

pub(super) fn game_error_code(error: RuntimeV2GameError) -> &'static str {
    match error {
        RuntimeV2GameError::NotReady => "sts2.game-mod/host_not_ready",
        RuntimeV2GameError::Rejected => "sts2.game-core/rejected",
    }
}

pub(super) fn settled_transition_is_valid(
    before: RuntimeV2Observation,
    after: RuntimeV2Observation,
) -> bool {
    before.combat_phase == RuntimeV2CombatPhase::PlayerTurn
        && after.combat_phase == RuntimeV2CombatPhase::PlayerTurn
        && before.generation.checked_add(1) == Some(after.generation)
        && before.turn_index.checked_add(1) == Some(after.turn_index)
        && after.turn_index <= RUNTIME_V2_MAX_TURN_INDEX
        && after.generation <= RUNTIME_V2_MAX_GENERATION
}

pub(super) fn rejected_action(
    request: &RuntimeV2Message,
    observation: RuntimeV2Observation,
    error_code: &str,
) -> RuntimeV2Message {
    RuntimeV2Message::action_response(
        request,
        RuntimeV2Status::Rejected,
        observation.generation,
        Some(observation),
        None,
        Some(error_code),
    )
}

pub(super) fn rejected_reconcile(
    request: &RuntimeV2Message,
    observation: RuntimeV2Observation,
    error_code: &str,
) -> RuntimeV2Message {
    RuntimeV2Message::reconcile_response(
        request,
        &RuntimeV2Action::end_turn(),
        RuntimeV2Status::Rejected,
        observation.generation,
        Some(observation),
        None,
        Some(error_code),
    )
}
