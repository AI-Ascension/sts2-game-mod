// SPDX-License-Identifier: MIT

//! Member validation: membership, freshness, local-only refusal and the pile inventory.

use std::collections::BTreeSet;

use crate::ContentManifest;

use super::super::identity::validate_opaque_identity;
use super::super::{
    COOP_CARD_KIND, COOP_CHARACTER_KIND, COOP_MAX_ENTRIES, COOP_MAX_PILE_CARDS, COOP_MAX_PILES,
    COOP_MAX_RESOURCES, COOP_POTION_KIND, COOP_POWER_KIND, COOP_RELIC_KIND,
    COOP_SPECIAL_MECHANIC_KIND, CoopEntry, CoopEntryKind, CoopError, CoopField, CoopFieldValue,
    CoopFieldVisibility, CoopPeerInput, CoopPeerRole, CoopPileKind, CoopQuantity, CoopResource,
};
use super::fields::{
    require_bounded, require_consistent, require_entry_count, require_non_empty_present,
    require_unique, validate_identity_field, validate_text, validate_text_field,
};
use super::manifest::validate_manifest_reference;

/// Every gameplay field of one member together with whether it carries a value.
fn stated_fields(peer: &CoopPeerInput) -> [(CoopField, bool); 9] {
    [
        (CoopField::Character, peer.character_id.is_present()),
        (CoopField::Health, peer.health.is_present()),
        (CoopField::Resources, peer.resources.is_present()),
        (
            CoopField::Piles,
            peer.piles.iter().any(|pile| pile.contents.is_present()),
        ),
        (CoopField::Relics, peer.relics.is_present()),
        (CoopField::Potions, peer.potions.is_present()),
        (CoopField::Powers, peer.powers.is_present()),
        (
            CoopField::SpecialMechanics,
            peer.special_mechanics.is_present(),
        ),
        (CoopField::Readiness, peer.readiness.is_present()),
    ]
}

/// Refuses a value that the member's own membership or freshness contradicts.
///
/// A member that is joining, disconnected or left has no coherent snapshot, and a member whose data
/// lags is not published as current, so a present value in either state is refused rather than
/// shown with a qualifier a reader might not read. A local-only value carried by an ally is refused
/// outright: it must never cross to another member, not even as a labelled field.
fn validate_occurrence(peer: &CoopPeerInput) -> Result<(), CoopError> {
    for (field, present) in stated_fields(peer) {
        if !present {
            continue;
        }
        if !peer.membership.carries_gameplay() {
            return Err(CoopError::GameplayForInactivePeer(peer.peer_id.clone()));
        }
        if !peer.freshness.is_current() {
            return Err(CoopError::ValueWithoutCurrentFreshness(
                peer.peer_id.clone(),
            ));
        }
        if matches!(peer.role, CoopPeerRole::Ally)
            && matches!(field.default_visibility(), CoopFieldVisibility::LocalOnly)
        {
            return Err(CoopError::LocalOnlyValueOnAlly(field.name().to_owned()));
        }
    }
    Ok(())
}

/// Validates one member and returns nothing; the caller owns the aggregate checks.
pub(super) fn validate_peer(
    peer: &CoopPeerInput,
    manifest: &ContentManifest,
) -> Result<(), CoopError> {
    validate_opaque_identity(&peer.peer_id, "peer_id")?;
    validate_occurrence(peer)?;
    validate_text_field(&peer.display_name, "display_name")?;
    validate_character(peer, manifest)?;
    validate_health(peer)?;
    validate_resources(peer)?;
    validate_entries(
        &peer.relics,
        CoopEntryKind::Relic,
        COOP_RELIC_KIND,
        manifest,
        "relics",
    )?;
    validate_entries(
        &peer.potions,
        CoopEntryKind::Potion,
        COOP_POTION_KIND,
        manifest,
        "potions",
    )?;
    validate_entries(
        &peer.powers,
        CoopEntryKind::Power,
        COOP_POWER_KIND,
        manifest,
        "powers",
    )?;
    validate_entries(
        &peer.special_mechanics,
        CoopEntryKind::SpecialMechanic,
        COOP_SPECIAL_MECHANIC_KIND,
        manifest,
        "special_mechanics",
    )?;
    validate_piles(peer, manifest)
}

fn validate_character(peer: &CoopPeerInput, manifest: &ContentManifest) -> Result<(), CoopError> {
    validate_identity_field(&peer.character_id, "character_id")?;
    if let Some(character_id) = peer.character_id.value() {
        validate_manifest_reference(manifest, COOP_CHARACTER_KIND, character_id)?;
    }
    Ok(())
}

fn validate_health(peer: &CoopPeerInput) -> Result<(), CoopError> {
    require_consistent(&peer.health, "health")?;
    let Some(health) = peer.health.value() else {
        return Ok(());
    };
    if health.current < 0 || health.maximum < 0 || health.current > health.maximum {
        return Err(CoopError::ImpossibleHealth(peer.peer_id.clone()));
    }
    Ok(())
}

fn validate_resources(peer: &CoopPeerInput) -> Result<(), CoopError> {
    require_non_empty_present(&peer.resources, "resources")?;
    let Some(resources) = peer.resources.value() else {
        return Ok(());
    };
    require_bounded(resources, COOP_MAX_RESOURCES, "resources")?;
    let mut seen = BTreeSet::new();
    for resource in resources {
        validate_text(&resource.resource_id, "resource_id")?;
        require_unique(&mut seen, &resource.resource_id)?;
        validate_quantity(&resource.amount, "resource_amount")?;
    }
    Ok(())
}

fn validate_quantity(quantity: &CoopQuantity, name: &'static str) -> Result<(), CoopError> {
    validate_text(&quantity.unit.unit, name)
}

fn validate_entries(
    field: &CoopFieldValue<Vec<CoopEntry>>,
    kind: CoopEntryKind,
    entity_kind: &str,
    manifest: &ContentManifest,
    name: &'static str,
) -> Result<(), CoopError> {
    require_non_empty_present(field, name)?;
    let Some(entries) = field.value() else {
        return Ok(());
    };
    require_bounded(entries, COOP_MAX_ENTRIES, name)?;
    let mut seen = BTreeSet::new();
    for entry in entries {
        if entry.kind != kind {
            return Err(CoopError::InvalidInput(name));
        }
        validate_text(&entry.entry_id, name)?;
        require_unique(&mut seen, &entry.entry_id)?;
        require_entry_count(entry.count, &entry.entry_id)?;
        validate_text_field(&entry.label, name)?;
        validate_manifest_reference(manifest, entity_kind, &entry.entry_id)?;
    }
    Ok(())
}

/// Requires one row per pile, each declaring the visibility its own kind carries.
fn validate_piles(peer: &CoopPeerInput, manifest: &ContentManifest) -> Result<(), CoopError> {
    if peer.piles.is_empty() {
        return Err(CoopError::EmptyPresentCollection("piles"));
    }
    require_bounded(&peer.piles, COOP_MAX_PILES, "piles")?;
    let mut seen = BTreeSet::new();
    for pile in &peer.piles {
        if !seen.insert(pile.kind.name()) {
            return Err(CoopError::DuplicatePile(pile.kind.name().to_owned()));
        }
        if pile.visibility != pile.kind.default_visibility() {
            return Err(CoopError::PileVisibilityMismatch(
                pile.kind.name().to_owned(),
            ));
        }
        require_consistent(&pile.contents, "pile_contents")?;
        if matches!(peer.role, CoopPeerRole::Ally)
            && pile.kind.is_local()
            && pile.contents.is_present()
        {
            return Err(CoopError::LocalPilePublishedToAlly(
                pile.kind.name().to_owned(),
            ));
        }
        let Some(contents) = pile.contents.value() else {
            continue;
        };
        require_bounded(&contents.cards, COOP_MAX_PILE_CARDS, "pile_cards")?;
        let mut cards = BTreeSet::new();
        for card_id in &contents.cards {
            validate_text(card_id, "pile_card_id")?;
            require_unique(&mut cards, card_id)?;
            validate_manifest_reference(manifest, COOP_CARD_KIND, card_id)?;
        }
        if contents.count < contents.cards.len() as u32 {
            return Err(CoopError::InvalidInput("pile_count"));
        }
    }
    for kind in CoopPileKind::ALL {
        if !seen.contains(kind.name()) {
            return Err(CoopError::InvalidInput("piles"));
        }
    }
    Ok(())
}

/// Reports the resource amount a member publishes, for the aggregate byte bound.
pub(super) fn resource_bytes(resources: &CoopFieldValue<Vec<CoopResource>>) -> usize {
    resources.value().map_or(0, |resources| {
        resources
            .iter()
            .map(|resource| resource.resource_id.len() + resource.amount.unit.unit.len())
            .sum()
    })
}

/// Reports the entry bytes a member publishes, for the aggregate byte bound.
pub(super) fn entry_bytes(field: &CoopFieldValue<Vec<CoopEntry>>) -> usize {
    field.value().map_or(0, |entries| {
        entries
            .iter()
            .map(|entry| entry.entry_id.len() + entry.label.value().map_or(0, String::len))
            .sum()
    })
}

/// Reports the pile bytes a member publishes, for the aggregate byte bound.
pub(super) fn pile_bytes(peer: &CoopPeerInput) -> usize {
    peer.piles
        .iter()
        .map(|pile| {
            pile.contents.value().map_or(0, |contents| {
                contents.cards.iter().map(String::len).sum::<usize>()
            })
        })
        .sum()
}
