// SPDX-License-Identifier: MIT

use super::{
    SETTINGS_MAX_KEYS, SettingsFieldStatus, SettingsText, SettingsUnavailableReason,
    validate_identity,
};

/// One closed enumeration option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingOption {
    /// Stable option identity.
    pub option_id: String,
    /// Localized option label.
    pub label: SettingsText,
}

impl SettingOption {
    /// Creates one bounded option.
    pub fn new(option_id: &str, label: &str) -> Result<Self, super::super::SettingsReferenceError> {
        validate_identity(option_id, "option_id")?;
        Ok(Self {
            option_id: option_id.to_owned(),
            label: SettingsText::available(label)?,
        })
    }
}

/// Declared bound for one setting value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingConstraint {
    /// Inclusive integer range with an optional step.
    Range {
        /// Inclusive minimum.
        min: i64,
        /// Inclusive maximum.
        max: i64,
        /// Optional positive step.
        step: Option<i64>,
    },
    /// Closed option set for an enumeration value.
    Options(Vec<SettingOption>),
    /// Maximum byte length of a text value.
    MaxLength {
        /// Maximum accepted bytes.
        max: usize,
    },
    /// Named owner-owned validation pattern for a text value.
    Pattern {
        /// Stable pattern identity; the pattern itself is never copied.
        pattern_id: String,
    },
}

impl SettingConstraint {
    /// Returns the stable constraint kind name.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Range { .. } => "range",
            Self::Options(_) => "options",
            Self::MaxLength { .. } => "max_length",
            Self::Pattern { .. } => "pattern",
        }
    }
}

/// Owner-defined reference kind carried by a setting definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsSemanticReferenceKind {
    /// The separate effective run-configuration feature.
    RunConfiguration,
    /// A definition in another manifest family.
    Content {
        /// Manifest family identity.
        entity_kind: String,
    },
    /// An owner rule identity that is not a manifest definition.
    Rule,
    /// The source could not classify the reference.
    Unknown,
}

/// One owner-defined semantic reference with a label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsSemanticReference {
    /// Reference kind.
    pub kind: SettingsSemanticReferenceKind,
    /// Referenced owner identity.
    pub id: String,
    /// Localized reference label.
    pub label: SettingsText,
}

/// Link from a setting to the separate effective run configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunConfigurationLink {
    /// The setting changes a value the run configuration also reports.
    RunAffecting {
        /// Manifest run-configuration definition identity this setting affects.
        configuration_id: String,
    },
    /// The setting does not affect run resolution.
    NotRunAffecting,
    /// The link could not be established for the stated reason.
    Unavailable(SettingsUnavailableReason),
}

impl RunConfigurationLink {
    /// Returns the coarse availability status of the link.
    #[must_use]
    pub const fn status(&self) -> SettingsFieldStatus {
        match self {
            Self::RunAffecting { .. } => SettingsFieldStatus::Available,
            Self::NotRunAffecting => SettingsFieldStatus::NotApplicable,
            Self::Unavailable(reason) => reason.status(),
        }
    }
}

/// Returns whether a key list is bounded and free of control characters.
pub(crate) fn validate_keys(
    keys: &[String],
    field: &'static str,
) -> Result<(), super::super::SettingsReferenceError> {
    if keys.is_empty() || keys.len() > SETTINGS_MAX_KEYS {
        return Err(super::super::SettingsReferenceError::InvalidInput(field));
    }
    for key in keys {
        validate_identity(key, field)?;
    }
    Ok(())
}
