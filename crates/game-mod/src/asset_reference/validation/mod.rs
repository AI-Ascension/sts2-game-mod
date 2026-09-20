// SPDX-License-Identifier: MIT

//! Eligibility rules: what the source may declare, and what a manifest must resolve.

mod entry;

pub(crate) use entry::validate_asset;

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::{
    ASSET_MAX_FIELD_ROWS, AssetClassCoverage, AssetClassState, AssetDefinitionReference,
    AssetEntry, AssetFieldStatus, AssetMediaClass, AssetReferenceError,
};

/// Validates that every media class is declared exactly once with a count the catalog observes.
pub(super) fn validate_coverage(
    classes: &[AssetClassCoverage],
    assets: &BTreeMap<String, AssetEntry>,
) -> Result<(), AssetReferenceError> {
    if classes.len() != AssetMediaClass::all().len() {
        return Err(AssetReferenceError::ClassCoverageIncomplete);
    }
    for class in AssetMediaClass::all() {
        let row = coverage_for(classes, class)?;
        if row.unsupported_fields.len() > ASSET_MAX_FIELD_ROWS {
            return Err(AssetReferenceError::InvalidInput("unsupported_fields"));
        }
        let observed = assets
            .values()
            .filter(|entry| entry.media_kind.media_class() == class)
            .count();
        if observed != row.asset_count {
            return Err(AssetReferenceError::ClassCountMismatch);
        }
    }
    for entry in assets.values() {
        validate_entry_class(classes, entry)?;
    }
    Ok(())
}

fn validate_entry_class(
    classes: &[AssetClassCoverage],
    entry: &AssetEntry,
) -> Result<(), AssetReferenceError> {
    let row = coverage_for(classes, entry.media_kind.media_class())?;
    if !matches!(row.state, AssetClassState::Projected) {
        return Err(AssetReferenceError::UnavailableClass);
    }
    for field in &row.unsupported_fields {
        if entry.field_status(*field) == Some(AssetFieldStatus::Available) {
            return Err(AssetReferenceError::InconsistentField(field.name()));
        }
    }
    Ok(())
}

fn coverage_for(
    classes: &[AssetClassCoverage],
    class: AssetMediaClass,
) -> Result<&AssetClassCoverage, AssetReferenceError> {
    let mut matches = classes.iter().filter(|row| row.class == class);
    let row = matches
        .next()
        .ok_or(AssetReferenceError::ClassCoverageIncomplete)?;
    if matches.next().is_some() {
        return Err(AssetReferenceError::ClassCoverageIncomplete);
    }
    Ok(row)
}

/// Validates that one referenced definition is present in a handled manifest family.
pub(super) fn validate_definition_reference(
    manifest: &ContentManifest,
    reference: &AssetDefinitionReference,
) -> Result<(), AssetReferenceError> {
    validate_handled_family(manifest, &reference.entity_kind)?;
    let present = manifest.definitions.iter().any(|definition| {
        definition.entity_kind == reference.entity_kind
            && definition.namespaced_id == reference.namespaced_id
    });
    if !present {
        return Err(AssetReferenceError::UnknownManifestReference {
            entity_kind: reference.entity_kind.clone(),
            namespaced_id: reference.namespaced_id.clone(),
        });
    }
    Ok(())
}

/// Refuses a family the manifest does not handle, so a reference cannot resolve silently.
pub(super) fn validate_handled_family(
    manifest: &ContentManifest,
    entity_kind: &str,
) -> Result<(), AssetReferenceError> {
    let handled = manifest
        .families
        .iter()
        .any(|family| family.entity_kind == entity_kind && family.handled);
    if handled {
        Ok(())
    } else {
        Err(AssetReferenceError::UnhandledManifestFamily(
            entity_kind.to_owned(),
        ))
    }
}
