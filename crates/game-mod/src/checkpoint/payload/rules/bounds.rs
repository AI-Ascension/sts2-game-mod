// SPDX-License-Identifier: MIT

//! Per-family identifier, range, and collection bounds.

use std::collections::BTreeSet;

use super::super::super::CANONICAL_MAX_SAFE_INTEGER;
use super::super::super::capability::CheckpointBoundary;
use super::super::error::CheckpointPayloadError as Error;
use super::super::families::*;

fn identifier(field: &'static str, value: &str) -> Result<(), Error> {
    let grammar =
        |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-');
    if value.is_empty() || value.len() > PAYLOAD_MAX_IDENTIFIER_BYTES || !value.bytes().all(grammar)
    {
        return Err(Error::InvalidIdentifier { field });
    }
    Ok(())
}

fn identifiers(field: &'static str, values: &[String], max: usize) -> Result<(), Error> {
    bound(field, values.len(), 0, max)?;
    values.iter().try_for_each(|value| identifier(field, value))
}

fn count(field: &'static str, value: u64) -> Result<(), Error> {
    if value > CANONICAL_MAX_SAFE_INTEGER as u64 {
        return Err(Error::OutOfRange { field });
    }
    Ok(())
}

fn integer(field: &'static str, value: i64) -> Result<(), Error> {
    if value.unsigned_abs() > CANONICAL_MAX_SAFE_INTEGER as u64 {
        return Err(Error::OutOfRange { field });
    }
    Ok(())
}

fn bound(field: &'static str, len: usize, min: usize, max: usize) -> Result<(), Error> {
    if len < min || len > max {
        return Err(Error::BoundExceeded { field });
    }
    Ok(())
}

pub(super) fn seed_and_rng(value: &PayloadSeedAndRng) -> Result<(), Error> {
    identifier("master_seed", &value.master_seed)?;
    identifier("derivation_version", &value.derivation_version)?;
    bound("streams", value.streams.len(), 1, PAYLOAD_MAX_RNG_STREAMS)?;
    let mut seen = BTreeSet::new();
    for stream in &value.streams {
        identifier("stream_id", &stream.stream_id)?;
        identifier("algorithm", &stream.algorithm)?;
        bound(
            "state_words",
            stream.state_words.len(),
            1,
            PAYLOAD_MAX_RNG_STATE_WORDS,
        )?;
        if !seen.insert(stream.stream_id.as_str()) {
            return Err(Error::DuplicateStreamId);
        }
    }
    Ok(())
}

pub(super) fn run_configuration(value: &PayloadRunConfiguration) -> Result<(), Error> {
    identifier("character", &value.character)?;
    count("ascension", value.ascension)?;
    identifier("mode", &value.mode)?;
    bound(
        "act_sequence",
        value.act_sequence.len(),
        1,
        PAYLOAD_MAX_ACTS,
    )?;
    identifiers("act_sequence", &value.act_sequence, PAYLOAD_MAX_ACTS)?;
    identifiers("modifiers", &value.modifiers, PAYLOAD_MAX_MODIFIERS)
}

pub(super) fn campaign_progress(value: &PayloadCampaignProgress) -> Result<(), Error> {
    count("act_index", value.act_index)?;
    count("floor", value.floor)?;
    identifier("current_node", &value.current_node)?;
    identifier("room_kind", &value.room_kind)?;
    identifiers(
        "visited_nodes",
        &value.visited_nodes,
        PAYLOAD_MAX_VISITED_NODES,
    )
}

pub(super) fn player_resources(value: &PayloadPlayerResources) -> Result<(), Error> {
    count("current_hp", value.current_hp)?;
    count("max_hp", value.max_hp)?;
    count("gold", value.gold)?;
    identifiers("keys", &value.keys, PAYLOAD_MAX_KEYS)
}

pub(super) fn deck_and_piles(value: &PayloadDeckAndPiles) -> Result<(), Error> {
    identifiers("deck", &value.deck, PAYLOAD_MAX_CARDS)?;
    bound("piles", value.piles.len(), 0, PAYLOAD_MAX_PILES)?;
    for pile in &value.piles {
        identifier("pile_id", &pile.pile_id)?;
        identifiers("cards", &pile.cards, PAYLOAD_MAX_CARDS)?;
    }
    Ok(())
}

pub(super) fn card_instances(value: &PayloadCardInstances) -> Result<(), Error> {
    bound("cards", value.cards.len(), 0, PAYLOAD_MAX_CARDS)?;
    let mut seen = BTreeSet::new();
    for card in &value.cards {
        identifier("instance_id", &card.instance_id)?;
        identifier("definition_id", &card.definition_id)?;
        count("upgrade_level", card.upgrade_level)?;
        bound(
            "temporary_values",
            card.temporary_values.len(),
            0,
            PAYLOAD_MAX_TEMPORARY_VALUES,
        )?;
        for temporary in &card.temporary_values {
            identifier("key", &temporary.key)?;
            integer("value", temporary.value)?;
        }
        if !seen.insert(card.instance_id.as_str()) {
            return Err(Error::DuplicateCardInstance);
        }
    }
    Ok(())
}

pub(super) fn relics(value: &PayloadRelics) -> Result<(), Error> {
    bound("relics", value.relics.len(), 0, PAYLOAD_MAX_RELICS)?;
    for relic in &value.relics {
        identifier("relic_id", &relic.relic_id)?;
        integer("counter", relic.counter)?;
    }
    Ok(())
}

pub(super) fn potions(value: &PayloadPotions) -> Result<(), Error> {
    count("capacity", value.capacity)?;
    bound("slots", value.slots.len(), 0, PAYLOAD_MAX_POTION_SLOTS)?;
    for slot in &value.slots {
        count("index", slot.index)?;
        identifier("potion_id", &slot.potion_id)?;
    }
    Ok(())
}

pub(super) fn combat_turn(
    boundary: CheckpointBoundary,
    value: &PayloadCombatTurn,
) -> Result<(), Error> {
    count("turn", value.turn)?;
    count("energy", value.energy)?;
    count("player_block", value.player_block)?;
    let minimum_turn = match boundary {
        CheckpointBoundary::LaterTurnCombat => 2,
        _ => 1,
    };
    if value.turn < minimum_turn {
        return Err(Error::TurnWitnessMismatch);
    }
    Ok(())
}

pub(super) fn powers(value: &PayloadPowers) -> Result<(), Error> {
    bound("powers", value.powers.len(), 0, PAYLOAD_MAX_POWERS)?;
    for power in &value.powers {
        identifier("owner", &power.owner)?;
        identifier("power_id", &power.power_id)?;
        integer("amount", power.amount)?;
    }
    Ok(())
}

pub(super) fn enemies_and_intents(value: &PayloadEnemies) -> Result<(), Error> {
    bound("enemies", value.enemies.len(), 0, PAYLOAD_MAX_ENEMIES)?;
    for enemy in &value.enemies {
        identifier("enemy_id", &enemy.enemy_id)?;
        count("current_hp", enemy.current_hp)?;
        count("max_hp", enemy.max_hp)?;
        count("block", enemy.block)?;
        bound("intents", enemy.intents.len(), 0, PAYLOAD_MAX_INTENTS)?;
        for intent in &enemy.intents {
            identifier("intent_id", &intent.intent_id)?;
            integer("value", intent.value)?;
        }
    }
    Ok(())
}

pub(super) fn pending_effects(value: &PayloadPendingEffects) -> Result<(), Error> {
    if !value.pending_effects.is_empty() {
        return Err(Error::PendingEffectsOutstanding);
    }
    bound(
        "external_inputs",
        value.external_inputs.len(),
        0,
        PAYLOAD_MAX_EXTERNAL_INPUTS,
    )?;
    value
        .external_inputs
        .iter()
        .try_for_each(|input| identifier("input_id", &input.input_id))
}
