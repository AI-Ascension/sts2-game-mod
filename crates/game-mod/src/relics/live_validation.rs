// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    catalog_reader::RelicCatalog,
    definition::{
        RelicActivationKind, RelicDefinition, RelicParameterValue, RelicResolvedParameter,
        RelicVisibilityScope,
    },
    error::RelicLiveError,
    live_model::{RelicActivationState, RelicInstanceInput, RelicPendingTrigger},
    model::{
        RELIC_MAX_LIVE_DETAIL_BYTES, RELIC_MAX_PENDING_TRIGGERS, RELIC_PRODUCER_VERSION,
        RelicField, RelicFieldStatus, RelicLiveBinding, RelicVisibility, validate_identity,
        validate_text,
    },
};

pub(super) fn validate_binding(binding: &RelicLiveBinding) -> Result<(), RelicLiveError> {
    if binding.catalog.producer_version != RELIC_PRODUCER_VERSION {
        return Err(RelicLiveError::ProducerVersionMismatch);
    }
    for (value, field) in [
        (&binding.catalog.locale, "locale"),
        (&binding.game_instance_id, "game_instance_id"),
        (&binding.run_id, "run_id"),
        (&binding.snapshot_id, "snapshot_id"),
    ] {
        validate_identity(value, field).map_err(RelicLiveError::InvalidBinding)?;
    }
    Ok(())
}

pub(super) fn validate_instance_identity(
    instance: &RelicInstanceInput,
) -> Result<(), RelicLiveError> {
    validate_identity(&instance.instance_id, "instance_id")
        .map_err(RelicLiveError::InvalidInput)?;
    validate_identity(&instance.definition_id, "definition_id")
        .map_err(RelicLiveError::InvalidInput)?;
    Ok(())
}

pub(super) fn validate_against_catalog(
    catalog: &RelicCatalog,
    instance: &RelicInstanceInput,
    scope: RelicVisibilityScope,
) -> Result<(), RelicLiveError> {
    let definition = catalog
        .definition(&instance.definition_id)
        .ok_or_else(|| RelicLiveError::UnknownDefinition(instance.definition_id.clone()))?;
    validate_counters(definition, instance, scope)?;
    validate_activation(definition, instance)?;
    validate_parameters(definition, &instance.resolved_parameters, scope)?;
    validate_accumulated(instance)?;
    validate_triggers(definition, &instance.pending_triggers, scope)?;
    let detail_bytes = instance_bytes(instance);
    if detail_bytes > RELIC_MAX_LIVE_DETAIL_BYTES {
        return Err(RelicLiveError::DetailTooLarge {
            limit: RELIC_MAX_LIVE_DETAIL_BYTES,
            actual: detail_bytes,
        });
    }
    Ok(())
}

fn validate_counters(
    definition: &RelicDefinition,
    instance: &RelicInstanceInput,
    scope: RelicVisibilityScope,
) -> Result<(), RelicLiveError> {
    let Some(values) = instance.counters.value() else {
        if definition.counters.is_empty()
            && instance.counters.status() != RelicFieldStatus::NotApplicable
        {
            return Err(RelicLiveError::InvalidState("counter_not_applicable"));
        }
        if instance.counters.status() == RelicFieldStatus::NotApplicable
            && !definition.counters.is_empty()
        {
            return Err(RelicLiveError::InvalidState("counter_not_applicable"));
        }
        return Ok(());
    };
    if definition.counters.is_empty() {
        return Err(RelicLiveError::UnknownCounter {
            relic_id: definition.reference.relic_id.clone(),
            counter_id: "none".to_owned(),
        });
    }
    let declared = definition
        .counters
        .iter()
        .map(|counter| counter.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    for counter in values {
        let declaration = definition
            .counters
            .iter()
            .find(|declaration| declaration.id == counter.id)
            .ok_or_else(|| RelicLiveError::UnknownCounter {
                relic_id: definition.reference.relic_id.clone(),
                counter_id: counter.id.clone(),
            })?;
        if !visibility_allowed(declaration.visibility, scope) {
            return Err(RelicLiveError::InvalidState("counter_visibility"));
        }
        if !seen.insert(counter.id.as_str()) {
            return Err(RelicLiveError::DuplicateCounter {
                relic_id: definition.reference.relic_id.clone(),
                counter_id: counter.id.clone(),
            });
        }
    }
    if let Some(counter_id) = declared
        .iter()
        .find(|counter_id| !seen.contains(**counter_id))
    {
        return Err(RelicLiveError::MissingCounter {
            relic_id: definition.reference.relic_id.clone(),
            counter_id: (*counter_id).to_owned(),
        });
    }
    Ok(())
}

fn validate_activation(
    definition: &RelicDefinition,
    instance: &RelicInstanceInput,
) -> Result<(), RelicLiveError> {
    let passive = definition.activation.kind == RelicActivationKind::Passive;
    match (passive, instance.activation.status()) {
        (true, RelicFieldStatus::NotApplicable) => Ok(()),
        (true, _) => Err(RelicLiveError::InvalidState("passive_activation")),
        (false, RelicFieldStatus::NotApplicable) => {
            Err(RelicLiveError::InvalidState("activation_not_applicable"))
        }
        _ => Ok(()),
    }
}

fn validate_parameters(
    definition: &RelicDefinition,
    values: &RelicField<Vec<RelicResolvedParameter>>,
    scope: RelicVisibilityScope,
) -> Result<(), RelicLiveError> {
    let Some(values) = values.value() else {
        return Ok(());
    };
    let mut seen = BTreeSet::new();
    for parameter in values {
        let Some(declaration) = definition
            .parameters
            .iter()
            .find(|declaration| declaration.id == parameter.id)
        else {
            return Err(RelicLiveError::UnknownParameter {
                relic_id: definition.reference.relic_id.clone(),
                parameter_id: parameter.id.clone(),
            });
        };
        if declaration.unit != parameter.unit {
            return Err(RelicLiveError::InvalidState("parameter_unit"));
        }
        if !visibility_allowed(declaration.visibility, scope) {
            return Err(RelicLiveError::InvalidState("parameter_visibility"));
        }
        if !seen.insert(parameter.id.as_str()) {
            return Err(RelicLiveError::DuplicateParameter {
                relic_id: definition.reference.relic_id.clone(),
                parameter_id: parameter.id.clone(),
            });
        }
        if let RelicParameterValue::Text(value) = &parameter.value {
            validate_text(value, "parameter_value").map_err(RelicLiveError::InvalidInput)?;
        }
    }
    Ok(())
}

fn validate_accumulated(instance: &RelicInstanceInput) -> Result<(), RelicLiveError> {
    let Some(values) = instance.accumulated.value() else {
        return Ok(());
    };
    let mut seen = BTreeSet::new();
    for value in values {
        validate_identity(&value.id, "accumulated_id").map_err(RelicLiveError::InvalidInput)?;
        validate_text(&value.label, "accumulated_label").map_err(RelicLiveError::InvalidInput)?;
        if !seen.insert(value.id.as_str()) {
            return Err(RelicLiveError::InvalidState("duplicate_accumulated"));
        }
    }
    Ok(())
}

fn validate_triggers(
    definition: &RelicDefinition,
    values: &RelicField<Vec<RelicPendingTrigger>>,
    scope: RelicVisibilityScope,
) -> Result<(), RelicLiveError> {
    let Some(values) = values.value() else {
        return Ok(());
    };
    if values.len() > RELIC_MAX_PENDING_TRIGGERS {
        return Err(RelicLiveError::InvalidInput("pending_triggers"));
    }
    let mut seen = BTreeSet::new();
    for trigger in values {
        let Some(declaration) = definition
            .triggers
            .iter()
            .find(|declaration| declaration.id == trigger.id)
        else {
            return Err(RelicLiveError::UnknownTrigger {
                relic_id: definition.reference.relic_id.clone(),
                trigger_id: trigger.id.clone(),
            });
        };
        if declaration.label != trigger.label {
            return Err(RelicLiveError::InvalidState("trigger_label"));
        }
        if !visibility_allowed(declaration.visibility, scope) {
            return Err(RelicLiveError::InvalidState("trigger_visibility"));
        }
        if !seen.insert(trigger.id.as_str()) {
            return Err(RelicLiveError::DuplicateTrigger {
                relic_id: definition.reference.relic_id.clone(),
                trigger_id: trigger.id.clone(),
            });
        }
        if let Some(condition) = &trigger.condition {
            validate_identity(&condition.id, "trigger_condition_id")
                .map_err(RelicLiveError::InvalidInput)?;
            validate_text(&condition.label, "trigger_condition_label")
                .map_err(RelicLiveError::InvalidInput)?;
        }
        validate_parameters(
            definition,
            &RelicField::Available(trigger.parameters.clone()),
            scope,
        )?;
    }
    Ok(())
}

pub(super) fn instance_bytes(instance: &RelicInstanceInput) -> usize {
    let mut total = instance.instance_id.len() + instance.definition_id.len();
    total += instance.owner_id.as_str().len();
    total += field_bytes(&instance.counters, |counter| counter.id.len() + 8);
    total += activation_bytes(&instance.activation);
    total += field_bytes(&instance.accumulated, |value| {
        value.id.len() + value.label.len() + value.unit.as_str().len() + 8
    });
    total += field_bytes(&instance.pending_triggers, |trigger| {
        trigger.id.len()
            + trigger.label.len()
            + trigger
                .condition
                .as_ref()
                .map_or(0, |condition| condition.id.len() + condition.label.len())
            + trigger
                .parameters
                .iter()
                .map(|parameter| {
                    parameter.id.len()
                        + parameter.unit.as_str().len()
                        + parameter_value_bytes(&parameter.value)
                })
                .sum::<usize>()
    });
    total += field_bytes(&instance.resolved_parameters, |parameter| {
        parameter.id.len() + parameter.unit.as_str().len() + parameter_value_bytes(&parameter.value)
    });
    total
}

fn field_bytes<T>(field: &RelicField<Vec<T>>, item: impl Fn(&T) -> usize) -> usize {
    field.value().map_or(0, |values| {
        values
            .iter()
            .map(item)
            .sum::<usize>()
            .saturating_add(values.len())
    })
}

fn activation_bytes(field: &RelicField<RelicActivationState>) -> usize {
    field.value().map_or(0, |_| 2)
}

fn parameter_value_bytes(value: &RelicParameterValue) -> usize {
    match value {
        RelicParameterValue::Integer(_) => 8,
        RelicParameterValue::Boolean(_) => 1,
        RelicParameterValue::Text(value) => value.len(),
    }
}

fn visibility_allowed(visibility: RelicVisibility, scope: RelicVisibilityScope) -> bool {
    match visibility {
        RelicVisibility::Visible => true,
        RelicVisibility::OwnerOnly => matches!(scope, RelicVisibilityScope::Owner),
        RelicVisibility::Hidden | RelicVisibility::Unknown => false,
    }
}
