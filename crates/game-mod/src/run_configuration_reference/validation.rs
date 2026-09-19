// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    error::RunConfigurationError,
    field::RunConfigurationUnavailableReason,
    identity::{validate_identity, validate_text},
    model::{
        RUN_CONFIGURATION_MAX_MODIFIERS, RUN_CONFIGURATION_MAX_RECORD_BYTES,
        RunConfigurationRecordInput, RunFieldKind, RunFieldRecord, RunModifierInput,
        RunModifierState, RunProvenance, RunSeedPolicy, RunSensitivity, RunVisibility,
    },
    value::{RunValue, active_modifier_ids, altered_kinds, validate_value},
};

/// Rejects any record that would publish an unsupported or unsafe configuration.
pub(super) fn validate_record(
    input: &RunConfigurationRecordInput,
) -> Result<(), RunConfigurationError> {
    let run_id = input.run_id.as_str();
    validate_identity(run_id, "run_id")?;
    validate_identity(&input.instance_id, "instance_id")?;
    if input.revision == 0 {
        return Err(RunConfigurationError::InvalidInput("revision"));
    }
    if input.fields.len() > RunFieldKind::ALL.len() {
        return Err(RunConfigurationError::InvalidInput("fields"));
    }
    if input.modifiers.len() > RUN_CONFIGURATION_MAX_MODIFIERS {
        return Err(RunConfigurationError::InvalidInput("modifiers"));
    }
    validate_modifiers(&input.modifiers)?;
    let mut seen = BTreeSet::new();
    for field in &input.fields {
        if !seen.insert(field.kind) {
            return Err(RunConfigurationError::DuplicateField {
                run_id: run_id.to_owned(),
                kind: field.kind,
            });
        }
        validate_field(run_id, field)?;
    }
    for required in RunFieldKind::ALL
        .into_iter()
        .filter(|kind| kind.is_required())
    {
        if !seen.contains(&required) {
            return Err(RunConfigurationError::MissingRequiredField {
                run_id: run_id.to_owned(),
                kind: required,
            });
        }
    }
    validate_modifier_set(run_id, input)?;
    validate_seed_policy(run_id, input)?;
    total_bytes(input)
}

fn validate_modifiers(modifiers: &[RunModifierInput]) -> Result<(), RunConfigurationError> {
    let mut seen = BTreeSet::new();
    for modifier in modifiers {
        let invalid = || RunConfigurationError::InvalidModifier(modifier.modifier_id.clone());
        validate_identity(&modifier.modifier_id, "modifier_id")?;
        validate_text(&modifier.label, "modifier_label")?;
        if !seen.insert(modifier.modifier_id.as_str()) {
            return Err(invalid());
        }
        if modifier.alters.len() > RunFieldKind::ALL.len() {
            return Err(invalid());
        }
        if modifier.state == RunModifierState::Unknown && !modifier.alters.is_empty() {
            return Err(invalid());
        }
    }
    Ok(())
}

fn validate_field(run_id: &str, field: &RunFieldRecord) -> Result<(), RunConfigurationError> {
    for value in [field.requested.value(), field.settled.value()]
        .into_iter()
        .flatten()
    {
        validate_value(field.kind, value)
            .map_err(|error| shape_error(run_id, field.kind, error))?;
    }
    if field.kind.is_required()
        && field.settled.reason() == Some(RunConfigurationUnavailableReason::NotApplicable)
    {
        return Err(RunConfigurationError::NotApplicableRequiredField {
            run_id: run_id.to_owned(),
            kind: field.kind,
        });
    }
    let hidden = matches!(
        field.visibility,
        RunVisibility::Hidden | RunVisibility::Unknown
    );
    let private = field.sensitivity == RunSensitivity::Private;
    if private && field.visibility == RunVisibility::Public {
        return Err(RunConfigurationError::ValueMustBeWithheld(
            run_id.to_owned(),
        ));
    }
    let carries_value = field.requested.value().is_some() || field.settled.value().is_some();
    if (hidden || private) && carries_value {
        return Err(RunConfigurationError::ValueMustBeWithheld(
            run_id.to_owned(),
        ));
    }
    Ok(())
}

fn shape_error(
    run_id: &str,
    kind: RunFieldKind,
    error: RunConfigurationError,
) -> RunConfigurationError {
    match error {
        RunConfigurationError::FieldShapeMismatch { .. } => {
            RunConfigurationError::FieldShapeMismatch {
                run_id: run_id.to_owned(),
                kind,
            }
        }
        other => other,
    }
}

fn validate_modifier_set(
    run_id: &str,
    input: &RunConfigurationRecordInput,
) -> Result<(), RunConfigurationError> {
    let declared = active_modifier_ids(&input.modifiers);
    if let Some(RunValue::Identifiers(identifiers)) = input
        .fields
        .iter()
        .find(|field| field.kind == RunFieldKind::Modifiers)
        .and_then(|field| field.settled.value())
    {
        let mut observed = identifiers.clone();
        observed.sort();
        if observed != declared {
            return Err(RunConfigurationError::ModifierSetMismatch(
                run_id.to_owned(),
            ));
        }
    }
    for kind in altered_kinds(&input.modifiers) {
        let Some(field) = input.fields.iter().find(|field| field.kind == kind) else {
            return Err(RunConfigurationError::ModifiedFieldEchoesRequest {
                run_id: run_id.to_owned(),
                kind,
            });
        };
        if field.provenance != RunProvenance::SettledHost {
            return Err(RunConfigurationError::ModifiedFieldEchoesRequest {
                run_id: run_id.to_owned(),
                kind,
            });
        }
    }
    Ok(())
}

fn validate_seed_policy(
    run_id: &str,
    input: &RunConfigurationRecordInput,
) -> Result<(), RunConfigurationError> {
    let mismatch = || RunConfigurationError::SeedPolicyMismatch(run_id.to_owned());
    let seed_field = input
        .fields
        .iter()
        .find(|field| field.kind == RunFieldKind::Seed);
    match (input.seed_policy, seed_field) {
        (RunSeedPolicy::Visible, Some(field)) => {
            if !matches!(field.settled.value(), Some(RunValue::Seed(_)))
                || field.visibility != RunVisibility::Public
            {
                return Err(mismatch());
            }
        }
        (RunSeedPolicy::Visible, None) => return Err(mismatch()),
        (RunSeedPolicy::Withheld, Some(field)) => {
            if field.settled.value().is_some() || field.visibility == RunVisibility::Public {
                return Err(mismatch());
            }
        }
        (RunSeedPolicy::Unknown, Some(field)) => {
            if field.settled.value().is_some() {
                return Err(mismatch());
            }
        }
        (RunSeedPolicy::Withheld | RunSeedPolicy::Unknown, None) => {}
    }
    Ok(())
}

fn total_bytes(input: &RunConfigurationRecordInput) -> Result<(), RunConfigurationError> {
    let mut total = input.run_id.len() + input.instance_id.len();
    for field in &input.fields {
        total += value_bytes(field.requested.value()) + value_bytes(field.settled.value());
    }
    for modifier in &input.modifiers {
        total += modifier.modifier_id.len() + modifier.label.len() + modifier.alters.len() * 16;
    }
    if total > RUN_CONFIGURATION_MAX_RECORD_BYTES {
        return Err(RunConfigurationError::DefinitionTooLarge {
            limit: RUN_CONFIGURATION_MAX_RECORD_BYTES,
            actual: total,
        });
    }
    Ok(())
}

fn value_bytes(value: Option<&RunValue>) -> usize {
    match value {
        None => 0,
        Some(
            RunValue::Mode(_) | RunValue::Difficulty(_) | RunValue::Toggle(_) | RunValue::Count(_),
        ) => 8,
        Some(RunValue::Identifier(text) | RunValue::Text(text) | RunValue::Seed(text)) => {
            text.len()
        }
        Some(RunValue::Identifiers(identifiers)) => identifiers.iter().map(String::len).sum(),
    }
}
