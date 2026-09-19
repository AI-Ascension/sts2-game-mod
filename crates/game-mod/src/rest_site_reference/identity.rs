// SPDX-License-Identifier: MIT

use super::{
    error::RestSiteError,
    field::RestText,
    model::{REST_MAX_IDENTITY_BYTES, REST_MAX_TEXT_BYTES},
};

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(value: &str, field: &'static str) -> Result<(), RestSiteError> {
    if value.is_empty()
        || value.len() > REST_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(RestSiteError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), RestSiteError> {
    if value.is_empty() || value.len() > REST_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(RestSiteError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized text value that may be explicitly withheld.
pub(super) fn validate_text_value(
    value: &RestText,
    field: &'static str,
) -> Result<(), RestSiteError> {
    if let RestText::Available(value) = value {
        validate_text(value, field)?;
    }
    Ok(())
}
