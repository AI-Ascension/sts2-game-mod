// SPDX-License-Identifier: MIT

//! Manifest resolution for the content references a retained result carries.

use crate::ContentManifest;

use super::super::RunResultError;

/// Requires one namespaced reference to resolve inside the content manifest.
///
/// An ending entry whose content the selected manifest no longer carries is refused rather than
/// dropped, because a published deck that silently lost a card would overstate what was retained.
pub(super) fn validate_manifest_reference(
    manifest: &ContentManifest,
    entity_kind: &str,
    namespaced_id: &str,
) -> Result<(), RunResultError> {
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == namespaced_id
    }) {
        Ok(())
    } else {
        Err(RunResultError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}
