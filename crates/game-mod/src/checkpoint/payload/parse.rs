// SPDX-License-Identifier: MIT

//! Strict structural parse from the canonical value model.
//!
//! This layer checks shape and types only; bounds, coverage rules, and
//! cross-family consistency are applied by `rules` through
//! [`CheckpointPayload::new`](super::CheckpointPayload::new).

mod families;

use std::collections::BTreeMap;

use super::super::canonical::CanonicalValue;
use super::super::capability::CheckpointBoundary;
use super::error::CheckpointPayloadError as Error;
use super::{CHECKPOINT_PAYLOAD_SCHEMA, CheckpointPayloadFamilies, PayloadFamilyCoverage};

/// Reads declared fields of one closed object and refuses any other field.
pub(super) struct Fields<'a> {
    within: &'static str,
    map: &'a BTreeMap<String, CanonicalValue>,
    taken: usize,
}

impl<'a> Fields<'a> {
    pub(super) fn new(value: &'a CanonicalValue, within: &'static str) -> Result<Self, Error> {
        match value {
            CanonicalValue::Object(map) => Ok(Self {
                within,
                map,
                taken: 0,
            }),
            _ => Err(Error::InvalidType { field: within }),
        }
    }

    pub(super) fn take(&mut self, field: &'static str) -> Result<&'a CanonicalValue, Error> {
        let value = self.map.get(field).ok_or(Error::MissingField { field })?;
        self.taken += 1;
        Ok(value)
    }

    pub(super) fn finish(self) -> Result<(), Error> {
        if self.taken == self.map.len() {
            Ok(())
        } else {
            Err(Error::UnexpectedField {
                within: self.within,
            })
        }
    }
}

pub(super) fn text(value: &CanonicalValue, field: &'static str) -> Result<String, Error> {
    match value {
        CanonicalValue::Text(text) => Ok(text.clone()),
        _ => Err(Error::InvalidType { field }),
    }
}

pub(super) fn count(value: &CanonicalValue, field: &'static str) -> Result<u64, Error> {
    match value {
        CanonicalValue::Integer(integer) => {
            u64::try_from(*integer).map_err(|_| Error::OutOfRange { field })
        }
        _ => Err(Error::InvalidType { field }),
    }
}

pub(super) fn integer(value: &CanonicalValue, field: &'static str) -> Result<i64, Error> {
    match value {
        CanonicalValue::Integer(integer) => Ok(*integer),
        _ => Err(Error::InvalidType { field }),
    }
}

/// Accepts the typed `Uint64` variant or its tagged canonical object form.
pub(super) fn uint64(value: &CanonicalValue, field: &'static str) -> Result<u64, Error> {
    match value {
        CanonicalValue::Uint64(number) => Ok(*number),
        CanonicalValue::Object(_) => {
            let mut fields = Fields::new(value, field)?;
            if text(fields.take("kind")?, field)? != "uint64" {
                return Err(Error::InvalidType { field });
            }
            let raw = text(fields.take("value")?, field)?;
            fields.finish()?;
            if raw != "0" && (raw.starts_with('0') || raw.is_empty()) {
                return Err(Error::InvalidType { field });
            }
            raw.parse::<u64>().map_err(|_| Error::InvalidType { field })
        }
        _ => Err(Error::InvalidType { field }),
    }
}

pub(super) fn list<T>(
    value: &CanonicalValue,
    field: &'static str,
    parse_item: impl Fn(&CanonicalValue) -> Result<T, Error>,
) -> Result<Vec<T>, Error> {
    match value {
        CanonicalValue::Array(items) => items.iter().map(parse_item).collect(),
        _ => Err(Error::InvalidType { field }),
    }
}

pub(super) fn texts(value: &CanonicalValue, field: &'static str) -> Result<Vec<String>, Error> {
    list(value, field, |item| text(item, field))
}

fn coverage<T>(
    value: &CanonicalValue,
    family: &'static str,
    parse_value: impl FnOnce(&CanonicalValue) -> Result<T, Error>,
) -> Result<PayloadFamilyCoverage<T>, Error> {
    let mut fields = Fields::new(value, family)?;
    let token = text(fields.take("coverage")?, family)?;
    let coverage = match token.as_str() {
        "captured" => PayloadFamilyCoverage::Captured(parse_value(fields.take("value")?)?),
        "unknown" => PayloadFamilyCoverage::Unknown,
        "not_applicable" => PayloadFamilyCoverage::NotApplicable,
        _ => return Err(Error::InvalidCoverage { family }),
    };
    fields.finish()?;
    Ok(coverage)
}

fn boundary(value: &CanonicalValue) -> Result<CheckpointBoundary, Error> {
    let token = text(value, "boundary")?;
    CheckpointBoundary::all()
        .iter()
        .copied()
        .find(|candidate| candidate.code() == token)
        .ok_or(Error::UnknownBoundary)
}

fn pinned(value: &CanonicalValue, field: &'static str, expected: &str) -> Result<(), Error> {
    if text(value, field)? == expected {
        Ok(())
    } else {
        Err(Error::SchemaMismatch { field })
    }
}

pub(super) fn parse(
    value: &CanonicalValue,
) -> Result<(CheckpointBoundary, CheckpointPayloadFamilies), Error> {
    let mut root = Fields::new(value, "payload")?;
    pinned(root.take("schema")?, "schema", CHECKPOINT_PAYLOAD_SCHEMA)?;
    pinned(
        root.take("canonical_profile")?,
        "canonical_profile",
        super::super::CHECKPOINT_CAPTURE_PROFILE,
    )?;
    let boundary = boundary(root.take("boundary")?)?;
    let mut fields = Fields::new(root.take("families")?, "families")?;
    let families = CheckpointPayloadFamilies {
        seed_and_rng: coverage(
            fields.take("seed_and_rng")?,
            "seed_and_rng",
            families::seed_and_rng,
        )?,
        run_configuration: coverage(
            fields.take("run_configuration")?,
            "run_configuration",
            families::run_configuration,
        )?,
        campaign_progress: coverage(
            fields.take("campaign_progress")?,
            "campaign_progress",
            families::campaign_progress,
        )?,
        player_resources: coverage(
            fields.take("player_resources")?,
            "player_resources",
            families::player_resources,
        )?,
        deck_and_piles: coverage(
            fields.take("deck_and_piles")?,
            "deck_and_piles",
            families::deck_and_piles,
        )?,
        card_instances: coverage(
            fields.take("card_instances")?,
            "card_instances",
            families::card_instances,
        )?,
        relics: coverage(fields.take("relics")?, "relics", families::relics)?,
        potions: coverage(fields.take("potions")?, "potions", families::potions)?,
        combat_turn: coverage(
            fields.take("combat_turn")?,
            "combat_turn",
            families::combat_turn,
        )?,
        powers: coverage(fields.take("powers")?, "powers", families::powers)?,
        enemies_and_intents: coverage(
            fields.take("enemies_and_intents")?,
            "enemies_and_intents",
            families::enemies_and_intents,
        )?,
        pending_effects: coverage(
            fields.take("pending_effects")?,
            "pending_effects",
            families::pending_effects,
        )?,
    };
    fields.finish()?;
    root.finish()?;
    Ok((boundary, families))
}
