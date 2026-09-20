// SPDX-License-Identifier: MIT

//! Opaque identity acceptance for event, actor, target and namespace keys.

use super::model::{SEMANTIC_MAX_IDENTITY_BYTES, SEMANTIC_MAX_IDENTITY_SEGMENTS};

/// Returns whether one identity is an opaque token this boundary will carry.
///
/// An identity is opaque: it must be non-empty, bounded, free of control bytes and path separators,
/// and free of the traversal segments that would make it addressable on a host filesystem. An event
/// identity that could be read as a path is refused so this vocabulary can never be turned into a
/// host read.
#[must_use]
pub fn is_opaque_semantic_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= SEMANTIC_MAX_IDENTITY_BYTES
        && !value.contains(|character: char| character.is_control())
        && !value.contains('/')
        && !value.contains('\\')
        && !value.contains(':')
        && !value.contains("..")
        && !value.to_ascii_lowercase().starts_with("file")
        && value.split('.').count() <= SEMANTIC_MAX_IDENTITY_SEGMENTS
}

/// Validates one opaque identity, naming the field in the refusal.
pub(super) fn validate_opaque_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::SemanticEventError> {
    if is_opaque_semantic_identity(value) {
        Ok(())
    } else {
        Err(super::SemanticEventError::NonOpaqueIdentity(field))
    }
}
