// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::{
    CONTENT_MANIFEST_MAX_SEMANTIC_BYTES, CONTENT_MANIFEST_MAX_TEXT_BYTES, ContentCatalogSnapshot,
    ContentManifestError, validate_identity,
};

pub(super) fn validate_snapshot(
    snapshot: &ContentCatalogSnapshot,
) -> Result<(), ContentManifestError> {
    validate_identity(&snapshot.game_build, "game_build")?;
    validate_identity(&snapshot.locale, "locale")?;

    let mut package_ids = BTreeSet::new();
    let mut package_orders = BTreeSet::new();
    for package in &snapshot.packages {
        validate_identity(&package.package_id, "package_id")?;
        if let Some(version) = &package.package_version {
            validate_identity(version, "package_version")
                .map_err(|_| ContentManifestError::InvalidPackageVersion)?;
        }
        if !package_ids.insert(package.package_id.as_str()) {
            return Err(ContentManifestError::DuplicatePackage);
        }
        if !package_orders.insert(package.order) {
            return Err(ContentManifestError::InvalidPackageOrder);
        }
    }
    let package_versions = snapshot
        .packages
        .iter()
        .map(|package| {
            (
                package.package_id.as_str(),
                package.package_version.as_deref(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let mut available_kinds = BTreeSet::new();
    for entity_kind in &snapshot.available_entity_kinds {
        validate_identity(entity_kind, "entity_kind")?;
        if !available_kinds.insert(entity_kind.as_str()) {
            return Err(ContentManifestError::DuplicateEntityKind);
        }
    }

    let mut definition_ids = BTreeSet::new();
    for definition in &snapshot.definitions {
        validate_identity(&definition.entity_kind, "entity_kind")?;
        validate_identity(&definition.namespaced_id, "namespaced_id")?;
        if !available_kinds.contains(definition.entity_kind.as_str()) {
            return Err(ContentManifestError::UnknownEntityKind);
        }
        if !definition_ids.insert((
            definition.entity_kind.as_str(),
            definition.namespaced_id.as_str(),
        )) {
            return Err(ContentManifestError::DuplicateDefinition);
        }
        if definition.semantic_inputs.is_empty()
            || definition.semantic_inputs.len() > CONTENT_MANIFEST_MAX_SEMANTIC_BYTES
            || definition.semantic_inputs.chars().any(char::is_control)
        {
            return Err(ContentManifestError::InvalidSemanticInput);
        }
        if let Some(text) = &definition.localized_text
            && (text.len() > CONTENT_MANIFEST_MAX_TEXT_BYTES || text.chars().any(char::is_control))
        {
            return Err(ContentManifestError::InvalidLocalizedText);
        }
        if let Some(package_version) = &definition.origin.package_version {
            validate_identity(package_version, "origin_package_version")
                .map_err(|_| ContentManifestError::InvalidPackageVersion)?;
        }
        if definition.origin.package_id.is_none() && definition.origin.package_version.is_some() {
            return Err(ContentManifestError::UnknownOriginPackage);
        }
        if let Some(package_id) = &definition.origin.package_id {
            let Some(active_version) = package_versions.get(package_id.as_str()) else {
                return Err(ContentManifestError::UnknownOriginPackage);
            };
            if let (Some(active_version), Some(origin_version)) = (
                *active_version,
                definition.origin.package_version.as_deref(),
            ) && active_version != origin_version
            {
                return Err(ContentManifestError::OriginPackageVersionMismatch);
            }
        }
        let mut override_ids = BTreeSet::new();
        for reference in &definition.override_chain {
            validate_identity(reference, "override_reference")?;
            if !override_ids.insert(reference.as_str()) {
                return Err(ContentManifestError::DuplicateOverrideReference);
            }
        }
    }

    Ok(())
}
