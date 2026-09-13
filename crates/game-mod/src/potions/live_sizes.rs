// SPDX-License-Identifier: MIT

use super::{
    definition::PotionParameterValue,
    live_model::{PotionInstanceInput, PotionModifierValue, PotionUsabilityReason},
    model::PotionField,
};

/// Conservative byte estimate for one bounded live instance.
pub(super) fn instance_bytes(instance: &PotionInstanceInput) -> usize {
    let mut total = instance.instance_id.len()
        + instance.definition_id.len()
        + instance.owner_id.as_str().len()
        + instance.slot.slot_id.len();
    total += field_bytes(&instance.modifiers, |modifier| {
        modifier.source_ref.len() + modifier_value_bytes(&modifier.value)
    });
    total += field_bytes(&instance.effective_parameters, |parameter| {
        parameter.id.len() + parameter.unit.as_str().len() + parameter_value_bytes(&parameter.value)
    });
    total += field_bytes(&instance.permitted_targets, |target| {
        target.target_id.len() + target.label.as_deref().map_or(0, str::len) + 1
    });
    total += instance.usability.value().map_or(1, |usability| {
        1 + usability.reason.as_ref().map_or(0, usability_reason_bytes)
    });
    total
}

fn field_bytes<T>(field: &PotionField<Vec<T>>, item: impl Fn(&T) -> usize) -> usize {
    field
        .value()
        .map_or(1, |values| 1 + values.iter().map(item).sum::<usize>())
}

fn modifier_value_bytes(value: &PotionModifierValue) -> usize {
    match value {
        PotionModifierValue::Integer(_)
        | PotionModifierValue::Boolean(_)
        | PotionModifierValue::Marker
        | PotionModifierValue::Unknown => 8,
        PotionModifierValue::Text(value) => value.len(),
        PotionModifierValue::Parameter(value) => parameter_value_bytes(value),
    }
}

fn parameter_value_bytes(value: &PotionParameterValue) -> usize {
    match value {
        PotionParameterValue::Integer(_) | PotionParameterValue::Boolean(_) => 8,
        PotionParameterValue::Text(value) => value.len(),
        PotionParameterValue::IntegerList(values) => 8 * values.len(),
        PotionParameterValue::Unknown => 1,
    }
}

fn usability_reason_bytes(reason: &PotionUsabilityReason) -> usize {
    match reason {
        PotionUsabilityReason::Condition(value) => value.len(),
        _ => 1,
    }
}
