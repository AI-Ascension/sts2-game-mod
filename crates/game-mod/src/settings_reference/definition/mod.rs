// SPDX-License-Identifier: MIT

use super::{
    RunConfigurationLink, SettingConstraint, SettingValueField, SettingsCatalogBinding,
    SettingsCategory, SettingsDefinitionReference, SettingsLevel, SettingsRestartState,
    SettingsSemanticReference, SettingsSensitivity, SettingsText, SettingsValueType,
    SettingsVisibility,
};

/// Typed source-owned setting record before it is bound to a catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingDefinitionInput {
    /// Namespaced setting definition identity.
    pub setting_id: String,
    /// Localized setting label.
    pub label: String,
    /// Localized setting description, when the source supplies one.
    pub description: Option<String>,
    /// Owner-defined category.
    pub category: SettingsCategory,
    /// Scope the preference is stored at.
    pub level: SettingsLevel,
    /// Declared value shape.
    pub value_type: SettingsValueType,
    /// Whether the setting may be discovered publicly.
    pub sensitivity: SettingsSensitivity,
    /// Owner-defined visibility.
    pub visibility: SettingsVisibility,
    /// Whether a stored change needs a restart.
    pub restart: SettingsRestartState,
    /// Link to the separate effective run configuration.
    pub run_link: RunConfigurationLink,
    /// Value the owner ships when no preference was stored.
    pub default_value: SettingValueField,
    /// Persisted preference value.
    pub stored_value: SettingValueField,
    /// Runtime-effective value currently applied.
    pub effective_value: SettingValueField,
    /// Declared bounds for the value.
    pub constraints: Vec<SettingConstraint>,
    /// Owner-defined semantic references.
    pub references: Vec<SettingsSemanticReference>,
}

/// Immutable setting definition bound to one catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingDefinition {
    /// Exact static definition reference.
    pub reference: SettingsDefinitionReference,
    /// Namespaced setting definition identity.
    pub setting_id: String,
    /// Localized setting label.
    pub label: SettingsText,
    /// Localized setting description or an explicit non-value.
    pub description: SettingsText,
    /// Owner-defined category.
    pub category: SettingsCategory,
    /// Scope the preference is stored at.
    pub level: SettingsLevel,
    /// Declared value shape.
    pub value_type: SettingsValueType,
    /// Whether the setting may be discovered publicly.
    pub sensitivity: SettingsSensitivity,
    /// Owner-defined visibility.
    pub visibility: SettingsVisibility,
    /// Whether a stored change needs a restart.
    pub restart: SettingsRestartState,
    /// Link to the separate effective run configuration.
    pub run_link: RunConfigurationLink,
    /// Value the owner ships when no preference was stored.
    pub default_value: SettingValueField,
    /// Persisted preference value.
    pub stored_value: SettingValueField,
    /// Runtime-effective value currently applied.
    pub effective_value: SettingValueField,
    /// Declared bounds for the value.
    pub constraints: Vec<SettingConstraint>,
    /// Owner-defined semantic references.
    pub references: Vec<SettingsSemanticReference>,
}

impl SettingDefinition {
    pub(crate) fn from_input(
        binding: &SettingsCatalogBinding,
        input: SettingDefinitionInput,
    ) -> Self {
        let description = match input.description {
            Some(description) => SettingsText::from_validated(description),
            None => SettingsText::Unavailable(super::SettingsUnavailableReason::NotObserved),
        };
        Self {
            reference: SettingsDefinitionReference {
                catalog: binding.clone(),
                setting_id: input.setting_id.clone(),
            },
            setting_id: input.setting_id,
            label: SettingsText::from_validated(input.label),
            description,
            category: input.category,
            level: input.level,
            value_type: input.value_type,
            sensitivity: input.sensitivity,
            visibility: input.visibility,
            restart: input.restart,
            run_link: input.run_link,
            default_value: input.default_value,
            stored_value: input.stored_value,
            effective_value: input.effective_value,
            constraints: input.constraints,
            references: input.references,
        }
    }
}

/// Estimates the aggregate bytes retained for one definition.
pub(super) fn definition_bytes(input: &SettingDefinitionInput) -> usize {
    let mut total = input.setting_id.len()
        + input.label.len()
        + input.description.as_ref().map_or(0, String::len);
    total += value_bytes(&input.default_value)
        + value_bytes(&input.stored_value)
        + value_bytes(&input.effective_value);
    for constraint in &input.constraints {
        total += constraint_bytes(constraint);
    }
    for reference in &input.references {
        total += reference.id.len() + reference.label.value().map_or(0, str::len);
    }
    total
}

fn constraint_bytes(constraint: &SettingConstraint) -> usize {
    match constraint {
        SettingConstraint::Range { .. } => 24,
        SettingConstraint::Options(options) => options
            .iter()
            .map(|option| option.option_id.len() + option.label.value().map_or(0, str::len))
            .sum(),
        SettingConstraint::MaxLength { .. } => 16,
        SettingConstraint::Pattern { pattern_id } => pattern_id.len(),
    }
}

fn value_bytes(field: &SettingValueField) -> usize {
    match field.get() {
        None => 0,
        Some(super::SettingValue::Boolean(_)) => 1,
        Some(super::SettingValue::Integer(_)) => 8,
        Some(super::SettingValue::Enumeration(option)) => option.len(),
        Some(super::SettingValue::Keys(keys)) => keys.iter().map(String::len).sum(),
        Some(super::SettingValue::Text(text)) => text.len(),
    }
}
