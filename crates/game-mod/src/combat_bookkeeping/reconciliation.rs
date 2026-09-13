// SPDX-License-Identifier: MIT

use super::{
    CombatBookkeepingError, CombatDeckReconciliation, CombatField, CombatFieldStatus,
    CombatZoneInput, CombatZoneInventory, CombatZoneKind, CombatZoneStatus,
};

pub(super) fn validate_deck_totals(
    deck: &CombatDeckReconciliation,
    known_zone_total: u32,
    known_temporary_total: u32,
    inventory: &CombatZoneInventory,
    zones: &[CombatZoneInput],
) -> Result<(), CombatBookkeepingError> {
    validate_count_field(&deck.permanent_deck_count, "permanent deck count")?;
    validate_count_field(&deck.combat_card_count, "combat card count")?;
    validate_count_field(&deck.temporary_card_count, "temporary card count")?;
    let inventory_complete = CombatZoneKind::all()
        .iter()
        .all(|zone| inventory.status(*zone) == CombatZoneStatus::Available);
    if let CombatField::Available(total) = deck.combat_card_count {
        if !inventory_complete {
            return Err(CombatBookkeepingError::InvalidInput(
                "combat total requires complete zone inventory",
            ));
        }
        if !zones
            .iter()
            .all(|zone| matches!(zone.total, CombatField::Available(_)))
        {
            return Err(CombatBookkeepingError::InvalidInput(
                "combat total requires known zone totals",
            ));
        }
        if total != known_zone_total {
            return Err(CombatBookkeepingError::CombatTotalMismatch);
        }
    }
    if let CombatField::Available(total) = deck.temporary_card_count {
        let Some(zone) = zones
            .iter()
            .find(|zone| zone.kind == CombatZoneKind::Temporary)
        else {
            return Err(CombatBookkeepingError::InvalidInput(
                "temporary total requires observed temporary zone",
            ));
        };
        if inventory.status(CombatZoneKind::Temporary) != CombatZoneStatus::Available
            || !matches!(zone.total, CombatField::Available(_))
        {
            return Err(CombatBookkeepingError::InvalidInput(
                "temporary total requires observed temporary zone",
            ));
        }
        if total != known_temporary_total {
            return Err(CombatBookkeepingError::TemporaryTotalMismatch);
        }
    }
    Ok(())
}

fn validate_count_field<T>(
    field: &CombatField<T>,
    name: &'static str,
) -> Result<(), CombatBookkeepingError> {
    if let CombatField::Available(_) = field {
        return Ok(());
    }
    if matches!(
        field.status(),
        CombatFieldStatus::Busy | CombatFieldStatus::Stale
    ) {
        return Err(CombatBookkeepingError::InvalidInput(name));
    }
    Ok(())
}
