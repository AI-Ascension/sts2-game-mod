// SPDX-License-Identifier: MIT

use super::{
    error::SelectionError,
    field::SelectionText,
    model::{SEL_MAX_IDENTITY_BYTES, SEL_MAX_TEXT_BYTES},
};

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(value: &str, field: &'static str) -> Result<(), SelectionError> {
    if value.is_empty()
        || value.len() > SEL_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(SelectionError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), SelectionError> {
    if value.is_empty() || value.len() > SEL_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(SelectionError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized text value that may be explicitly withheld.
pub(super) fn validate_text_value(
    value: &SelectionText,
    field: &'static str,
) -> Result<(), SelectionError> {
    if let SelectionText::Available(value) = value {
        validate_text(value, field)?;
    }
    Ok(())
}
