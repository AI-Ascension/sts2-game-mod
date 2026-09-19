// SPDX-License-Identifier: MIT

use crate::ContentManifest;

use super::super::{
    definition::SelectionDefinitionInput,
    error::SelectionError,
    field::SelectionField,
    model::{
        SELECTION_REFERENCE_CARD_KIND, SELECTION_REFERENCE_EFFECT_KIND,
        SELECTION_REFERENCE_ENTITY_KIND, SELECTION_REFERENCE_PLAYER_KIND,
        SELECTION_REFERENCE_POTION_KIND, SELECTION_REFERENCE_RELIC_KIND, SelectionReferenceKind,
        SelectionSemanticReference,
    },
};

/// Requires every reference of one definition to resolve inside the content manifest.
pub(in crate::selection_reference) fn validate_manifest_references(
    input: &SelectionDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), SelectionError> {
    ensure_all(manifest, &input.references)?;
    for candidate in &input.candidates {
        ensure_manifest_reference(manifest, &candidate.definition)?;
        ensure_all(manifest, &candidate.references)?;
        ensure_all(manifest, &candidate.eligibility.references)?;
        for effect in &candidate.prospective {
            ensure_all(manifest, &effect.references)?;
            if let SelectionField::Available(target) = &effect.target {
                ensure_manifest_reference(manifest, target)?;
            }
        }
    }
    Ok(())
}

fn ensure_all(
    manifest: &ContentManifest,
    references: &[SelectionSemanticReference],
) -> Result<(), SelectionError> {
    for reference in references {
        ensure_manifest_reference(manifest, reference)?;
    }
    Ok(())
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    reference: &SelectionSemanticReference,
) -> Result<(), SelectionError> {
    let Some(entity_kind) = reference_kind_token(&reference.kind) else {
        return Ok(());
    };
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == reference.id
    }) {
        Ok(())
    } else {
        Err(SelectionError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: reference.id.clone(),
        })
    }
}

/// Returns the manifest entity family a typed reference resolves to, when it names one.
///
/// Candidate references are local to their owning selection and therefore name no manifest family;
/// an unclassified reference is deliberately `None` rather than assumed.
#[must_use]
pub(super) fn reference_kind_token(kind: &SelectionReferenceKind) -> Option<&str> {
    match kind {
        SelectionReferenceKind::Selection => Some(SELECTION_REFERENCE_ENTITY_KIND),
        SelectionReferenceKind::Effect => Some(SELECTION_REFERENCE_EFFECT_KIND),
        SelectionReferenceKind::Card => Some(SELECTION_REFERENCE_CARD_KIND),
        SelectionReferenceKind::Relic => Some(SELECTION_REFERENCE_RELIC_KIND),
        SelectionReferenceKind::Potion => Some(SELECTION_REFERENCE_POTION_KIND),
        SelectionReferenceKind::Player => Some(SELECTION_REFERENCE_PLAYER_KIND),
        SelectionReferenceKind::Content { entity_kind } => Some(entity_kind),
        SelectionReferenceKind::Candidate | SelectionReferenceKind::Unknown => None,
    }
}

/// Returns a stable token naming the family a typed reference resolves to.
#[must_use]
pub(super) fn reference_family(kind: &SelectionReferenceKind) -> Option<String> {
    reference_kind_token(kind).map(str::to_owned)
}
