// SPDX-License-Identifier: MIT

use super::{
    definition::{
        PowerStatusAmountDefinition, PowerStatusDecayRule, PowerStatusDefinitionInput,
        PowerStatusDurationRule, PowerStatusStackingPolicy,
    },
    model::{
        POWER_STATUS_MAX_COUNTERS, POWER_STATUS_MAX_REFERENCES, PowerStatusKind, PowerStatusReset,
        validate_identity, validate_text,
    },
};

/// Validates one static source record before it enters a catalog.
pub(super) fn validate_definition(input: &PowerStatusDefinitionInput) -> Result<(), &'static str> {
    validate_identity(&input.definition_id, "definition_id")?;
    validate_text(&input.title, "title")?;
    validate_text(&input.description, "description")?;
    validate_kind(&input.kind)?;
    validate_identity(input.category.as_str(), "category")?;
    validate_identity(&input.origin.kind, "origin_kind")?;
    if let Some(package_id) = &input.origin.package_id {
        validate_identity(package_id, "package_id")?;
    }
    if let Some(package_version) = &input.origin.package_version {
        validate_identity(package_version, "package_version")?;
    }
    validate_amount(&input.amount)?;
    validate_stacking(&input.stacking)?;
    validate_duration(&input.duration)?;
    validate_decay(&input.decay)?;
    if input.references.len() > POWER_STATUS_MAX_REFERENCES {
        return Err("references");
    }
    let mut reference_ids = std::collections::BTreeSet::new();
    for reference in &input.references {
        validate_identity(&reference.id, "reference_id")?;
        validate_text(&reference.label, "reference_label")?;
        if !reference_ids.insert(reference.id.as_str()) {
            return Err("reference_id");
        }
    }
    Ok(())
}

fn validate_kind(kind: &PowerStatusKind) -> Result<(), &'static str> {
    if let PowerStatusKind::Custom(value) = kind {
        validate_identity(value, "kind")?;
    }
    Ok(())
}

fn validate_amount(amount: &PowerStatusAmountDefinition) -> Result<(), &'static str> {
    match amount {
        PowerStatusAmountDefinition::Amountless => {}
        PowerStatusAmountDefinition::Integer { unit }
        | PowerStatusAmountDefinition::Boolean { unit }
        | PowerStatusAmountDefinition::Text { unit } => {
            validate_identity(unit.as_str(), "amount_unit")?;
        }
        PowerStatusAmountDefinition::Decimal { unit, scale } => {
            validate_identity(unit.as_str(), "amount_unit")?;
            if *scale > 9 {
                return Err("amount_scale");
            }
        }
        PowerStatusAmountDefinition::Counters(counters) => {
            if counters.is_empty() || counters.len() > POWER_STATUS_MAX_COUNTERS {
                return Err("counters");
            }
            let mut ids = std::collections::BTreeSet::new();
            for counter in counters {
                validate_identity(&counter.id, "counter_id")?;
                validate_text(&counter.label, "counter_label")?;
                validate_identity(counter.unit.as_str(), "counter_unit")?;
                if !ids.insert(counter.id.as_str()) {
                    return Err("counter_id");
                }
                if counter.cap.is_some_and(|cap| cap < 0) {
                    return Err("counter_cap");
                }
            }
        }
        PowerStatusAmountDefinition::Custom { kind, unit } => {
            validate_identity(kind, "amount_kind")?;
            if let Some(unit) = unit {
                validate_identity(unit.as_str(), "amount_unit")?;
            }
        }
    }
    Ok(())
}

fn validate_stacking(
    stacking: &super::definition::PowerStatusStackingDefinition,
) -> Result<(), &'static str> {
    if let PowerStatusStackingPolicy::Custom(value) = &stacking.policy {
        validate_identity(value, "stacking_policy")?;
    }
    if let Some(cap) = &stacking.cap {
        validate_identity(cap.unit.as_str(), "cap_unit")?;
        if cap.minimum.is_none() && cap.maximum.is_none() {
            return Err("cap");
        }
        if cap.minimum.is_some_and(|value| value < 0) || cap.maximum.is_some_and(|value| value < 0)
        {
            return Err("cap");
        }
        if let (Some(minimum), Some(maximum)) = (cap.minimum, cap.maximum)
            && minimum > maximum
        {
            return Err("cap");
        }
    }
    Ok(())
}

fn validate_duration(
    duration: &super::definition::PowerStatusDurationDefinition,
) -> Result<(), &'static str> {
    if duration.default.is_some_and(|value| value < 0) {
        return Err("duration_default");
    }
    match &duration.rule {
        PowerStatusDurationRule::Permanent => {
            if duration.default.is_some() || duration.reset != PowerStatusReset::Never {
                return Err("duration_rule");
            }
        }
        PowerStatusDurationRule::Counter { unit } => {
            validate_identity(unit.as_str(), "duration_unit")?;
            if duration.reset == PowerStatusReset::Never {
                return Err("duration_reset");
            }
        }
        PowerStatusDurationRule::Boundary(reset) => {
            if *reset == PowerStatusReset::Never || duration.reset != *reset {
                return Err("duration_boundary");
            }
        }
        PowerStatusDurationRule::Condition(value) | PowerStatusDurationRule::Event(value) => {
            validate_identity(value, "duration_reference")?;
        }
        PowerStatusDurationRule::Unknown => {}
    }
    Ok(())
}

fn validate_decay(
    decay: &super::definition::PowerStatusDecayDefinition,
) -> Result<(), &'static str> {
    match &decay.rule {
        PowerStatusDecayRule::None => {
            if decay.reset != PowerStatusReset::Never {
                return Err("decay_reset");
            }
        }
        PowerStatusDecayRule::Unknown => {}
        PowerStatusDecayRule::By { amount, unit } => {
            if *amount <= 0 {
                return Err("decay_amount");
            }
            validate_identity(unit.as_str(), "decay_unit")?;
            if decay.reset == PowerStatusReset::Never {
                return Err("decay_reset");
            }
        }
        PowerStatusDecayRule::To { amount, unit } => {
            if *amount < 0 {
                return Err("decay_amount");
            }
            validate_identity(unit.as_str(), "decay_unit")?;
            if decay.reset == PowerStatusReset::Never {
                return Err("decay_reset");
            }
        }
        PowerStatusDecayRule::Formula(value) => {
            validate_identity(value, "decay_formula")?;
        }
    }
    Ok(())
}

/// Measures bounded source-owned definition bytes conservatively.
pub(super) fn definition_bytes(input: &PowerStatusDefinitionInput) -> usize {
    let mut total = input.definition_id.len()
        + input.title.len()
        + input.description.len()
        + input.category.as_str().len()
        + input.origin.kind.len();
    total += input.origin.package_id.as_deref().map_or(0, str::len);
    total += input.origin.package_version.as_deref().map_or(0, str::len);
    total += kind_bytes(&input.kind);
    total += amount_bytes(&input.amount);
    if let Some(cap) = &input.stacking.cap {
        total += cap.unit.as_str().len() + 16;
    }
    total += duration_bytes(&input.duration);
    total += decay_bytes(&input.decay);
    total += input
        .references
        .iter()
        .map(|reference| reference.id.len() + reference.label.len() + 2)
        .sum::<usize>();
    total
}

fn kind_bytes(kind: &PowerStatusKind) -> usize {
    match kind {
        PowerStatusKind::Custom(value) => value.len(),
        PowerStatusKind::Power
        | PowerStatusKind::Status
        | PowerStatusKind::Debuff
        | PowerStatusKind::Stance => 1,
    }
}

fn amount_bytes(amount: &PowerStatusAmountDefinition) -> usize {
    match amount {
        PowerStatusAmountDefinition::Amountless => 1,
        PowerStatusAmountDefinition::Integer { unit }
        | PowerStatusAmountDefinition::Boolean { unit }
        | PowerStatusAmountDefinition::Text { unit } => unit.as_str().len() + 1,
        PowerStatusAmountDefinition::Decimal { unit, .. } => unit.as_str().len() + 2,
        PowerStatusAmountDefinition::Counters(counters) => counters
            .iter()
            .map(|counter| {
                counter.id.len() + counter.label.len() + counter.unit.as_str().len() + 10
            })
            .sum(),
        PowerStatusAmountDefinition::Custom { kind, unit } => {
            kind.len() + unit.as_ref().map_or(0, |value| value.as_str().len()) + 1
        }
    }
}

fn duration_bytes(duration: &super::definition::PowerStatusDurationDefinition) -> usize {
    duration.default.map_or(1, |_| 9) + duration_rule_bytes(&duration.rule)
}

fn duration_rule_bytes(rule: &super::definition::PowerStatusDurationRule) -> usize {
    match rule {
        PowerStatusDurationRule::Counter { unit } => unit.as_str().len(),
        PowerStatusDurationRule::Condition(value) | PowerStatusDurationRule::Event(value) => {
            value.len()
        }
        PowerStatusDurationRule::Permanent
        | PowerStatusDurationRule::Boundary(_)
        | PowerStatusDurationRule::Unknown => 1,
    }
}

fn decay_bytes(decay: &super::definition::PowerStatusDecayDefinition) -> usize {
    match &decay.rule {
        PowerStatusDecayRule::By { unit, .. } | PowerStatusDecayRule::To { unit, .. } => {
            unit.as_str().len() + 9
        }
        PowerStatusDecayRule::Formula(value) => value.len(),
        PowerStatusDecayRule::None | PowerStatusDecayRule::Unknown => 1,
    }
}
