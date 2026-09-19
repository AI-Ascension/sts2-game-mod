// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::SettingsReferenceError;
use super::definition::SettingDefinitionInput;
use super::model::{
    RunConfigurationLink, SETTINGS_MAX_CONSTRAINTS, SETTINGS_MAX_KEYS, SETTINGS_MAX_OPTIONS,
    SETTINGS_MAX_REFERENCES, SettingConstraint, SettingOption, SettingValue, SettingValueField,
    SettingsLevel, SettingsReadSeam, SettingsSemanticReferenceKind, SettingsSensitivity,
    SettingsValueType, SettingsVisibility, validate_identity, validate_keys, validate_text,
};

/// Rejects any definition that would publish an unrepresentable or unsafe settings value.
pub(super) fn validate_definition(
    input: &SettingDefinitionInput,
) -> Result<(), SettingsReferenceError> {
    validate_identity(&input.setting_id, "setting_id")?;
    validate_text(&input.label, "label")?;
    if let Some(description) = &input.description {
        validate_text(description, "description")?;
    }
    if input.constraints.len() > SETTINGS_MAX_CONSTRAINTS {
        return Err(SettingsReferenceError::InvalidInput("constraints"));
    }
    if input.references.len() > SETTINGS_MAX_REFERENCES {
        return Err(SettingsReferenceError::InvalidInput("references"));
    }
    validate_constraints(input)?;
    validate_values(input)?;
    validate_withholding(input)?;
    validate_run_link(input)
}

fn validate_constraints(input: &SettingDefinitionInput) -> Result<(), SettingsReferenceError> {
    let mut seen = BTreeSet::new();
    for constraint in &input.constraints {
        if !seen.insert(constraint.kind()) {
            return Err(SettingsReferenceError::InvalidInput("duplicate_constraint"));
        }
        match constraint {
            SettingConstraint::Range { min, max, step } => {
                validate_range(input, *min, *max, *step)?
            }
            SettingConstraint::Options(options) => validate_options(input, options)?,
            SettingConstraint::MaxLength { max } => {
                require_type(input, SettingsValueType::Text)?;
                if *max == 0 {
                    return Err(SettingsReferenceError::InvalidInput("max_length"));
                }
            }
            SettingConstraint::Pattern { pattern_id } => {
                require_type(input, SettingsValueType::Text)?;
                validate_identity(pattern_id, "pattern_id")?;
            }
        }
    }
    Ok(())
}

fn require_type(
    input: &SettingDefinitionInput,
    expected: SettingsValueType,
) -> Result<(), SettingsReferenceError> {
    if input.value_type == expected {
        return Ok(());
    }
    Err(SettingsReferenceError::ConstraintValueTypeMismatch(
        input.setting_id.clone(),
    ))
}

fn validate_range(
    input: &SettingDefinitionInput,
    min: i64,
    max: i64,
    step: Option<i64>,
) -> Result<(), SettingsReferenceError> {
    require_type(input, SettingsValueType::Integer)?;
    if min > max || step.is_some_and(|step| step <= 0) {
        return Err(SettingsReferenceError::InvalidInput("range"));
    }
    Ok(())
}

fn validate_options(
    input: &SettingDefinitionInput,
    options: &[SettingOption],
) -> Result<(), SettingsReferenceError> {
    require_type(input, SettingsValueType::Enumeration)?;
    if options.is_empty() || options.len() > SETTINGS_MAX_OPTIONS {
        return Err(SettingsReferenceError::InvalidInput("options"));
    }
    let mut ids = BTreeSet::new();
    for option in options {
        validate_identity(&option.option_id, "option_id")?;
        if !ids.insert(option.option_id.as_str()) {
            return Err(SettingsReferenceError::InvalidInput("duplicate_option"));
        }
        validate_text(option.label.value().unwrap_or_default(), "option_label")?;
    }
    Ok(())
}

fn validate_values(input: &SettingDefinitionInput) -> Result<(), SettingsReferenceError> {
    for field in [
        &input.default_value,
        &input.stored_value,
        &input.effective_value,
    ] {
        validate_field(input, field)?;
    }
    Ok(())
}

fn validate_field(
    input: &SettingDefinitionInput,
    field: &SettingValueField,
) -> Result<(), SettingsReferenceError> {
    let Some(value) = field.get() else {
        return Ok(());
    };
    if field.seam != expected_seam(input.level) {
        return Err(SettingsReferenceError::InvalidInput("value_seam"));
    }
    if input.value_type == SettingsValueType::Unknown || value.value_type() != input.value_type {
        return Err(SettingsReferenceError::ValueTypeMismatch(
            input.setting_id.clone(),
        ));
    }
    validate_value_shape(input, value)
}

fn expected_seam(level: SettingsLevel) -> SettingsReadSeam {
    match level {
        SettingsLevel::Global => SettingsReadSeam::OwnerSettingsApi,
        SettingsLevel::Profile => SettingsReadSeam::ProfilePreferenceApi,
        SettingsLevel::Addon => SettingsReadSeam::AddonSettingsApi,
        SettingsLevel::Unknown => SettingsReadSeam::Unknown,
    }
}

fn validate_value_shape(
    input: &SettingDefinitionInput,
    value: &SettingValue,
) -> Result<(), SettingsReferenceError> {
    match value {
        SettingValue::Boolean(_) => Ok(()),
        SettingValue::Integer(number) => validate_integer(input, *number),
        SettingValue::Enumeration(option_id) => validate_option_id(input, option_id),
        SettingValue::Keys(keys) => {
            if input.value_type != SettingsValueType::KeyBinding {
                return Err(SettingsReferenceError::ValueTypeMismatch(
                    input.setting_id.clone(),
                ));
            }
            validate_keys(keys, "keys").map_err(|_| SettingsReferenceError::InvalidInput("keys"))
        }
        SettingValue::Text(text) => validate_text_value(input, text),
    }
}

fn validate_integer(
    input: &SettingDefinitionInput,
    number: i64,
) -> Result<(), SettingsReferenceError> {
    for constraint in &input.constraints {
        if let SettingConstraint::Range { min, max, .. } = constraint
            && (number < *min || number > *max)
        {
            return Err(SettingsReferenceError::ValueOutOfRange {
                setting_id: input.setting_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_option_id(
    input: &SettingDefinitionInput,
    option_id: &str,
) -> Result<(), SettingsReferenceError> {
    validate_identity(option_id, "option_id")?;
    for constraint in &input.constraints {
        if let SettingConstraint::Options(options) = constraint {
            let known = options.iter().any(|option| option.option_id == option_id);
            if !known {
                return Err(SettingsReferenceError::UnknownOption {
                    setting_id: input.setting_id.clone(),
                    option_id: option_id.to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn validate_text_value(
    input: &SettingDefinitionInput,
    text: &str,
) -> Result<(), SettingsReferenceError> {
    if text.len() > SETTINGS_MAX_KEYS * 64 {
        return Err(SettingsReferenceError::InvalidInput("text"));
    }
    for constraint in &input.constraints {
        if let SettingConstraint::MaxLength { max } = constraint
            && text.len() > *max
        {
            return Err(SettingsReferenceError::ValueOutOfRange {
                setting_id: input.setting_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_withholding(input: &SettingDefinitionInput) -> Result<(), SettingsReferenceError> {
    let hidden = matches!(
        input.visibility,
        SettingsVisibility::Hidden | SettingsVisibility::Unknown
    );
    let private = input.sensitivity == SettingsSensitivity::Private;
    if private && input.visibility == SettingsVisibility::Visible {
        return Err(SettingsReferenceError::ValueMustBeWithheld(
            input.setting_id.clone(),
        ));
    }
    if !hidden && !private {
        return Ok(());
    }
    for field in [
        &input.default_value,
        &input.stored_value,
        &input.effective_value,
    ] {
        if field.get().is_some() {
            return Err(SettingsReferenceError::ValueMustBeWithheld(
                input.setting_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_run_link(input: &SettingDefinitionInput) -> Result<(), SettingsReferenceError> {
    let reference = input.references.iter().find(|reference| {
        matches!(
            reference.kind,
            SettingsSemanticReferenceKind::RunConfiguration
        )
    });
    match &input.run_link {
        RunConfigurationLink::RunAffecting { configuration_id } => {
            validate_identity(configuration_id, "configuration_id")?;
            match reference {
                Some(reference) if reference.id == *configuration_id => Ok(()),
                _ => Err(SettingsReferenceError::MissingRunReference(
                    input.setting_id.clone(),
                )),
            }
        }
        RunConfigurationLink::NotRunAffecting | RunConfigurationLink::Unavailable(_) => {
            if reference.is_some() {
                return Err(SettingsReferenceError::UnexpectedRunReference(
                    input.setting_id.clone(),
                ));
            }
            Ok(())
        }
    }
}
