// SPDX-License-Identifier: MIT

//! Ending-deck, ending-inventory, statistic and trajectory-linkage validation.

use std::collections::BTreeSet;

use crate::ContentManifest;

use super::super::{
    RUN_RESULT_CARD_KIND, RUN_RESULT_MAX_DECK_ENTRIES, RUN_RESULT_MAX_INVENTORY_ENTRIES,
    RUN_RESULT_MAX_STATISTICS, RUN_RESULT_POTION_KIND, RUN_RESULT_RELIC_KIND, RunResultCardEntry,
    RunResultError, RunResultFieldValue, RunResultInput, RunResultItemEntry, RunResultItemKind,
    RunResultLinkage, RunStatistic,
};
use super::fields::{
    require_bounded, require_consistent, require_entry_count, require_non_empty_present,
    validate_identity_field, validate_text, validate_text_field,
};
use super::manifest::validate_manifest_reference;

/// Validates the ending deck, inventory, statistics and linkage of one result.
pub(super) fn validate_run_records(
    input: &RunResultInput,
    manifest: &ContentManifest,
) -> Result<(), RunResultError> {
    validate_ending_deck(&input.ending_deck, manifest)?;
    validate_ending_inventory(&input.ending_inventory, manifest)?;
    validate_statistics(&input.statistics)?;
    validate_linkage(&input.linkage)
}

fn validate_ending_deck(
    field: &RunResultFieldValue<Vec<RunResultCardEntry>>,
    manifest: &ContentManifest,
) -> Result<(), RunResultError> {
    require_non_empty_present(field, "ending_deck")?;
    let Some(entries) = field.value() else {
        return Ok(());
    };
    require_bounded(entries, RUN_RESULT_MAX_DECK_ENTRIES, "ending_deck")?;
    let mut seen = BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.card_id.clone()) {
            return Err(RunResultError::DuplicateEndingEntry(entry.card_id.clone()));
        }
        require_entry_count(entry.count, &entry.card_id)?;
        validate_manifest_reference(manifest, RUN_RESULT_CARD_KIND, &entry.card_id)?;
    }
    Ok(())
}

fn validate_ending_inventory(
    field: &RunResultFieldValue<Vec<RunResultItemEntry>>,
    manifest: &ContentManifest,
) -> Result<(), RunResultError> {
    require_non_empty_present(field, "ending_inventory")?;
    let Some(entries) = field.value() else {
        return Ok(());
    };
    require_bounded(
        entries,
        RUN_RESULT_MAX_INVENTORY_ENTRIES,
        "ending_inventory",
    )?;
    let mut seen = BTreeSet::new();
    for entry in entries {
        if !seen.insert(entry.item_id.clone()) {
            return Err(RunResultError::DuplicateEndingEntry(entry.item_id.clone()));
        }
        require_entry_count(entry.count, &entry.item_id)?;
        let entity_kind = match &entry.kind {
            RunResultItemKind::Relic => RUN_RESULT_RELIC_KIND,
            RunResultItemKind::Potion => RUN_RESULT_POTION_KIND,
            RunResultItemKind::Other { entity_kind } => entity_kind.as_str(),
        };
        validate_manifest_reference(manifest, entity_kind, &entry.item_id)?;
    }
    Ok(())
}

fn validate_statistics(
    field: &RunResultFieldValue<Vec<RunStatistic>>,
) -> Result<(), RunResultError> {
    require_non_empty_present(field, "statistics")?;
    let Some(entries) = field.value() else {
        return Ok(());
    };
    require_bounded(entries, RUN_RESULT_MAX_STATISTICS, "statistics")?;
    let mut seen = BTreeSet::new();
    for entry in entries {
        validate_text(&entry.statistic_id, "statistic_id")?;
        if !seen.insert(entry.statistic_id.clone()) {
            return Err(RunResultError::DuplicateStatistic(
                entry.statistic_id.clone(),
            ));
        }
        validate_text_field(&entry.label, "statistic_label")?;
        require_consistent(&entry.value, "statistic_value")?;
    }
    Ok(())
}

fn validate_linkage(field: &RunResultFieldValue<RunResultLinkage>) -> Result<(), RunResultError> {
    require_consistent(field, "linkage")?;
    let Some(linkage) = field.value() else {
        return Ok(());
    };
    validate_identity_field(
        &RunResultFieldValue::present(linkage.trajectory_id.clone()),
        "trajectory_id",
    )?;
    validate_identity_field(&linkage.evidence_id, "evidence_id")
}
