// SPDX-License-Identifier: MIT

use std::mem::size_of;

use super::super::{RewardField, RewardLiveError, RewardNumericValue, RewardText};
use super::snapshot::{RewardLiveCollection, RewardLiveOffer, RewardLiveSnapshotInput};

pub(super) fn snapshot_bytes(input: &RewardLiveSnapshotInput) -> Result<usize, RewardLiveError> {
    let mut total = size_of::<RewardLiveSnapshotInput>();
    add(&mut total, binding_bytes(&input.catalog)?)?;
    add(&mut total, input.instance_id.capacity())?;
    add(&mut total, input.run_id.capacity())?;
    add(&mut total, input.room_id.capacity())?;
    add(&mut total, input.snapshot_id.capacity())?;
    add(&mut total, field_string_bytes(&input.source_revision))?;
    add(&mut total, fence_bytes(input)?)?;
    add(&mut total, collection_bytes(&input.offers)?)?;
    for offer in &input.offers.entries {
        add(&mut total, offer_bytes(offer)?)?;
    }
    Ok(total)
}

fn fence_bytes(input: &RewardLiveSnapshotInput) -> Result<usize, RewardLiveError> {
    let mut total = size_of::<super::identity::RewardLiveSnapshotFence>()
        .checked_add(2 * size_of::<usize>())
        .ok_or(RewardLiveError::AccountingOverflow)?;
    add(&mut total, binding_bytes(&input.catalog)?)?;
    add(&mut total, input.instance_id.capacity())?;
    add(&mut total, input.run_id.capacity())?;
    add(&mut total, input.room_id.capacity())?;
    add(&mut total, input.snapshot_id.capacity())?;
    add(&mut total, field_string_bytes(&input.source_revision))?;
    Ok(total)
}

pub(super) fn offer_bytes(offer: &RewardLiveOffer) -> Result<usize, RewardLiveError> {
    let mut total = size_of::<RewardLiveOffer>();
    add(&mut total, offer.id.capacity())?;
    add(
        &mut total,
        field_definition_reference_bytes(&offer.definition)?,
    )?;
    add(&mut total, kind_bytes(&offer.kind))?;
    add(&mut total, collection_bytes(&offer.groups)?)?;
    add(&mut total, collection_bytes(&offer.items)?)?;
    for group in &offer.groups.entries {
        add(&mut total, size_of::<super::snapshot::RewardLiveGroup>())?;
        add(&mut total, group.id.capacity())?;
        add(&mut total, field_selection_bytes(&group.selection)?)?;
        add(&mut total, text_bytes(&group.label))?;
        add(&mut total, collection_bytes(&group.items)?)?;
        add(&mut total, collection_bytes(&group.actions)?)?;
        for action in &group.actions.entries {
            add(&mut total, size_of::<super::snapshot::RewardLiveAction>())?;
            add(&mut total, action.id.capacity())?;
            add(&mut total, action_kind_bytes(&action.kind))?;
            add(&mut total, text_bytes(&action.label))?;
            add(&mut total, collection_bytes(&action.targets)?)?;
            for id in &action.targets.entries {
                add(&mut total, id.capacity())?;
            }
        }
        for item_id in &group.items.entries {
            add(&mut total, item_id.capacity())?;
        }
    }
    for item in &offer.items.entries {
        add(&mut total, size_of::<super::snapshot::RewardLiveItem>())?;
        add(&mut total, item.id.capacity())?;
        if let RewardField::Available(id) = &item.instance_id {
            add(&mut total, id.capacity())?;
        }
        add(&mut total, field_item_reference_bytes(&item.definition)?)?;
        add(&mut total, field_semantic_bytes(&item.content)?)?;
        add(&mut total, text_bytes(&item.label))?;
        add(&mut total, quantity_bytes(&item.quantity)?)?;
    }
    Ok(total)
}

fn collection_bytes<T>(collection: &RewardLiveCollection<T>) -> Result<usize, RewardLiveError> {
    collection
        .entries
        .capacity()
        .checked_mul(size_of::<T>())
        .ok_or(RewardLiveError::AccountingOverflow)
}

fn binding_bytes(binding: &super::super::RewardCatalogBinding) -> Result<usize, RewardLiveError> {
    let manifest = &binding.manifest;
    let mut total = size_of::<super::super::RewardCatalogBinding>();
    add(&mut total, manifest.adapter_compatibility.capacity())?;
    add(&mut total, manifest.content_set_revision.capacity())?;
    add(&mut total, manifest.localized_text_revision.capacity())?;
    add(&mut total, manifest.inventory_revision.capacity())?;
    add(&mut total, binding.locale.capacity())?;
    add(&mut total, binding.producer_version.capacity())?;
    Ok(total)
}

fn definition_reference_bytes(
    reference: &super::super::RewardDefinitionReference,
) -> Result<usize, RewardLiveError> {
    let mut total = size_of::<super::super::RewardDefinitionReference>();
    add(&mut total, binding_bytes(&reference.catalog)?)?;
    add(&mut total, reference.reward_id.capacity())?;
    Ok(total)
}

fn field_definition_reference_bytes(
    field: &RewardField<super::super::RewardDefinitionReference>,
) -> Result<usize, RewardLiveError> {
    match field {
        RewardField::Available(reference) => definition_reference_bytes(reference),
        RewardField::Unavailable(_) => Ok(size_of::<
            RewardField<super::super::RewardDefinitionReference>,
        >()),
    }
}

fn field_item_reference_bytes(
    field: &RewardField<super::super::RewardItemReference>,
) -> Result<usize, RewardLiveError> {
    match field {
        RewardField::Available(reference) => {
            let mut total = size_of::<super::super::RewardItemReference>();
            add(&mut total, binding_bytes(&reference.catalog)?)?;
            add(&mut total, reference.reward_id.capacity())?;
            add(&mut total, reference.item_id.capacity())?;
            Ok(total)
        }
        RewardField::Unavailable(_) => {
            Ok(size_of::<RewardField<super::super::RewardItemReference>>())
        }
    }
}

fn field_selection_bytes(
    field: &RewardField<super::snapshot::RewardLiveSelectionReference>,
) -> Result<usize, RewardLiveError> {
    match field {
        RewardField::Available(reference) => {
            let mut total = definition_reference_bytes(&reference.reward)?;
            add(&mut total, reference.group_id.capacity())?;
            Ok(total)
        }
        RewardField::Unavailable(_) => Ok(size_of::<
            RewardField<super::snapshot::RewardLiveSelectionReference>,
        >()),
    }
}

fn field_semantic_bytes(
    field: &RewardField<super::super::RewardSemanticReference>,
) -> Result<usize, RewardLiveError> {
    match field {
        RewardField::Available(reference) => semantic_bytes(reference),
        RewardField::Unavailable(_) => {
            Ok(size_of::<RewardField<super::super::RewardSemanticReference>>())
        }
    }
}

fn semantic_bytes(
    reference: &super::super::RewardSemanticReference,
) -> Result<usize, RewardLiveError> {
    let mut total = size_of::<super::super::RewardSemanticReference>();
    add(&mut total, reference.id.capacity())?;
    add(&mut total, text_bytes(&reference.label))?;
    if let super::super::RewardSemanticReferenceKind::Content { entity_kind } = &reference.kind {
        add(&mut total, entity_kind.capacity())?;
    }
    Ok(total)
}

fn text_bytes(value: &RewardText) -> usize {
    match value {
        RewardText::Available(text) => text.capacity(),
        RewardText::Unavailable(_) => size_of::<RewardText>(),
    }
}

fn field_string_bytes(value: &RewardField<String>) -> usize {
    match value {
        RewardField::Available(text) => text.capacity(),
        RewardField::Unavailable(_) => size_of::<RewardField<String>>(),
    }
}

fn quantity_bytes(quantity: &super::super::RewardQuantity) -> Result<usize, RewardLiveError> {
    let mut total = size_of::<super::super::RewardQuantity>();
    add(&mut total, field_string_bytes(&quantity.unit))?;
    add(&mut total, numeric_bytes(&quantity.base_amount)?)?;
    add(&mut total, numeric_bytes(&quantity.visible_amount)?)?;
    Ok(total)
}

fn numeric_bytes(value: &RewardNumericValue) -> Result<usize, RewardLiveError> {
    match value {
        RewardNumericValue::Formula(formula) => {
            let mut total = size_of::<super::super::RewardFormula>()
                .checked_add(
                    formula
                        .unresolved_inputs
                        .capacity()
                        .checked_mul(size_of::<String>())
                        .ok_or(RewardLiveError::AccountingOverflow)?,
                )
                .ok_or(RewardLiveError::AccountingOverflow)?;
            add(&mut total, formula.rule_reference.capacity())?;
            for input in &formula.unresolved_inputs {
                add(&mut total, input.capacity())?;
            }
            Ok(total)
        }
        RewardNumericValue::Fixed(_) | RewardNumericValue::Unavailable(_) => {
            Ok(size_of::<RewardNumericValue>())
        }
    }
}

fn kind_bytes(kind: &super::super::RewardKind) -> usize {
    match kind {
        super::super::RewardKind::Custom(value) | super::super::RewardKind::Unsupported(value) => {
            value.capacity()
        }
        _ => size_of::<super::super::RewardKind>(),
    }
}

fn action_kind_bytes(kind: &super::super::RewardActionKind) -> usize {
    match kind {
        super::super::RewardActionKind::Custom(value)
        | super::super::RewardActionKind::Unsupported(value) => value.capacity(),
        _ => size_of::<super::super::RewardActionKind>(),
    }
}

fn add(total: &mut usize, amount: usize) -> Result<(), RewardLiveError> {
    *total = total
        .checked_add(amount)
        .ok_or(RewardLiveError::AccountingOverflow)?;
    Ok(())
}
