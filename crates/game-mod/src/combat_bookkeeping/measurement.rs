// SPDX-License-Identifier: MIT

use super::{
    CombatBookkeepingBinding, CombatCardInstanceReference, CombatCounterProvenance, CombatField,
    CombatPending, CombatResolutionState, CombatSnapshotInput,
};

const REFERENCE_OVERHEAD_BYTES: usize = 128;

pub(super) fn snapshot_bytes(input: &CombatSnapshotInput) -> usize {
    let mut bytes = 0;
    add_binding_bytes(&mut bytes, &input.binding);
    if let CombatField::Available(turn) = &input.turn {
        add_text_bytes(&mut bytes, &turn.round_id);
        add_text_bytes(&mut bytes, &turn.player_turn_id);
    }
    for zone in &input.zones {
        if let CombatField::Available(cards) = &zone.composition {
            for card in cards {
                add_card_bytes(&mut bytes, &card.card);
            }
        }
    }
    for counter in input.counters.all() {
        if let CombatCounterProvenance::SemanticHistory { history_id, .. } = &counter.provenance {
            add_text_bytes(&mut bytes, history_id);
        }
    }
    if let CombatField::Available(CombatResolutionState::Resolving { effect_id }) =
        &input.resolution
    {
        add_text_bytes(&mut bytes, effect_id);
    }
    if let CombatField::Available(pending) = &input.pending {
        match pending {
            CombatPending::None => {}
            CombatPending::Selection {
                selection_id,
                kind,
                choices,
            } => {
                add_text_bytes(&mut bytes, selection_id);
                add_text_bytes(&mut bytes, kind);
                for card in choices {
                    add_card_bytes(&mut bytes, card);
                }
            }
            CombatPending::Effect {
                effect_id,
                kind,
                target_ids,
            } => {
                add_text_bytes(&mut bytes, effect_id);
                add_text_bytes(&mut bytes, kind);
                for target_id in target_ids {
                    add_reference_bytes(&mut bytes);
                    add_text_bytes(&mut bytes, target_id);
                }
            }
        }
    }
    bytes
}

fn add_binding_bytes(bytes: &mut usize, binding: &CombatBookkeepingBinding) {
    for value in [
        &binding.content_manifest,
        &binding.game_instance_id,
        &binding.run_id,
        &binding.combat_id,
        &binding.snapshot_id,
    ] {
        add_text_bytes(bytes, value);
    }
}

fn add_card_bytes(bytes: &mut usize, card: &CombatCardInstanceReference) {
    add_reference_bytes(bytes);
    add_binding_bytes(bytes, &card.binding);
    add_text_bytes(bytes, &card.instance_id);
    add_text_bytes(bytes, &card.definition_id);
    add_text_bytes(bytes, &card.owner_id);
}

fn add_reference_bytes(bytes: &mut usize) {
    *bytes = bytes.saturating_add(REFERENCE_OVERHEAD_BYTES);
}

fn add_text_bytes(bytes: &mut usize, value: &str) {
    *bytes = bytes.saturating_add(value.len());
}
