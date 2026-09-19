// SPDX-License-Identifier: MIT

use super::{
    error::RunConfigurationError,
    model::{
        RUN_CONFIGURATION_FORBIDDEN_IDENTITY_PREFIXES, RUN_CONFIGURATION_MAX_IDENTIFIER_BYTES,
        RUN_CONFIGURATION_MAX_TEXT_BYTES,
    },
};

/// Validates an owner-defined namespaced identity.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), RunConfigurationError> {
    if value.is_empty() || value.len() > RUN_CONFIGURATION_MAX_IDENTIFIER_BYTES {
        return Err(RunConfigurationError::InvalidInput(field));
    }
    let shaped = value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_.:".contains(character));
    if !shaped {
        return Err(RunConfigurationError::InvalidInput(field));
    }
    validate_not_rng_state(value)
}

/// Rejects identities that name RNG state outside the dedicated RNG surfaces.
pub(super) fn validate_not_rng_state(value: &str) -> Result<(), RunConfigurationError> {
    let namespace = value.split(':').next().unwrap_or(value);
    if RUN_CONFIGURATION_FORBIDDEN_IDENTITY_PREFIXES.contains(&namespace) {
        return Err(RunConfigurationError::RngStateNotPermitted(
            value.to_owned(),
        ));
    }
    Ok(())
}

/// Validates a bounded localized text value.
pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), RunConfigurationError> {
    if value.is_empty() || value.len() > RUN_CONFIGURATION_MAX_TEXT_BYTES {
        return Err(RunConfigurationError::InvalidInput(field));
    }
    if value.chars().any(char::is_control) {
        return Err(RunConfigurationError::InvalidInput(field));
    }
    Ok(())
}
