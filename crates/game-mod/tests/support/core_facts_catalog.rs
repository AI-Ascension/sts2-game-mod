// SPDX-License-Identifier: MIT

//! Independently written synthetic signatures; no production catalog constructs these fixtures.

use sts2_game_mod::{
    FactsEvidenceStatus, FactsInputAvailability, FactsInputSource, FactsRuleEntry, FactsRuleInput,
    FactsSourceKind,
};

pub const SIGNATURES: &[(&str, &[(&str, &str)])] = &[
    (
        "damage.nominal_card",
        &[
            ("card_damage", "damage"),
            ("hit_count", "count"),
            ("target", "entity"),
        ],
    ),
    (
        "damage.synthetic_modifier_ordering",
        &[
            ("base_damage", "damage"),
            ("flat_bonus", "damage"),
            ("multiplier", "multiplier"),
            ("hit_count", "count"),
        ],
    ),
    (
        "resource.fixed_card_cost",
        &[("energy", "energy"), ("card_cost", "energy")],
    ),
    (
        "block.incoming_after_block",
        &[("incoming_damage", "damage"), ("player_block", "block")],
    ),
    (
        "heal.saturating_recovery",
        &[
            ("current_hp", "hit_points"),
            ("heal_amount", "hit_points"),
            ("max_hp", "hit_points"),
        ],
    ),
    (
        "card_movement.draw_to_hand_size",
        &[
            ("hand_size", "count"),
            ("max_hand_size", "count"),
            ("draw_pile_size", "count"),
        ],
    ),
    (
        "card_movement.exhaust_removes_from_zone",
        &[
            ("exhausted_cards", "count"),
            ("discard_pile_size", "count"),
            ("exhaust_pile_size", "count"),
        ],
    ),
    (
        "turn_timing.start_of_turn_energy_refill",
        &[
            ("energy_before", "energy"),
            ("base_energy", "energy"),
            ("energy_cap", "energy"),
        ],
    ),
    (
        "turn_timing.block_expiry_at_turn_start",
        &[("block_before", "block"), ("expiring_block", "block")],
    ),
    (
        "acquisition.reward_choice_picks",
        &[("offered_cards", "count"), ("picks", "count")],
    ),
    (
        "difficulty_scaling.ascension_enemy_hp",
        &[
            ("base_hp", "hit_points"),
            ("scaling_steps", "count"),
            ("scaling_numerator", "ratio"),
            ("scaling_denominator", "ratio"),
        ],
    ),
    (
        "coop_scaling.enemy_hp_by_player_count",
        &[
            ("base_hp", "hit_points"),
            ("player_count", "count"),
            ("hp_share_numerator", "ratio"),
            ("hp_share_denominator", "ratio"),
        ],
    ),
    (
        "card_interaction.multi_hit_ordering",
        &[
            ("base_damage", "damage"),
            ("flat_bonus", "damage"),
            ("multiplier_numerator", "multiplier"),
            ("multiplier_denominator", "multiplier"),
            ("hit_count", "count"),
        ],
    ),
    (
        "relic_interaction.flat_bonus_ordering",
        &[
            ("base_damage", "damage"),
            ("multiplier_numerator", "multiplier"),
            ("multiplier_denominator", "multiplier"),
            ("flat_bonus", "damage"),
        ],
    ),
    (
        "potion_interaction.consumption_budget",
        &[("slots_held", "count"), ("slot_capacity", "count")],
    ),
    (
        "status_interaction.vulnerable_multiplier",
        &[
            ("incoming_damage", "damage"),
            ("status_numerator", "multiplier"),
            ("status_denominator", "multiplier"),
        ],
    ),
];

pub fn rules() -> Vec<FactsRuleEntry> {
    SIGNATURES
        .iter()
        .enumerate()
        .map(|(rule_index, (id, inputs))| FactsRuleEntry {
            rule_id: (*id).to_owned(),
            evidence: FactsEvidenceStatus::SourceDerived,
            inputs: inputs
                .iter()
                .enumerate()
                .map(|(input_index, (name, unit))| FactsRuleInput {
                    name: (*name).to_owned(),
                    unit: (*unit).to_owned(),
                    availability: FactsInputAvailability::Required,
                    source: Some(FactsInputSource {
                        kind: FactsSourceKind::GameMod,
                        reference: format!("synthetic.r{rule_index}.i{input_index}"),
                    }),
                })
                .collect(),
        })
        .collect()
}
