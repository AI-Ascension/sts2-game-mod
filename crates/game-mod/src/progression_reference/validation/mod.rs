// SPDX-License-Identifier: MIT

//! Eligibility rules: what the source may declare, and what a manifest must resolve.

mod entry;

pub(crate) use entry::validate_entry;

use std::collections::BTreeMap;

use crate::ContentManifest;

use super::{
    PROGRESSION_MAX_FIELD_ROWS, ProgressionContentReference, ProgressionDomain,
    ProgressionDomainCoverage, ProgressionDomainState, ProgressionEntry, ProgressionReferenceError,
};

/// Validates that every domain is declared exactly once with a count the catalog observes.
pub(super) fn validate_capability(
    domains: &[ProgressionDomainCoverage],
    entries: &BTreeMap<String, ProgressionEntry>,
) -> Result<(), ProgressionReferenceError> {
    if domains.len() != ProgressionDomain::all().len() {
        return Err(ProgressionReferenceError::DomainCoverageIncomplete);
    }
    for domain in ProgressionDomain::all() {
        let row = coverage_for(domains, domain)?;
        if row.unsupported_fields.len() > PROGRESSION_MAX_FIELD_ROWS {
            return Err(ProgressionReferenceError::InvalidInput(
                "unsupported_fields",
            ));
        }
        if !domain.is_game_progression() && matches!(row.state, ProgressionDomainState::Projected) {
            return Err(ProgressionReferenceError::AccountScopedDomainClaimed);
        }
        let observed = entries
            .values()
            .filter(|entry| entry.domain == domain)
            .count();
        if observed != row.entry_count {
            return Err(ProgressionReferenceError::DomainCountMismatch);
        }
    }
    for entry in entries.values() {
        validate_entry_domain(domains, entry)?;
    }
    Ok(())
}

fn validate_entry_domain(
    domains: &[ProgressionDomainCoverage],
    entry: &ProgressionEntry,
) -> Result<(), ProgressionReferenceError> {
    let row = coverage_for(domains, entry.domain)?;
    if !matches!(row.state, ProgressionDomainState::Projected) {
        return Err(ProgressionReferenceError::UnavailableDomain);
    }
    for field in &row.unsupported_fields {
        if entry.field_status(*field) == Some(super::ProgressionFieldStatus::Available) {
            return Err(ProgressionReferenceError::InconsistentField(field.name()));
        }
    }
    Ok(())
}

fn coverage_for(
    domains: &[ProgressionDomainCoverage],
    domain: ProgressionDomain,
) -> Result<&ProgressionDomainCoverage, ProgressionReferenceError> {
    let mut matches = domains.iter().filter(|row| row.domain == domain);
    let row = matches
        .next()
        .ok_or(ProgressionReferenceError::DomainCoverageIncomplete)?;
    if matches.next().is_some() {
        return Err(ProgressionReferenceError::DomainCoverageIncomplete);
    }
    Ok(row)
}

/// Validates that this domain's own manifest family is handled by the content manifest.
pub(super) fn validate_domain_family(
    manifest: &ContentManifest,
    domain: ProgressionDomain,
) -> Result<(), ProgressionReferenceError> {
    let Some(kind) = domain.manifest_kind() else {
        return Err(ProgressionReferenceError::AccountScopedEntryInProgression);
    };
    validate_handled_family(manifest, kind)
}

/// Validates that one referenced definition is present in a handled manifest family.
pub(super) fn validate_content_reference(
    manifest: &ContentManifest,
    reference: &ProgressionContentReference,
) -> Result<(), ProgressionReferenceError> {
    super::validate_identity(&reference.entity_kind, "content_reference_kind")?;
    super::validate_identity(&reference.namespaced_id, "content_reference_id")?;
    validate_handled_family(manifest, &reference.entity_kind)?;
    let present = manifest.definitions.iter().any(|definition| {
        definition.entity_kind == reference.entity_kind
            && definition.namespaced_id == reference.namespaced_id
    });
    if !present {
        return Err(ProgressionReferenceError::UnknownManifestReference {
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
) -> Result<(), ProgressionReferenceError> {
    let handled = manifest
        .families
        .iter()
        .any(|family| family.entity_kind == entity_kind && family.handled);
    if handled {
        Ok(())
    } else {
        Err(ProgressionReferenceError::UnhandledManifestFamily(
            entity_kind.to_owned(),
        ))
    }
}
