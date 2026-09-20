// SPDX-License-Identifier: MIT

//! Manifest resolution for the content identities an event names.

use crate::ContentManifest;

use super::super::SemanticEventError;
use super::super::model::SemanticReference;

/// Requires one namespaced reference to resolve inside the content manifest.
///
/// An event naming content the selected manifest no longer carries is refused rather than dropped,
/// because a published history that silently lost a card play would overstate what was observed.
pub(super) fn validate_reference(
    manifest: &ContentManifest,
    reference: &SemanticReference,
) -> Result<(), SemanticEventError> {
    if reference.entity_kind.is_empty() || reference.namespaced_id.is_empty() {
        return Err(SemanticEventError::InvalidInput("reference"));
    }
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == reference.entity_kind
            && definition.namespaced_id == reference.namespaced_id
    }) {
        Ok(())
    } else {
        Err(SemanticEventError::UnknownManifestReference {
            entity_kind: reference.entity_kind.clone(),
            namespaced_id: reference.namespaced_id.clone(),
        })
    }
}
