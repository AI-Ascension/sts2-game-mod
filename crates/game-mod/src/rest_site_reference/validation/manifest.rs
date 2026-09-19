// SPDX-License-Identifier: MIT

use crate::ContentManifest;

use super::super::{
    definition::RestSiteDefinitionInput,
    error::RestSiteError,
    field::RestField,
    model::{
        REST_REFERENCE_CARD_KIND, REST_REFERENCE_EFFECT_KIND, REST_REFERENCE_ENTITY_KIND,
        REST_REFERENCE_PLAYER_KIND, REST_REFERENCE_POTION_KIND, REST_REFERENCE_RELIC_KIND,
        RestReferenceKind, RestSemanticReference,
    },
};

/// Requires every reference of one definition to resolve inside the content manifest.
pub(in crate::rest_site_reference) fn validate_manifest_references(
    input: &RestSiteDefinitionInput,
    manifest: &ContentManifest,
) -> Result<(), RestSiteError> {
    for reference in &input.references {
        ensure_manifest_reference(manifest, reference)?;
    }
    for option in &input.options {
        ensure_manifest_reference(manifest, &option.definition)?;
        ensure_all(manifest, &option.references)?;
        for requirement in &option.requirements {
            ensure_all(manifest, &requirement.references)?;
        }
        for cost in &option.costs {
            ensure_all(manifest, &cost.references)?;
        }
        for limit in &option.limits {
            ensure_all(manifest, &limit.references)?;
        }
        for effect in &option.effects {
            ensure_all(manifest, &effect.references)?;
            if let RestField::Available(target) = &effect.target {
                ensure_manifest_reference(manifest, target)?;
            }
        }
        ensure_all(manifest, &option.selection.references)?;
        for candidate in &option.selection.candidates {
            ensure_manifest_reference(manifest, &candidate.reference)?;
            ensure_all(manifest, &candidate.references)?;
        }
        if let Some(comparison) = &option.comparison {
            for change in &comparison.changes {
                ensure_all(manifest, &change.references)?;
                if let RestField::Available(target) = &change.target {
                    ensure_manifest_reference(manifest, target)?;
                }
            }
            if let RestField::Available(upgrade) = &comparison.upgrade {
                ensure_manifest_reference(manifest, &upgrade.candidate)?;
                ensure_all(manifest, &upgrade.references)?;
            }
        }
    }
    Ok(())
}

fn ensure_all(
    manifest: &ContentManifest,
    references: &[RestSemanticReference],
) -> Result<(), RestSiteError> {
    for reference in references {
        ensure_manifest_reference(manifest, reference)?;
    }
    Ok(())
}

fn ensure_manifest_reference(
    manifest: &ContentManifest,
    reference: &RestSemanticReference,
) -> Result<(), RestSiteError> {
    let Some(entity_kind) = reference_kind_token(&reference.kind) else {
        return Ok(());
    };
    if manifest.definitions.iter().any(|definition| {
        definition.entity_kind == entity_kind && definition.namespaced_id == reference.id
    }) {
        Ok(())
    } else {
        Err(RestSiteError::UnknownManifestReference {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: reference.id.clone(),
        })
    }
}

/// Returns the manifest entity family a typed reference resolves to, when it names one.
///
/// Option references are local to their owning rest site and therefore name no manifest family; an
/// unclassified reference is deliberately `None` rather than assumed.
#[must_use]
pub(super) fn reference_kind_token(kind: &RestReferenceKind) -> Option<&str> {
    match kind {
        RestReferenceKind::Site => Some(REST_REFERENCE_ENTITY_KIND),
        RestReferenceKind::Effect => Some(REST_REFERENCE_EFFECT_KIND),
        RestReferenceKind::Card => Some(REST_REFERENCE_CARD_KIND),
        RestReferenceKind::Relic => Some(REST_REFERENCE_RELIC_KIND),
        RestReferenceKind::Potion => Some(REST_REFERENCE_POTION_KIND),
        RestReferenceKind::Player => Some(REST_REFERENCE_PLAYER_KIND),
        RestReferenceKind::Content { entity_kind } => Some(entity_kind),
        RestReferenceKind::Option | RestReferenceKind::Unknown => None,
    }
}

/// Returns a stable token naming the family a typed reference resolves to.
#[must_use]
pub(super) fn reference_family(kind: &RestReferenceKind) -> Option<String> {
    reference_kind_token(kind).map(str::to_owned)
}
