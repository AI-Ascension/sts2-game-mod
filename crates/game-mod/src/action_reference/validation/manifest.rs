// SPDX-License-Identifier: MIT

use crate::ContentManifest;

use super::super::{
    definition::ActionDefinitionInput,
    error::ActionError,
    field::ActionField,
    model::{
        ACTION_REFERENCE_CARD_KIND, ACTION_REFERENCE_EFFECT_KIND, ACTION_REFERENCE_ENEMY_KIND,
        ACTION_REFERENCE_ENTITY_KIND, ACTION_REFERENCE_PLAYER_KIND, ACTION_REFERENCE_POTION_KIND,
        ACTION_REFERENCE_RELIC_KIND, ACTION_REFERENCE_RESOURCE_KIND,
        ACTION_REFERENCE_SELECTION_KIND, ACTION_REFERENCE_STATUS_KIND, ActionReferenceKind,
        ActionSemanticReference,
    },
};

/// Requires every reference of one definition to resolve inside the content manifest.
pub(in crate::action_reference) fn validate_manifest_references(
    input: &ActionDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), ActionError> {
    ensure_all(manifest, &input.references)?;
    for target in &input.targets {
        ensure_manifest_reference(manifest, &target.definition)?;
        ensure_all(manifest, &target.references)?;
        ensure_all(manifest, &target.eligibility.references)?;
    }
    for cost in &input.costs {
        ensure_all(manifest, &cost.references)?;
        if let ActionField::Available(resource) = &cost.resource {
            ensure_manifest_reference(manifest, resource)?;
        }
    }
    for restriction in &input.restrictions {
        ensure_all(manifest, &restriction.references)?;
    }
    for preview in &input.previews {
        ensure_all(manifest, &preview.references)?;
        ensure_preview_references(manifest, preview)?;
    }
    Ok(())
}

fn ensure_preview_references(
    manifest: &ContentManifest,
    preview: &super::super::preview::ActionPreviewInput,
) -> Result<(), ActionError> {
    for change in &preview.changes {
        ensure_all(manifest, &change.references)?;
        if let ActionField::Available(target) = &change.target {
            ensure_manifest_reference(manifest, target)?;
        }
    }
    for status in &preview.statuses {
        ensure_manifest_reference(manifest, &status.status)?;
        ensure_all(manifest, &status.references)?;
        if let ActionField::Available(target) = &status.target {
            ensure_manifest_reference(manifest, target)?;
        }
    }
    for movement in &preview.movements {
        ensure_manifest_reference(manifest, &movement.card)?;
        ensure_all(manifest, &movement.references)?;
    }
    for selection in &preview.selections {
        ensure_manifest_reference(manifest, &selection.selection)?;
        ensure_all(manifest, &selection.references)?;
    }
    for assumption in &preview.assumptions {
        ensure_all(manifest, &assumption.references)?;
    }
    for omission in &preview.omissions {
        ensure_all(manifest, &omission.references)?;
    }
    Ok(())
}

fn ensure_all(
    manifest: &ContentManifest,
    references: &[ActionSemanticReference],
) -> Result<(), ActionError> {
    for reference in references {
        ensure_manifest_reference(manifest, reference)?;
    }
    Ok(())
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    reference: &ActionSemanticReference,
) -> Result<(), ActionError> {
    let Some(entity_kind) = reference_kind_token(&reference.kind) else {
        return Ok(());
    };
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == reference.id
    }) {
        Ok(())
    } else {
        Err(ActionError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: reference.id.clone(),
        })
    }
}

/// Returns the manifest entity family a typed reference resolves to, when it names one.
///
/// A destination is local to the action's own presented targets and therefore names no manifest
/// family; an unclassified reference is deliberately `None` rather than assumed.
#[must_use]
pub(super) fn reference_kind_token(kind: &ActionReferenceKind) -> Option<&str> {
    match kind {
        ActionReferenceKind::Action => Some(ACTION_REFERENCE_ENTITY_KIND),
        ActionReferenceKind::Effect => Some(ACTION_REFERENCE_EFFECT_KIND),
        ActionReferenceKind::Selection => Some(ACTION_REFERENCE_SELECTION_KIND),
        ActionReferenceKind::Card => Some(ACTION_REFERENCE_CARD_KIND),
        ActionReferenceKind::Enemy => Some(ACTION_REFERENCE_ENEMY_KIND),
        ActionReferenceKind::Player => Some(ACTION_REFERENCE_PLAYER_KIND),
        ActionReferenceKind::Status => Some(ACTION_REFERENCE_STATUS_KIND),
        ActionReferenceKind::Relic => Some(ACTION_REFERENCE_RELIC_KIND),
        ActionReferenceKind::Potion => Some(ACTION_REFERENCE_POTION_KIND),
        ActionReferenceKind::Resource => Some(ACTION_REFERENCE_RESOURCE_KIND),
        ActionReferenceKind::Content { entity_kind } => Some(entity_kind),
        ActionReferenceKind::Destination | ActionReferenceKind::Unknown => None,
    }
}

/// Returns a stable token naming the family a typed reference resolves to.
#[must_use]
pub(super) fn reference_family(kind: &ActionReferenceKind) -> Option<String> {
    reference_kind_token(kind).map(str::to_owned)
}
