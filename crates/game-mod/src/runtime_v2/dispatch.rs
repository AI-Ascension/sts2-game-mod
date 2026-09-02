// SPDX-License-Identifier: MIT

use super::contract::{RuntimeV2Message, RuntimeV2Observation};
use super::fake::RuntimeV2GamePort;
use super::receipt::{OperationState, QueuedOperation, SettledOutcome};
use super::support::{game_error_code, legality_error, settled_transition_is_valid};
use super::types::{RuntimeV2Error, RuntimeV2Mod};

impl<G: RuntimeV2GamePort> RuntimeV2Mod<G> {
    pub(super) fn execute_queued(
        &mut self,
        queued: QueuedOperation,
        disconnect_after_write: bool,
    ) -> Result<RuntimeV2Message, RuntimeV2Error> {
        let index = self
            .receipt_index(&queued.operation_id)
            .ok_or(RuntimeV2Error::OperationNotFound)?;
        if !self.receipts[index].is_pending() {
            return Ok(self.receipts[index].action_replay());
        }
        let request = self.receipts[index].request.clone();
        let before = match self.checked_snapshot() {
            Ok(observation) => observation,
            Err(_) => {
                return Ok(self.finish_unknown(index, "sts2.runtime/settlement_unproven", None));
            }
        };
        if !self.identity.matches(&request) {
            return Ok(self.finish_rejected(index, before, "stale_identity"));
        }
        if request.generation != before.generation {
            return Ok(self.finish_rejected(index, before, "sts2.game-core/stale_generation"));
        }
        if let Some(error_code) = legality_error(before) {
            return Ok(self.finish_rejected(index, before, error_code));
        }
        let result = self.game.end_turn();
        let after = self.game.snapshot();
        if after.validate().is_err() {
            return Ok(self.finish_unknown(index, "sts2.runtime/settlement_unproven", None));
        }
        match result {
            Err(error) if after == before => {
                Ok(self.finish_rejected(index, before, game_error_code(error)))
            }
            Err(_) => Ok(self.finish_unknown(index, "sts2.runtime/unknown_after_reject", None)),
            Ok(()) if settled_transition_is_valid(before, after) => {
                let outcome = SettledOutcome {
                    observation: after,
                    witness: super::contract::RuntimeV2EffectWitness::turn_end_settled(
                        after.generation,
                    ),
                };
                if disconnect_after_write {
                    Ok(self.finish_unknown(
                        index,
                        "sts2.runtime/unknown_after_disconnect",
                        Some(outcome),
                    ))
                } else {
                    Ok(self.finish_settled(index, outcome))
                }
            }
            Ok(()) => Ok(self.finish_unknown(index, "sts2.runtime/settlement_unproven", None)),
        }
    }

    fn finish_settled(&mut self, index: usize, outcome: SettledOutcome) -> RuntimeV2Message {
        self.receipts[index].state = OperationState::Settled(outcome);
        self.receipts[index].action_replay()
    }

    fn finish_rejected(
        &mut self,
        index: usize,
        observation: RuntimeV2Observation,
        error_code: &str,
    ) -> RuntimeV2Message {
        self.receipts[index].state = OperationState::Rejected {
            observation,
            error_code: error_code.to_owned(),
        };
        self.receipts[index].action_replay()
    }

    fn finish_unknown(
        &mut self,
        index: usize,
        error_code: &str,
        settled: Option<SettledOutcome>,
    ) -> RuntimeV2Message {
        self.receipts[index].state = OperationState::Unknown {
            error_code: error_code.to_owned(),
            settled,
        };
        self.receipts[index].action_replay()
    }

    pub(super) fn checked_snapshot(&self) -> Result<RuntimeV2Observation, RuntimeV2Error> {
        let observation = self.game.snapshot();
        observation
            .validate()
            .map_err(|_| RuntimeV2Error::InvalidObservation)?;
        Ok(observation)
    }

    pub(super) fn receipt_index(&self, operation_id: &str) -> Option<usize> {
        self.receipts
            .iter()
            .position(|receipt| receipt.request.operation_id.as_deref() == Some(operation_id))
    }
}
