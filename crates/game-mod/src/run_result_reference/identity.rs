// SPDX-License-Identifier: MIT

//! Opaque identifiers, validated so no save path or profile path can be published as an identity.

/// Returns whether every byte of an opaque identifier is path-free and printable.
///
/// A separator, a drive letter or a traversal segment is refused rather than sanitized: a value
/// that looks like a filesystem path is not an opaque identity, and publishing it would disclose
/// where a save lives.
#[must_use]
pub fn is_opaque_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= super::RUN_RESULT_MAX_IDENTITY_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
        && !value
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b'_' | b'-' | b'#'))
}

/// Validates an opaque result, run, summary or profile identity.
pub(super) fn validate_opaque_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::RunResultError> {
    if is_opaque_identity(value) {
        Ok(())
    } else {
        Err(super::RunResultError::NonOpaqueIdentity(field))
    }
}
