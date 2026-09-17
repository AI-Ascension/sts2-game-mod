// SPDX-License-Identifier: MIT

//! Per-family structural parsers.

use super::super::super::canonical::CanonicalValue;
use super::super::error::CheckpointPayloadError as Error;
use super::super::families::*;
use super::{Fields, count, integer, list, text, texts, uint64};

pub(super) fn seed_and_rng(value: &CanonicalValue) -> Result<PayloadSeedAndRng, Error> {
    let mut fields = Fields::new(value, "seed_and_rng")?;
    let parsed = PayloadSeedAndRng {
        master_seed: text(fields.take("master_seed")?, "master_seed")?,
        derivation_version: text(fields.take("derivation_version")?, "derivation_version")?,
        streams: list(fields.take("streams")?, "streams", rng_stream)?,
    };
    fields.finish()?;
    Ok(parsed)
}

fn rng_stream(value: &CanonicalValue) -> Result<PayloadRngStream, Error> {
    let mut fields = Fields::new(value, "streams")?;
    let parsed = PayloadRngStream {
        stream_id: text(fields.take("stream_id")?, "stream_id")?,
        algorithm: text(fields.take("algorithm")?, "algorithm")?,
        cursor: uint64(fields.take("cursor")?, "cursor")?,
        state_words: list(fields.take("state_words")?, "state_words", |word| {
            uint64(word, "state_words")
        })?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn run_configuration(value: &CanonicalValue) -> Result<PayloadRunConfiguration, Error> {
    let mut fields = Fields::new(value, "run_configuration")?;
    let parsed = PayloadRunConfiguration {
        character: text(fields.take("character")?, "character")?,
        ascension: count(fields.take("ascension")?, "ascension")?,
        mode: text(fields.take("mode")?, "mode")?,
        act_sequence: texts(fields.take("act_sequence")?, "act_sequence")?,
        modifiers: texts(fields.take("modifiers")?, "modifiers")?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn campaign_progress(value: &CanonicalValue) -> Result<PayloadCampaignProgress, Error> {
    let mut fields = Fields::new(value, "campaign_progress")?;
    let parsed = PayloadCampaignProgress {
        act_index: count(fields.take("act_index")?, "act_index")?,
        floor: count(fields.take("floor")?, "floor")?,
        current_node: text(fields.take("current_node")?, "current_node")?,
        room_kind: text(fields.take("room_kind")?, "room_kind")?,
        visited_nodes: texts(fields.take("visited_nodes")?, "visited_nodes")?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn player_resources(value: &CanonicalValue) -> Result<PayloadPlayerResources, Error> {
    let mut fields = Fields::new(value, "player_resources")?;
    let parsed = PayloadPlayerResources {
        current_hp: count(fields.take("current_hp")?, "current_hp")?,
        max_hp: count(fields.take("max_hp")?, "max_hp")?,
        gold: count(fields.take("gold")?, "gold")?,
        keys: texts(fields.take("keys")?, "keys")?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn deck_and_piles(value: &CanonicalValue) -> Result<PayloadDeckAndPiles, Error> {
    let mut fields = Fields::new(value, "deck_and_piles")?;
    let parsed = PayloadDeckAndPiles {
        deck: texts(fields.take("deck")?, "deck")?,
        piles: list(fields.take("piles")?, "piles", |pile| {
            let mut fields = Fields::new(pile, "piles")?;
            let parsed = PayloadCardPile {
                pile_id: text(fields.take("pile_id")?, "pile_id")?,
                cards: texts(fields.take("cards")?, "cards")?,
            };
            fields.finish()?;
            Ok(parsed)
        })?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn card_instances(value: &CanonicalValue) -> Result<PayloadCardInstances, Error> {
    let mut fields = Fields::new(value, "card_instances")?;
    let parsed = PayloadCardInstances {
        cards: list(fields.take("cards")?, "cards", card_instance)?,
    };
    fields.finish()?;
    Ok(parsed)
}

fn card_instance(value: &CanonicalValue) -> Result<PayloadCardInstance, Error> {
    let mut fields = Fields::new(value, "cards")?;
    let parsed = PayloadCardInstance {
        instance_id: text(fields.take("instance_id")?, "instance_id")?,
        definition_id: text(fields.take("definition_id")?, "definition_id")?,
        upgrade_level: count(fields.take("upgrade_level")?, "upgrade_level")?,
        temporary_values: list(
            fields.take("temporary_values")?,
            "temporary_values",
            |temporary| {
                let mut fields = Fields::new(temporary, "temporary_values")?;
                let parsed = PayloadTemporaryValue {
                    key: text(fields.take("key")?, "key")?,
                    value: integer(fields.take("value")?, "value")?,
                };
                fields.finish()?;
                Ok(parsed)
            },
        )?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn relics(value: &CanonicalValue) -> Result<PayloadRelics, Error> {
    let mut fields = Fields::new(value, "relics")?;
    let parsed = PayloadRelics {
        relics: list(fields.take("relics")?, "relics", |relic| {
            let mut fields = Fields::new(relic, "relics")?;
            let parsed = PayloadRelic {
                relic_id: text(fields.take("relic_id")?, "relic_id")?,
                counter: integer(fields.take("counter")?, "counter")?,
            };
            fields.finish()?;
            Ok(parsed)
        })?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn potions(value: &CanonicalValue) -> Result<PayloadPotions, Error> {
    let mut fields = Fields::new(value, "potions")?;
    let parsed = PayloadPotions {
        capacity: count(fields.take("capacity")?, "capacity")?,
        slots: list(fields.take("slots")?, "slots", |slot| {
            let mut fields = Fields::new(slot, "slots")?;
            let parsed = PayloadPotionSlot {
                index: count(fields.take("index")?, "index")?,
                potion_id: text(fields.take("potion_id")?, "potion_id")?,
            };
            fields.finish()?;
            Ok(parsed)
        })?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn combat_turn(value: &CanonicalValue) -> Result<PayloadCombatTurn, Error> {
    let mut fields = Fields::new(value, "combat_turn")?;
    let parsed = PayloadCombatTurn {
        turn: count(fields.take("turn")?, "turn")?,
        energy: count(fields.take("energy")?, "energy")?,
        player_block: count(fields.take("player_block")?, "player_block")?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn powers(value: &CanonicalValue) -> Result<PayloadPowers, Error> {
    let mut fields = Fields::new(value, "powers")?;
    let parsed = PayloadPowers {
        powers: list(fields.take("powers")?, "powers", |power| {
            let mut fields = Fields::new(power, "powers")?;
            let parsed = PayloadPower {
                owner: text(fields.take("owner")?, "owner")?,
                power_id: text(fields.take("power_id")?, "power_id")?,
                amount: integer(fields.take("amount")?, "amount")?,
            };
            fields.finish()?;
            Ok(parsed)
        })?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn enemies_and_intents(value: &CanonicalValue) -> Result<PayloadEnemies, Error> {
    let mut fields = Fields::new(value, "enemies_and_intents")?;
    let parsed = PayloadEnemies {
        enemies: list(fields.take("enemies")?, "enemies", enemy)?,
    };
    fields.finish()?;
    Ok(parsed)
}

fn enemy(value: &CanonicalValue) -> Result<PayloadEnemy, Error> {
    let mut fields = Fields::new(value, "enemies")?;
    let parsed = PayloadEnemy {
        enemy_id: text(fields.take("enemy_id")?, "enemy_id")?,
        current_hp: count(fields.take("current_hp")?, "current_hp")?,
        max_hp: count(fields.take("max_hp")?, "max_hp")?,
        block: count(fields.take("block")?, "block")?,
        intents: list(fields.take("intents")?, "intents", |intent| {
            let mut fields = Fields::new(intent, "intents")?;
            let parsed = PayloadEnemyIntent {
                intent_id: text(fields.take("intent_id")?, "intent_id")?,
                value: integer(fields.take("value")?, "value")?,
            };
            fields.finish()?;
            Ok(parsed)
        })?,
    };
    fields.finish()?;
    Ok(parsed)
}

pub(super) fn pending_effects(value: &CanonicalValue) -> Result<PayloadPendingEffects, Error> {
    let mut fields = Fields::new(value, "pending_effects")?;
    let parsed = PayloadPendingEffects {
        pending_effects: texts(fields.take("pending_effects")?, "pending_effects")?,
        external_inputs: list(
            fields.take("external_inputs")?,
            "external_inputs",
            |input| {
                let mut fields = Fields::new(input, "external_inputs")?;
                let input_id = text(fields.take("input_id")?, "input_id")?;
                let mode = match text(fields.take("mode")?, "mode")?.as_str() {
                    "controlled" => PayloadExternalInputMode::Controlled,
                    "absent" => PayloadExternalInputMode::Absent,
                    _ => return Err(Error::InvalidType { field: "mode" }),
                };
                fields.finish()?;
                Ok(PayloadExternalInput { input_id, mode })
            },
        )?,
    };
    fields.finish()?;
    Ok(parsed)
}
