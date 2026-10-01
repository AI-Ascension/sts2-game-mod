// SPDX-License-Identifier: MIT

//! The bounds on free-form text and identity fields.
//!
//! The two are bounded differently because the schema bounds them differently: `maxLength`
//! counts Unicode characters, while the identity pattern is ASCII-only and so counts bytes.

/// Maximum length of one player-visible text field, in Unicode characters.
///
/// Mirrors the schema's `#/$defs/text` `maxLength`, which counts characters rather than UTF-8
/// bytes, so a 512-character non-ASCII name is admitted.
pub const RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS: usize = 512;
/// Maximum length of one identity field, in bytes.
///
/// Identities are ASCII-only (`^[A-Za-z0-9_.:/-]{1,512}$`), so bytes and characters coincide.
pub const RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES: usize = 512;

/// Legacy name for the 512 limit, kept so existing callers still compile.
///
/// It never described text correctly: text is bounded in Unicode scalar values, not bytes, and
/// nothing validates text by byte length any more. Only the numeric value is preserved.
#[deprecated(
    since = "0.0.0",
    note = "use RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS for text or \
            RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES for identities"
)]
pub const RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES: usize = 512;

pub(super) fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

pub(super) fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS
        && !value.chars().any(char::is_control)
}
