// SPDX-License-Identifier: MIT

//! Manifest resolution for the content references a party record carries.

use crate::ContentManifest;

use super::super::CoopError;

/// Requires one namespaced reference to resolve inside the content manifest.
///
/// A member entry whose content the selected manifest no longer carries is refused rather than
/// dropped, because a published party row that silently lost a relic would overstate what the
/// source reported.
pub(super) fn validate_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), CoopError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(CoopError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}
