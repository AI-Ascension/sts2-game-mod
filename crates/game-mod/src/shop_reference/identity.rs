// SPDX-License-Identifier: MIT

use super::{
    error::ShopCatalogError,
    field::ShopText,
    model::{SHOP_MAX_IDENTITY_BYTES, SHOP_MAX_TEXT_BYTES},
};

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(value: &str, field: &'static str) -> Result<(), ShopCatalogError> {
    if value.is_empty()
        || value.len() > SHOP_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(ShopCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), ShopCatalogError> {
    if value.is_empty() || value.len() > SHOP_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(ShopCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized text value that may be explicitly withheld.
pub(super) fn validate_text_value(
    value: &ShopText,
    field: &'static str,
) -> Result<(), ShopCatalogError> {
    if let ShopText::Available(value) = value {
        validate_text(value, field)?;
    }
    Ok(())
}
