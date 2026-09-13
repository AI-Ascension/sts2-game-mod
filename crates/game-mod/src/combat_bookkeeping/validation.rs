// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    COMBAT_BOOKKEEPING_MAX_PENDING_CHOICES, COMBAT_BOOKKEEPING_MAX_SNAPSHOT_BYTES,
    COMBAT_BOOKKEEPING_MAX_ZONE_CARDS, COMBAT_BOOKKEEPING_MAX_ZONE_TOTAL, CombatBookkeepingError,
    CombatCounterKind, CombatZoneInventory,
    model::{
        CombatBookkeepingBinding, CombatCardPosition, CombatCompositionCompleteness, CombatCounter,
        CombatCounterProvenance, CombatField, CombatPending, CombatResolutionState,
        CombatSnapshotInput, CombatZoneInput, CombatZoneKind, CombatZoneStatus, validate_identity,
        validate_text,
    },
};
use super::{measurement, reconciliation};

pub(super) fn validate_snapshot(input: &CombatSnapshotInput) -> Result<(), CombatBookkeepingError> {
    validate_binding(&input.binding)?;
    if input.zones.len() > super::COMBAT_BOOKKEEPING_MAX_ZONES {
        return Err(CombatBookkeepingError::InvalidInput("zone count"));
    }
    validate_zone_inventory(&input.zone_inventory)?;
    super::model::validate_turn_identity(&input.turn)?;
    let mut kinds = BTreeSet::new();
    let mut cards = BTreeSet::new();
    let mut zone_total = 0u32;
    let mut temporary_total = 0u32;
    for zone in &input.zones {
        if !kinds.insert(zone.kind) {
            return Err(CombatBookkeepingError::DuplicateZone(zone.kind));
        }
        if input.zone_inventory.status(zone.kind) != CombatZoneStatus::Available {
            return Err(CombatBookkeepingError::InvalidInput(
                "zone has no available inventory entry",
            ));
        }
        validate_zone(&input.binding, zone, &mut cards)?;
        if let CombatField::Available(total) = zone.total {
            zone_total = zone_total
                .checked_add(total)
                .ok_or(CombatBookkeepingError::InvalidInput("zone total overflow"))?;
            if zone.kind == CombatZoneKind::Temporary {
                temporary_total = temporary_total.checked_add(total).ok_or(
                    CombatBookkeepingError::InvalidInput("temporary total overflow"),
                )?;
            }
        }
    }
    for zone in CombatZoneKind::all() {
        if input.zone_inventory.status(*zone) == CombatZoneStatus::Available
            && !kinds.contains(zone)
        {
            return Err(CombatBookkeepingError::InvalidInput(
                "available zone has no projection",
            ));
        }
    }
    reconciliation::validate_deck_totals(
        &input.deck,
        zone_total,
        temporary_total,
        &input.zone_inventory,
        &input.zones,
    )?;
    validate_counters(&input.counters)?;
    validate_resolution(&input.resolution)?;
    validate_pending(&input.binding, &input.pending)?;
    if measurement::snapshot_bytes(input) > COMBAT_BOOKKEEPING_MAX_SNAPSHOT_BYTES {
        return Err(CombatBookkeepingError::InvalidInput("snapshot too large"));
    }
    Ok(())
}

fn validate_binding(binding: &CombatBookkeepingBinding) -> Result<(), CombatBookkeepingError> {
    for (field, value) in [
        ("content_manifest", binding.content_manifest.as_str()),
        ("game_instance_id", binding.game_instance_id.as_str()),
        ("run_id", binding.run_id.as_str()),
        ("combat_id", binding.combat_id.as_str()),
        ("snapshot_id", binding.snapshot_id.as_str()),
    ] {
        validate_identity(value, field)?;
    }
    Ok(())
}

fn validate_zone_inventory(inventory: &CombatZoneInventory) -> Result<(), CombatBookkeepingError> {
    for zone in inventory.statuses.keys() {
        if !CombatZoneKind::all().contains(zone) {
            return Err(CombatBookkeepingError::InvalidInput("unknown zone"));
        }
    }
    Ok(())
}

fn validate_zone(
    binding: &CombatBookkeepingBinding,
    zone: &CombatZoneInput,
    cards: &mut BTreeSet<String>,
) -> Result<(), CombatBookkeepingError> {
    if let CombatField::Available(total) = zone.total
        && total > COMBAT_BOOKKEEPING_MAX_ZONE_TOTAL
    {
        return Err(CombatBookkeepingError::InvalidInput("zone total"));
    }
    let Some(members) = zone.composition.value() else {
        return Ok(());
    };
    if members.len() > COMBAT_BOOKKEEPING_MAX_ZONE_CARDS {
        return Err(CombatBookkeepingError::InvalidInput("zone composition"));
    }
    if let CombatField::Available(total) = zone.total {
        if zone.completeness == CombatCompositionCompleteness::Complete
            && total as usize != members.len()
        {
            return Err(CombatBookkeepingError::CountMismatch(zone.kind));
        }
        if zone.completeness == CombatCompositionCompleteness::Partial
            && (total as usize) < members.len()
        {
            return Err(CombatBookkeepingError::CountMismatch(zone.kind));
        }
    }
    let mut positions = BTreeSet::new();
    for member in members {
        if member.card.binding != *binding {
            return Err(CombatBookkeepingError::StaleReference);
        }
        validate_identity(&member.card.instance_id, "instance_id")?;
        validate_identity(&member.card.definition_id, "definition_id")?;
        validate_identity(&member.card.owner_id, "owner_id")?;
        if !cards.insert(member.card.instance_id.clone()) {
            return Err(CombatBookkeepingError::DuplicateCard(
                member.card.instance_id.clone(),
            ));
        }
        match (zone.ordering, member.position) {
            (super::CombatOrder::Public, CombatCardPosition::Known(position)) => {
                if !positions.insert(position) {
                    return Err(CombatBookkeepingError::InvalidPublicOrder);
                }
            }
            (super::CombatOrder::Public, _) => {
                return Err(CombatBookkeepingError::InvalidPublicOrder);
            }
            (_, CombatCardPosition::Known(_)) => {
                return Err(CombatBookkeepingError::SecretOrderExposed);
            }
            (_, CombatCardPosition::NotObserved | CombatCardPosition::NotApplicable) => {}
        }
    }
    Ok(())
}

fn validate_counters(counters: &super::CombatCounters) -> Result<(), CombatBookkeepingError> {
    for (expected, counter) in [
        (CombatCounterKind::CardsPlayed, &counters.cards_played),
        (CombatCounterKind::DamageTaken, &counters.damage_taken),
        (CombatCounterKind::DamageDealt, &counters.damage_dealt),
        (
            CombatCounterKind::EnemiesDefeated,
            &counters.enemies_defeated,
        ),
    ] {
        if counter.kind != expected {
            return Err(CombatBookkeepingError::InvalidInput("counter kind"));
        }
        validate_counter(counter)?;
    }
    Ok(())
}

fn validate_counter(counter: &CombatCounter) -> Result<(), CombatBookkeepingError> {
    if counter.kind == CombatCounterKind::CardsPlayed
        && matches!(counter.value, CombatField::Available(_))
        && counter.reset == super::CombatCounterReset::Unknown
    {
        return Err(CombatBookkeepingError::InvalidInput(
            "cards played reset boundary",
        ));
    }
    if let CombatCounterProvenance::SemanticHistory { history_id, .. } = &counter.provenance {
        validate_identity(history_id, "history_id")?;
    }
    Ok(())
}

fn validate_resolution(
    field: &CombatField<CombatResolutionState>,
) -> Result<(), CombatBookkeepingError> {
    if let Some(CombatResolutionState::Resolving { effect_id }) = field.value() {
        validate_identity(effect_id, "effect_id")?;
    }
    Ok(())
}

fn validate_pending(
    binding: &CombatBookkeepingBinding,
    field: &CombatField<CombatPending>,
) -> Result<(), CombatBookkeepingError> {
    let Some(pending) = field.value() else {
        return Ok(());
    };
    match pending {
        CombatPending::None => {}
        CombatPending::Selection {
            selection_id,
            kind,
            choices,
        } => {
            validate_text(selection_id, "selection_id")?;
            validate_text(kind, "selection_kind")?;
            if choices.len() > COMBAT_BOOKKEEPING_MAX_PENDING_CHOICES {
                return Err(CombatBookkeepingError::InvalidInput("pending choice count"));
            }
            let mut seen = BTreeSet::new();
            for card in choices {
                if card.binding != *binding {
                    return Err(CombatBookkeepingError::StaleReference);
                }
                validate_identity(&card.instance_id, "choice_instance_id")?;
                validate_identity(&card.definition_id, "choice_definition_id")?;
                validate_identity(&card.owner_id, "choice_owner_id")?;
                if !seen.insert(card.instance_id.as_str()) {
                    return Err(CombatBookkeepingError::DuplicateCard(
                        card.instance_id.clone(),
                    ));
                }
            }
        }
        CombatPending::Effect {
            effect_id,
            kind,
            target_ids,
        } => {
            validate_text(effect_id, "effect_id")?;
            validate_text(kind, "effect_kind")?;
            if target_ids.len() > super::COMBAT_BOOKKEEPING_MAX_ZONE_CARDS {
                return Err(CombatBookkeepingError::InvalidInput("target count"));
            }
            for target in target_ids {
                validate_identity(target, "target_id")?;
            }
        }
    }
    Ok(())
}
