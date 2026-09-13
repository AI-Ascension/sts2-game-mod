// SPDX-License-Identifier: MIT

use super::{
    definition::{RelicActivationKind, RelicDefinitionInput},
    model::{
        RELIC_MAX_ACQUISITION_RULES, RELIC_MAX_COUNTERS, RELIC_MAX_PARAMETERS,
        RELIC_MAX_PENDING_TRIGGERS, RELIC_MAX_REFERENCES, RELIC_MAX_VARIANTS, validate_identity,
        validate_text,
    },
};

/// Validates one static input record before it can enter a catalog.
pub(super) fn validate_definition(input: &RelicDefinitionInput) -> Result<(), &'static str> {
    validate_identity(&input.relic_id, "relic_id")?;
    validate_text(&input.title, "title")?;
    validate_text(&input.description, "description")?;
    validate_identity(&input.origin.kind, "origin_kind")?;
    if let Some(package_id) = &input.origin.package_id {
        validate_identity(package_id, "package_id")?;
    }
    if let Some(package_version) = &input.origin.package_version {
        validate_identity(package_version, "package_version")?;
    }
    if input.acquisition.rules.is_empty()
        || input.acquisition.rules.len() > RELIC_MAX_ACQUISITION_RULES
    {
        return Err("acquisition_rules");
    }
    for rule in &input.acquisition.rules {
        validate_identity(&rule.kind, "acquisition_kind")?;
        for value in [&rule.reference, &rule.requirement].into_iter().flatten() {
            validate_text(value, "acquisition_value")?;
        }
    }
    if input.acquisition.unlock.requirements.len() > RELIC_MAX_ACQUISITION_RULES {
        return Err("unlock_requirements");
    }
    for requirement in &input.acquisition.unlock.requirements {
        validate_identity(requirement, "unlock_requirement")?;
    }
    if input.references.len() > RELIC_MAX_REFERENCES {
        return Err("references");
    }
    for reference in &input.references {
        validate_identity(&reference.id, "reference_id")?;
        validate_text(&reference.label, "reference_label")?;
    }
    if input.variants.len() > RELIC_MAX_VARIANTS {
        return Err("variants");
    }
    for variant in &input.variants {
        validate_identity(&variant.id, "variant_id")?;
        validate_text(&variant.label, "variant_label")?;
        if let Some(description) = &variant.description {
            validate_text(description, "variant_description")?;
        }
    }
    if input.parameters.len() > RELIC_MAX_PARAMETERS {
        return Err("parameters");
    }
    for parameter in &input.parameters {
        validate_identity(&parameter.id, "parameter_id")?;
        validate_text(&parameter.label, "parameter_label")?;
    }
    if input.counters.len() > RELIC_MAX_COUNTERS {
        return Err("counters");
    }
    for counter in &input.counters {
        validate_identity(&counter.id, "counter_id")?;
        validate_text(&counter.label, "counter_label")?;
    }
    if input.activation.counter_ids.len() > RELIC_MAX_COUNTERS {
        return Err("activation_counter_ids");
    }
    for counter_id in &input.activation.counter_ids {
        validate_identity(counter_id, "activation_counter_id")?;
    }
    if input.activation.kind == RelicActivationKind::Conditional
        && input.activation.condition.is_none()
    {
        return Err("activation_condition");
    }
    if let Some(condition) = &input.activation.condition {
        validate_identity(&condition.id, "activation_condition_id")?;
        validate_text(&condition.label, "activation_condition_label")?;
    }
    if input.triggers.len() > RELIC_MAX_PENDING_TRIGGERS {
        return Err("triggers");
    }
    for trigger in &input.triggers {
        validate_identity(&trigger.id, "trigger_id")?;
        validate_text(&trigger.label, "trigger_label")?;
        if let Some(condition) = &trigger.condition {
            validate_identity(&condition.id, "trigger_condition_id")?;
            validate_text(&condition.label, "trigger_condition_label")?;
        }
    }
    ensure_unique_ids(
        input.counters.iter().map(|value| value.id.as_str()),
        "counter_id",
    )?;
    ensure_unique_ids(
        input.parameters.iter().map(|value| value.id.as_str()),
        "parameter_id",
    )?;
    ensure_unique_ids(
        input.variants.iter().map(|value| value.id.as_str()),
        "variant_id",
    )?;
    ensure_unique_ids(
        input.triggers.iter().map(|value| value.id.as_str()),
        "trigger_id",
    )?;
    if input.activation.counter_ids.iter().any(|counter_id| {
        !input
            .counters
            .iter()
            .any(|counter| &counter.id == counter_id)
    }) {
        return Err("activation_counter_id");
    }
    Ok(())
}

fn ensure_unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    field: &'static str,
) -> Result<(), &'static str> {
    let mut seen = std::collections::BTreeSet::new();
    values
        .into_iter()
        .all(|value| seen.insert(value))
        .then_some(())
        .ok_or(field)
}

/// Measures bounded source-owned static payload bytes conservatively.
pub(super) fn definition_bytes(input: &RelicDefinitionInput) -> usize {
    let mut total = input.relic_id.len() + input.title.len() + input.description.len();
    total += input.origin.kind.len();
    total += input.origin.package_id.as_deref().map_or(0, str::len);
    total += input.origin.package_version.as_deref().map_or(0, str::len);
    for rule in &input.acquisition.rules {
        total += rule.kind.len()
            + rule.reference.as_deref().map_or(0, str::len)
            + rule.requirement.as_deref().map_or(0, str::len);
    }
    total += input
        .acquisition
        .unlock
        .requirements
        .iter()
        .map(String::len)
        .sum::<usize>();
    total += input
        .references
        .iter()
        .map(|reference| reference.id.len() + reference.label.len())
        .sum::<usize>();
    total += input
        .variants
        .iter()
        .map(|variant| {
            variant.id.len()
                + variant.label.len()
                + variant.description.as_deref().map_or(0, str::len)
        })
        .sum::<usize>();
    total += input
        .parameters
        .iter()
        .map(|parameter| parameter.id.len() + parameter.label.len() + parameter.unit.as_str().len())
        .sum::<usize>();
    total += input
        .counters
        .iter()
        .map(|counter| counter.id.len() + counter.label.len() + counter.unit.as_str().len())
        .sum::<usize>();
    total += input
        .triggers
        .iter()
        .map(|trigger| trigger.id.len() + trigger.label.len())
        .sum::<usize>();
    total
}
