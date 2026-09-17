// SPDX-License-Identifier: MIT

//! Lowering of the typed payload to the restricted canonical value model.

use std::collections::BTreeMap;

use super::super::canonical::CanonicalValue;
use super::families::*;
use super::{CHECKPOINT_PAYLOAD_SCHEMA, CheckpointPayload, PayloadFamilyCoverage};

type Entry = (&'static str, CanonicalValue);

fn object(entries: impl IntoIterator<Item = Entry>) -> CanonicalValue {
    let mut map = BTreeMap::new();
    for (key, value) in entries {
        map.insert(key.to_owned(), value);
    }
    CanonicalValue::Object(map)
}

fn text(value: &str) -> CanonicalValue {
    CanonicalValue::Text(value.to_owned())
}

/// Counts are validated to the safe range; an out-of-range value is lowered to
/// an integer the canonical encoder rejects, never to a substituted value.
fn count(value: u64) -> CanonicalValue {
    CanonicalValue::Integer(i64::try_from(value).unwrap_or(i64::MAX))
}

fn integer(value: i64) -> CanonicalValue {
    CanonicalValue::Integer(value)
}

fn uint64(value: u64) -> CanonicalValue {
    CanonicalValue::Uint64(value)
}

fn texts(values: &[String]) -> CanonicalValue {
    CanonicalValue::Array(values.iter().map(|value| text(value)).collect())
}

fn list<T>(values: &[T], lower_item: impl Fn(&T) -> CanonicalValue) -> CanonicalValue {
    CanonicalValue::Array(values.iter().map(lower_item).collect())
}

fn family<T>(
    coverage: &PayloadFamilyCoverage<T>,
    lower_value: impl FnOnce(&T) -> CanonicalValue,
) -> CanonicalValue {
    let mut entries = vec![("coverage", text(coverage.token()))];
    if let PayloadFamilyCoverage::Captured(value) = coverage {
        entries.push(("value", lower_value(value)));
    }
    object(entries)
}

pub(super) fn lower(payload: &CheckpointPayload) -> CanonicalValue {
    let families = payload.families();
    object([
        ("schema", text(CHECKPOINT_PAYLOAD_SCHEMA)),
        (
            "canonical_profile",
            text(super::super::CHECKPOINT_CAPTURE_PROFILE),
        ),
        ("boundary", text(payload.boundary().code())),
        (
            "families",
            object([
                ("seed_and_rng", family(&families.seed_and_rng, seed_and_rng)),
                (
                    "run_configuration",
                    family(&families.run_configuration, run_configuration),
                ),
                (
                    "campaign_progress",
                    family(&families.campaign_progress, campaign_progress),
                ),
                (
                    "player_resources",
                    family(&families.player_resources, player_resources),
                ),
                (
                    "deck_and_piles",
                    family(&families.deck_and_piles, deck_and_piles),
                ),
                (
                    "card_instances",
                    family(&families.card_instances, card_instances),
                ),
                ("relics", family(&families.relics, relics)),
                ("potions", family(&families.potions, potions)),
                ("combat_turn", family(&families.combat_turn, combat_turn)),
                ("powers", family(&families.powers, powers)),
                (
                    "enemies_and_intents",
                    family(&families.enemies_and_intents, enemies_and_intents),
                ),
                (
                    "pending_effects",
                    family(&families.pending_effects, pending_effects),
                ),
            ]),
        ),
    ])
}

fn seed_and_rng(value: &PayloadSeedAndRng) -> CanonicalValue {
    object([
        ("master_seed", text(&value.master_seed)),
        ("derivation_version", text(&value.derivation_version)),
        (
            "streams",
            list(&value.streams, |stream| {
                object([
                    ("stream_id", text(&stream.stream_id)),
                    ("algorithm", text(&stream.algorithm)),
                    ("cursor", uint64(stream.cursor)),
                    (
                        "state_words",
                        list(&stream.state_words, |word| uint64(*word)),
                    ),
                ])
            }),
        ),
    ])
}

fn run_configuration(value: &PayloadRunConfiguration) -> CanonicalValue {
    object([
        ("character", text(&value.character)),
        ("ascension", count(value.ascension)),
        ("mode", text(&value.mode)),
        ("act_sequence", texts(&value.act_sequence)),
        ("modifiers", texts(&value.modifiers)),
    ])
}

fn campaign_progress(value: &PayloadCampaignProgress) -> CanonicalValue {
    object([
        ("act_index", count(value.act_index)),
        ("floor", count(value.floor)),
        ("current_node", text(&value.current_node)),
        ("room_kind", text(&value.room_kind)),
        ("visited_nodes", texts(&value.visited_nodes)),
    ])
}

fn player_resources(value: &PayloadPlayerResources) -> CanonicalValue {
    object([
        ("current_hp", count(value.current_hp)),
        ("max_hp", count(value.max_hp)),
        ("gold", count(value.gold)),
        ("keys", texts(&value.keys)),
    ])
}

fn deck_and_piles(value: &PayloadDeckAndPiles) -> CanonicalValue {
    object([
        ("deck", texts(&value.deck)),
        (
            "piles",
            list(&value.piles, |pile| {
                object([
                    ("pile_id", text(&pile.pile_id)),
                    ("cards", texts(&pile.cards)),
                ])
            }),
        ),
    ])
}

fn card_instances(value: &PayloadCardInstances) -> CanonicalValue {
    object([(
        "cards",
        list(&value.cards, |card| {
            object([
                ("instance_id", text(&card.instance_id)),
                ("definition_id", text(&card.definition_id)),
                ("upgrade_level", count(card.upgrade_level)),
                (
                    "temporary_values",
                    list(&card.temporary_values, |temporary| {
                        object([
                            ("key", text(&temporary.key)),
                            ("value", integer(temporary.value)),
                        ])
                    }),
                ),
            ])
        }),
    )])
}

fn relics(value: &PayloadRelics) -> CanonicalValue {
    object([(
        "relics",
        list(&value.relics, |relic| {
            object([
                ("relic_id", text(&relic.relic_id)),
                ("counter", integer(relic.counter)),
            ])
        }),
    )])
}

fn potions(value: &PayloadPotions) -> CanonicalValue {
    object([
        ("capacity", count(value.capacity)),
        (
            "slots",
            list(&value.slots, |slot| {
                object([
                    ("index", count(slot.index)),
                    ("potion_id", text(&slot.potion_id)),
                ])
            }),
        ),
    ])
}

fn combat_turn(value: &PayloadCombatTurn) -> CanonicalValue {
    object([
        ("turn", count(value.turn)),
        ("energy", count(value.energy)),
        ("player_block", count(value.player_block)),
    ])
}

fn powers(value: &PayloadPowers) -> CanonicalValue {
    object([(
        "powers",
        list(&value.powers, |power| {
            object([
                ("owner", text(&power.owner)),
                ("power_id", text(&power.power_id)),
                ("amount", integer(power.amount)),
            ])
        }),
    )])
}

fn enemies_and_intents(value: &PayloadEnemies) -> CanonicalValue {
    object([(
        "enemies",
        list(&value.enemies, |enemy| {
            object([
                ("enemy_id", text(&enemy.enemy_id)),
                ("current_hp", count(enemy.current_hp)),
                ("max_hp", count(enemy.max_hp)),
                ("block", count(enemy.block)),
                (
                    "intents",
                    list(&enemy.intents, |intent| {
                        object([
                            ("intent_id", text(&intent.intent_id)),
                            ("value", integer(intent.value)),
                        ])
                    }),
                ),
            ])
        }),
    )])
}

fn pending_effects(value: &PayloadPendingEffects) -> CanonicalValue {
    object([
        ("pending_effects", texts(&value.pending_effects)),
        (
            "external_inputs",
            list(&value.external_inputs, |input| {
                object([
                    ("input_id", text(&input.input_id)),
                    ("mode", text(input.mode.token())),
                ])
            }),
        ),
    ])
}
