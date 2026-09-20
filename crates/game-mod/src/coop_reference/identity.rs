// SPDX-License-Identifier: MIT

//! Opaque co-op identifiers, validated so no save path or player path is published as an identity.

/// Returns whether every byte of an opaque identifier is path-free and printable.
///
/// A separator, a drive letter or a traversal segment is refused rather than sanitized: a value
/// that looks like a filesystem path is not an opaque identity, and publishing it would disclose
/// where a save, a profile or a peer record lives.
#[must_use]
pub fn is_opaque_coop_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= super::COOP_MAX_IDENTITY_BYTES
        && !value.chars().any(char::is_control)
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains("..")
        && !value
            .bytes()
            .any(|byte| !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b'_' | b'-' | b'#'))
}

/// Validates an opaque party, peer, effect, resource or scaling-rule identity.
pub(super) fn validate_opaque_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::CoopError> {
    if is_opaque_coop_identity(value) {
        Ok(())
    } else {
        Err(super::CoopError::NonOpaqueIdentity(field))
    }
}
