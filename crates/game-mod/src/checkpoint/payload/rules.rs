// SPDX-License-Identifier: MIT

//! Boundary coverage rules and cross-family consistency.

mod bounds;

use std::collections::BTreeSet;

use bounds::*;

use super::super::capability::CheckpointBoundary;
use super::error::CheckpointPayloadError as Error;
use super::{CheckpointPayloadFamilies, PayloadFamilyCoverage};

/// What a boundary requires of one family.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Requirement {
    Captured,
    NotApplicable,
}

fn requirement(boundary: CheckpointBoundary, family: &'static str) -> Requirement {
    let combat_only = matches!(family, "combat_turn" | "powers" | "enemies_and_intents");
    if boundary == CheckpointBoundary::SettledMapChoice && combat_only {
        Requirement::NotApplicable
    } else {
        Requirement::Captured
    }
}

fn check<T>(
    boundary: CheckpointBoundary,
    family: &'static str,
    coverage: &PayloadFamilyCoverage<T>,
    validate: impl FnOnce(&T) -> Result<(), Error>,
) -> Result<(), Error> {
    match (requirement(boundary, family), coverage) {
        (_, PayloadFamilyCoverage::Unknown) => Err(Error::RequiredFamilyUnknown { family }),
        (Requirement::Captured, PayloadFamilyCoverage::NotApplicable) => {
            Err(Error::FamilyNotCaptured { family })
        }
        (Requirement::NotApplicable, PayloadFamilyCoverage::Captured(_)) => {
            Err(Error::FamilyNotApplicable { family })
        }
        (Requirement::NotApplicable, PayloadFamilyCoverage::NotApplicable) => Ok(()),
        (Requirement::Captured, PayloadFamilyCoverage::Captured(value)) => validate(value),
    }
}

pub(super) fn validate(
    boundary: CheckpointBoundary,
    families: &CheckpointPayloadFamilies,
) -> Result<(), Error> {
    if !super::CHECKPOINT_PAYLOAD_BOUNDARIES.contains(&boundary) {
        return Err(Error::UnsupportedBoundary { boundary });
    }
    check(
        boundary,
        "seed_and_rng",
        &families.seed_and_rng,
        seed_and_rng,
    )?;
    check(
        boundary,
        "run_configuration",
        &families.run_configuration,
        run_configuration,
    )?;
    check(
        boundary,
        "campaign_progress",
        &families.campaign_progress,
        campaign_progress,
    )?;
    check(
        boundary,
        "player_resources",
        &families.player_resources,
        player_resources,
    )?;
    check(
        boundary,
        "deck_and_piles",
        &families.deck_and_piles,
        deck_and_piles,
    )?;
    check(
        boundary,
        "card_instances",
        &families.card_instances,
        card_instances,
    )?;
    check(boundary, "relics", &families.relics, relics)?;
    check(boundary, "potions", &families.potions, potions)?;
    check(boundary, "combat_turn", &families.combat_turn, |turn| {
        combat_turn(boundary, turn)
    })?;
    check(boundary, "powers", &families.powers, powers)?;
    check(
        boundary,
        "enemies_and_intents",
        &families.enemies_and_intents,
        enemies_and_intents,
    )?;
    check(
        boundary,
        "pending_effects",
        &families.pending_effects,
        pending_effects,
    )?;
    card_references(families)
}

fn card_references(families: &CheckpointPayloadFamilies) -> Result<(), Error> {
    let (Some(deck), Some(cards)) = (
        families.deck_and_piles.value(),
        families.card_instances.value(),
    ) else {
        return Ok(());
    };
    let known: BTreeSet<&str> = cards
        .cards
        .iter()
        .map(|card| card.instance_id.as_str())
        .collect();
    let references = deck
        .deck
        .iter()
        .chain(deck.piles.iter().flat_map(|pile| pile.cards.iter()));
    for reference in references {
        if !known.contains(reference.as_str()) {
            return Err(Error::UnresolvedCardReference);
        }
    }
    Ok(())
}
