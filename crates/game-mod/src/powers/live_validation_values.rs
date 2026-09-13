// SPDX-License-Identifier: MIT

use super::super::{
    error::PowerStatusLiveError,
    live_model::{
        PowerStatusAmount, PowerStatusDurationState, PowerStatusInstanceInput,
        PowerStatusPendingExpiry, PowerStatusSourceReference,
    },
    model::{PowerStatusField, validate_identity, validate_text},
};

/// Measures one live instance conservatively before it crosses the detail boundary.
pub(crate) fn instance_bytes(instance: &PowerStatusInstanceInput) -> usize {
    let mut total = instance.instance_id.len()
        + instance.definition_id.len()
        + instance.owner.id.as_str().len();
    total += field_bytes(&instance.owner.label, String::len);
    total += source_bytes(&instance.source);
    total += amount_bytes(&instance.amount);
    total += duration_bytes(&instance.duration);
    total += field_bytes(&instance.application_order, |_| 4);
    total += field_bytes(&instance.active, |_| 1);
    total += field_bytes(&instance.suppressed, |_| 1);
    total += pending_expiry_bytes(&instance.pending_expiry);
    total
}

pub(crate) fn validate_pending_expiry(
    field: &PowerStatusField<PowerStatusPendingExpiry>,
) -> Result<(), PowerStatusLiveError> {
    let Some(expiry) = field.value() else {
        return Ok(());
    };
    match expiry {
        PowerStatusPendingExpiry::Remaining { value, .. } if *value < 0 => {
            Err(PowerStatusLiveError::DurationMismatch("expiry_value"))
        }
        PowerStatusPendingExpiry::Condition { id, label } => {
            validate_identity(id, "expiry_condition")
                .map_err(PowerStatusLiveError::InvalidInput)?;
            validate_text(label, "expiry_condition_label")
                .map_err(PowerStatusLiveError::InvalidInput)
        }
        PowerStatusPendingExpiry::Boundary { .. }
        | PowerStatusPendingExpiry::Remaining { .. }
        | PowerStatusPendingExpiry::Unknown => Ok(()),
    }
}

fn source_bytes(source: &PowerStatusField<PowerStatusSourceReference>) -> usize {
    source.value().map_or(0, |value| {
        value.id.as_str().len() + field_bytes(&value.label, String::len) + 1
    })
}

fn amount_bytes(field: &PowerStatusField<PowerStatusAmount>) -> usize {
    field.value().map_or(0, |amount| match amount {
        PowerStatusAmount::Amountless => 1,
        PowerStatusAmount::Integer { unit, .. }
        | PowerStatusAmount::Boolean { unit, .. }
        | PowerStatusAmount::Decimal { unit, .. } => unit.as_str().len() + 9,
        PowerStatusAmount::Text { value, unit } => value.len() + unit.as_str().len(),
        PowerStatusAmount::Counters(values) => values
            .iter()
            .map(|value| value.id.len() + value.unit.as_str().len() + 9)
            .sum(),
        PowerStatusAmount::Custom { kind, value, unit } => {
            kind.len() + value.len() + unit.as_ref().map_or(0, |unit| unit.as_str().len())
        }
    })
}

fn duration_bytes(field: &PowerStatusField<PowerStatusDurationState>) -> usize {
    field.value().map_or(0, |duration| match duration {
        PowerStatusDurationState::Permanent
        | PowerStatusDurationState::Boundary { .. }
        | PowerStatusDurationState::Unknown => 1,
        PowerStatusDurationState::Remaining { unit, .. } => unit.as_str().len() + 9,
        PowerStatusDurationState::Condition { id, label } => id.len() + label.len(),
    })
}

fn pending_expiry_bytes(field: &PowerStatusField<PowerStatusPendingExpiry>) -> usize {
    field.value().map_or(0, |expiry| match expiry {
        PowerStatusPendingExpiry::Boundary { .. } | PowerStatusPendingExpiry::Unknown => 1,
        PowerStatusPendingExpiry::Remaining { unit, .. } => unit.as_str().len() + 9,
        PowerStatusPendingExpiry::Condition { id, label } => id.len() + label.len(),
    })
}

fn field_bytes<T>(field: &PowerStatusField<T>, size: impl Fn(&T) -> usize) -> usize {
    field.value().map_or(0, size)
}
