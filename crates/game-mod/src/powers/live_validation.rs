// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

#[path = "live_validation_values.rs"]
mod values;

use super::{
    catalog_reader::PowerStatusCatalog,
    definition::{PowerStatusAmountDefinition, PowerStatusDefinition, PowerStatusDurationRule},
    error::PowerStatusLiveError,
    live_model::{
        PowerStatusAmount, PowerStatusCounterValue, PowerStatusDurationState,
        PowerStatusInstanceInput, PowerStatusSourceKind, duration_rule_matches,
    },
    model::{
        POWER_STATUS_MAX_COUNTERS, POWER_STATUS_MAX_LIVE_DETAIL_BYTES,
        POWER_STATUS_PRODUCER_VERSION, PowerStatusField, PowerStatusFieldStatus,
        PowerStatusLiveBinding, PowerStatusVisibility, PowerStatusVisibilityScope,
        validate_identity, validate_text,
    },
};

pub(super) use values::{instance_bytes, validate_pending_expiry};

pub(super) fn validate_binding(
    binding: &PowerStatusLiveBinding,
) -> Result<(), PowerStatusLiveError> {
    if binding.catalog.producer_version != POWER_STATUS_PRODUCER_VERSION {
        return Err(PowerStatusLiveError::ProducerVersionMismatch);
    }
    for (value, field) in [
        (&binding.catalog.locale, "locale"),
        (&binding.game_instance_id, "game_instance_id"),
        (&binding.run_id, "run_id"),
        (&binding.snapshot_id, "snapshot_id"),
    ] {
        validate_identity(value, field).map_err(PowerStatusLiveError::InvalidBinding)?;
    }
    Ok(())
}

pub(super) fn validate_instance_identity(
    instance: &PowerStatusInstanceInput,
) -> Result<(), PowerStatusLiveError> {
    validate_identity(&instance.instance_id, "instance_id")
        .map_err(PowerStatusLiveError::InvalidInput)?;
    validate_identity(&instance.definition_id, "definition_id")
        .map_err(PowerStatusLiveError::InvalidInput)?;
    validate_identity(instance.owner.id.as_str(), "owner_id")
        .map_err(PowerStatusLiveError::InvalidInput)?;
    Ok(())
}

pub(super) fn validate_against_catalog(
    catalog: &PowerStatusCatalog,
    instance: &PowerStatusInstanceInput,
    scope: PowerStatusVisibilityScope,
) -> Result<(), PowerStatusLiveError> {
    let definition = catalog
        .definition(&instance.definition_id)
        .ok_or_else(|| PowerStatusLiveError::UnknownDefinition(instance.definition_id.clone()))?;
    if !visibility_allowed(definition.visibility, scope) {
        return Err(PowerStatusLiveError::VisibilityDenied("definition"));
    }
    validate_owner(instance)?;
    validate_source(&instance.source)?;
    validate_amount(definition, &instance.amount, scope)?;
    validate_duration(definition, &instance.duration)?;
    validate_pending_expiry(&instance.pending_expiry)?;
    let detail_bytes = instance_bytes(instance);
    if detail_bytes > POWER_STATUS_MAX_LIVE_DETAIL_BYTES {
        return Err(PowerStatusLiveError::DetailTooLarge {
            limit: POWER_STATUS_MAX_LIVE_DETAIL_BYTES,
            actual: detail_bytes,
        });
    }
    Ok(())
}

fn validate_owner(instance: &PowerStatusInstanceInput) -> Result<(), PowerStatusLiveError> {
    if let PowerStatusField::Available(label) = &instance.owner.label {
        validate_text(label, "owner_label").map_err(PowerStatusLiveError::InvalidInput)?;
    }
    Ok(())
}

fn validate_source(
    source: &PowerStatusField<super::live_model::PowerStatusSourceReference>,
) -> Result<(), PowerStatusLiveError> {
    let Some(source) = source.value() else {
        return Ok(());
    };
    validate_identity(source.id.as_str(), "source_id")
        .map_err(PowerStatusLiveError::InvalidInput)?;
    if let PowerStatusSourceKind::Custom(kind) = &source.kind {
        validate_identity(kind, "source_kind").map_err(PowerStatusLiveError::InvalidInput)?;
    }
    if let PowerStatusField::Available(label) = &source.label {
        validate_text(label, "source_label").map_err(PowerStatusLiveError::InvalidInput)?;
    }
    Ok(())
}

fn validate_amount(
    definition: &PowerStatusDefinition,
    field: &PowerStatusField<PowerStatusAmount>,
    scope: PowerStatusVisibilityScope,
) -> Result<(), PowerStatusLiveError> {
    let Some(amount) = field.value() else {
        if matches!(
            (&definition.amount, field.status()),
            (
                PowerStatusAmountDefinition::Amountless,
                PowerStatusFieldStatus::Available
            )
        ) {
            return Err(PowerStatusLiveError::AmountShapeMismatch("amountless"));
        }
        if matches!(
            (&definition.amount, field.status()),
            (
                PowerStatusAmountDefinition::Amountless,
                PowerStatusFieldStatus::NotApplicable
            )
        ) {
            return Ok(());
        }
        return Ok(());
    };
    match (&definition.amount, amount) {
        (
            PowerStatusAmountDefinition::Integer { unit },
            PowerStatusAmount::Integer { unit: observed, .. },
        )
        | (
            PowerStatusAmountDefinition::Boolean { unit },
            PowerStatusAmount::Boolean { unit: observed, .. },
        )
        | (
            PowerStatusAmountDefinition::Text { unit },
            PowerStatusAmount::Text { unit: observed, .. },
        ) => {
            if unit != observed {
                return Err(PowerStatusLiveError::AmountShapeMismatch("amount_unit"));
            }
            if let PowerStatusAmount::Integer { value, .. } = amount {
                validate_scalar_cap(definition, *value, observed)?;
            }
            if let PowerStatusAmount::Text { value, .. } = amount {
                validate_text(value, "amount_text").map_err(PowerStatusLiveError::InvalidInput)?;
            }
            Ok(())
        }
        (
            PowerStatusAmountDefinition::Decimal { unit, scale },
            PowerStatusAmount::Decimal {
                unit: observed,
                scale: observed_scale,
                ..
            },
        ) => {
            if unit != observed || scale != observed_scale || *observed_scale > 9 {
                Err(PowerStatusLiveError::AmountShapeMismatch("amount_decimal"))
            } else if let PowerStatusAmount::Decimal { value, .. } = amount {
                validate_scalar_cap(definition, *value, observed)
            } else {
                Ok(())
            }
        }
        (
            PowerStatusAmountDefinition::Counters(declarations),
            PowerStatusAmount::Counters(values),
        ) => validate_counters(
            &definition.reference.definition_id,
            declarations,
            values,
            scope,
        ),
        (
            PowerStatusAmountDefinition::Custom {
                kind,
                unit: expected_unit,
            },
            PowerStatusAmount::Custom {
                kind: observed_kind,
                unit: observed_unit,
                value,
            },
        ) => {
            if kind != observed_kind || expected_unit.as_ref() != observed_unit.as_ref() {
                return Err(PowerStatusLiveError::AmountShapeMismatch("amount_custom"));
            }
            validate_text(value, "amount_value").map_err(PowerStatusLiveError::InvalidInput)
        }
        (PowerStatusAmountDefinition::Amountless, _) => {
            Err(PowerStatusLiveError::AmountShapeMismatch("amountless"))
        }
        _ => Err(PowerStatusLiveError::AmountShapeMismatch("amount_kind")),
    }
}

fn validate_scalar_cap(
    definition: &PowerStatusDefinition,
    value: i64,
    unit: &super::model::PowerStatusUnit,
) -> Result<(), PowerStatusLiveError> {
    let Some(cap) = &definition.stacking.cap else {
        return Ok(());
    };
    if &cap.unit != unit {
        return Err(PowerStatusLiveError::AmountShapeMismatch("cap_unit"));
    }
    if cap.minimum.is_some_and(|minimum| value < minimum)
        || cap.maximum.is_some_and(|maximum| value > maximum)
    {
        return Err(PowerStatusLiveError::AmountShapeMismatch("cap"));
    }
    Ok(())
}

fn validate_counters(
    definition_id: &str,
    declarations: &[super::definition::PowerStatusCounterDefinition],
    values: &[PowerStatusCounterValue],
    scope: PowerStatusVisibilityScope,
) -> Result<(), PowerStatusLiveError> {
    if values.len() > POWER_STATUS_MAX_COUNTERS {
        return Err(PowerStatusLiveError::AmountShapeMismatch("counter_count"));
    }
    let mut seen = BTreeSet::new();
    for value in values {
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.id == value.id)
            .ok_or_else(|| PowerStatusLiveError::UnknownCounter {
                definition_id: definition_id.to_owned(),
                counter_id: value.id.clone(),
            })?;
        if declaration.unit != value.unit {
            return Err(PowerStatusLiveError::AmountShapeMismatch("counter_unit"));
        }
        if !visibility_allowed(declaration.visibility, scope) {
            return Err(PowerStatusLiveError::VisibilityDenied("counter"));
        }
        if !seen.insert(value.id.as_str()) {
            return Err(PowerStatusLiveError::DuplicateCounter {
                definition_id: definition_id.to_owned(),
                counter_id: value.id.clone(),
            });
        }
        if let Some(cap) = declaration.cap
            && let Some(observed) = value.value.value()
            && *observed > cap
        {
            return Err(PowerStatusLiveError::AmountShapeMismatch("counter_cap"));
        }
    }
    if let Some(declaration) = declarations
        .iter()
        .find(|declaration| !seen.contains(declaration.id.as_str()))
    {
        return Err(PowerStatusLiveError::MissingCounter {
            definition_id: definition_id.to_owned(),
            counter_id: declaration.id.clone(),
        });
    }
    Ok(())
}

fn validate_duration(
    definition: &PowerStatusDefinition,
    field: &PowerStatusField<PowerStatusDurationState>,
) -> Result<(), PowerStatusLiveError> {
    let Some(value) = field.value() else {
        if field.status() == PowerStatusFieldStatus::NotApplicable
            && !matches!(definition.duration.rule, PowerStatusDurationRule::Permanent)
        {
            return Err(PowerStatusLiveError::DurationMismatch("not_applicable"));
        }
        return Ok(());
    };
    if !duration_rule_matches(&definition.duration.rule, value) {
        return Err(PowerStatusLiveError::DurationMismatch("duration_rule"));
    }
    if let PowerStatusDurationState::Remaining { value, .. } = value
        && *value < 0
    {
        return Err(PowerStatusLiveError::DurationMismatch("duration_value"));
    }
    if let PowerStatusDurationState::Condition { id, label } = value {
        validate_identity(id, "duration_condition").map_err(PowerStatusLiveError::InvalidInput)?;
        validate_text(label, "duration_condition_label")
            .map_err(PowerStatusLiveError::InvalidInput)?;
    }
    Ok(())
}

fn visibility_allowed(
    visibility: PowerStatusVisibility,
    scope: PowerStatusVisibilityScope,
) -> bool {
    match visibility {
        PowerStatusVisibility::Visible => true,
        PowerStatusVisibility::OwnerOnly => matches!(scope, PowerStatusVisibilityScope::Owner),
        PowerStatusVisibility::Hidden | PowerStatusVisibility::Unknown => false,
    }
}
