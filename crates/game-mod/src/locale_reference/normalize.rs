// SPDX-License-Identifier: MIT

use super::{LOCALE_REFERENCE_MAX_IDENTITY_BYTES, LocaleCatalogError};

/// Presentation tokens that would let stored game text execute inside a structured consumer.
///
/// Game and mod strings are data, so a rendered value is rejected rather than repaired when it
/// carries one of these.  Ordinary `<`, `>` and `&` remain allowed: this slice normalizes
/// executable presentation, it does not strip markup a locale may legitimately render.
const EXECUTABLE_TOKENS: [&str; 3] = ["<script", "javascript:", "data:text/html"];

/// Returns whether a rendered value carries executable or otherwise unsafe presentation.
///
/// Unicode, right-to-left text and reference links are preserved.  Only control characters that
/// cannot appear in rendered text, and the executable tokens above, are refused.
#[must_use]
pub(super) fn is_unsafe_presentation(value: &str) -> bool {
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\t'))
    {
        return true;
    }
    let lowered = value.to_ascii_lowercase();
    EXECUTABLE_TOKENS
        .iter()
        .any(|token| lowered.contains(token))
}

/// Validates one literal rendered value without changing its meaning.
pub(super) fn validate_presentation(
    value: &str,
    field: &'static str,
) -> Result<(), LocaleCatalogError> {
    if is_unsafe_presentation(value) {
        return Err(LocaleCatalogError::UnsafePresentation(field));
    }
    Ok(())
}

/// Validates one effect amount.
///
/// An amount is always carried verbatim, so it must already be a non-empty rendered number and is
/// never reformatted, rounded or recomputed here.
pub(super) fn validate_effect_amount(kind: &str, amount: &str) -> Result<(), LocaleCatalogError> {
    if amount.is_empty()
        || amount.len() > LOCALE_REFERENCE_MAX_IDENTITY_BYTES
        || amount.chars().any(char::is_control)
        || !amount.chars().any(|character| character.is_ascii_digit())
    {
        return Err(LocaleCatalogError::InvalidInput("effect_amount"));
    }
    validate_presentation(kind, "effect_kind")
}
