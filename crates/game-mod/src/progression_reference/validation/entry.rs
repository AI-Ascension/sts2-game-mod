// SPDX-License-Identifier: MIT

//! Per-entry eligibility: state, units, collections, field rows and resolvable references.

use std::collections::BTreeSet;

use crate::ContentManifest;

use super::super::definition::{entry_bytes, validate_field_rows};
use super::super::validate_entry_bounds;
use super::super::{
    PROGRESSION_MAX_ENTRY_BYTES, ProgressionCatalogBinding, ProgressionDomain,
    ProgressionEntryInput, ProgressionFieldValue, ProgressionReadState, ProgressionReferenceError,
    ProgressionRequirement, ProgressionSensitivity, validate_identity, validate_text,
};
use super::{validate_content_reference, validate_domain_family};

pub(crate) fn validate_entry(
    input: &ProgressionEntryInput,
    manifest: &ContentManifest,
    binding: &ProgressionCatalogBinding,
) -> Result<(), ProgressionReferenceError> {
    if binding.profile.user_data_id.as_str().is_empty() {
        return Err(ProgressionReferenceError::ProfileNotPermitted);
    }
    validate_identity(&input.entry_id, "entry_id")?;
    validate_text(&input.title, "title")?;
    if let Some(description) = &input.description {
        validate_text(description, "description")?;
    }
    if input.account_id.is_some() {
        return Err(ProgressionReferenceError::AccountIdentifierInProgression);
    }
    validate_classification(input)?;
    validate_domain_family(manifest, input.domain)?;
    validate_state(input)?;
    validate_units(input)?;
    validate_collections(input)?;
    validate_entry_bounds(
        stated_len(&input.requirements),
        stated_len(&input.content_references),
        stated_len(&input.related),
    )?;
    validate_field_rows(input)?;
    validate_requirements(&input.requirements)?;
    validate_references(input, manifest)?;
    let actual = entry_bytes(input);
    if actual > PROGRESSION_MAX_ENTRY_BYTES {
        return Err(ProgressionReferenceError::EntryTooLarge {
            limit: PROGRESSION_MAX_ENTRY_BYTES,
            actual,
        });
    }
    Ok(())
}

fn validate_classification(input: &ProgressionEntryInput) -> Result<(), ProgressionReferenceError> {
    if !input.domain.is_game_progression()
        || !matches!(input.sensitivity, ProgressionSensitivity::GameProgression)
    {
        return Err(ProgressionReferenceError::AccountScopedEntryInProgression);
    }
    Ok(())
}

fn validate_state(input: &ProgressionEntryInput) -> Result<(), ProgressionReferenceError> {
    if input.read_state == ProgressionReadState::Undiscovered
        && input.domain != ProgressionDomain::Compendium
    {
        return Err(ProgressionReferenceError::UndiscoveredOutsideCompendium);
    }
    if input.read_state == ProgressionReadState::Locked && stated_len(&input.requirements) == 0 {
        return Err(ProgressionReferenceError::LockedWithoutRequirement);
    }
    if input.read_state == ProgressionReadState::Unlocked
        && requirement_is_locked(&input.requirements)
    {
        return Err(ProgressionReferenceError::UnlockedWithUnsatisfiedRequirement);
    }
    let asserts_nothing = matches!(
        input.read_state,
        ProgressionReadState::NotTracked
            | ProgressionReadState::Unavailable
            | ProgressionReadState::Unclassified
    );
    if asserts_nothing && input.progress.is_available() {
        return Err(ProgressionReferenceError::StateCarriesValue("progress"));
    }
    if asserts_nothing && input.best.is_available() {
        return Err(ProgressionReferenceError::StateCarriesValue("best_value"));
    }
    if input.best.is_available() && input.domain != ProgressionDomain::BestRecords {
        return Err(ProgressionReferenceError::BestRecordOutsideBestRecords);
    }
    Ok(())
}

fn requirement_is_locked(
    requirements: &ProgressionFieldValue<Vec<ProgressionRequirement>>,
) -> bool {
    requirements.value().is_some_and(|requirements| {
        requirements
            .iter()
            .any(|requirement| requirement.state == ProgressionReadState::Locked)
    })
}

fn validate_units(input: &ProgressionEntryInput) -> Result<(), ProgressionReferenceError> {
    for field in [&input.progress, &input.best] {
        if let Some(value) = field.value()
            && !value.is_bounded_percentage()
        {
            return Err(ProgressionReferenceError::UnboundedPercentage);
        }
    }
    if let Some(requirements) = input.requirements.value() {
        for requirement in requirements {
            if let Some(value) = requirement.progress.value()
                && !value.is_bounded_percentage()
            {
                return Err(ProgressionReferenceError::UnboundedPercentage);
            }
        }
    }
    Ok(())
}

/// Refuses a collection stated available that is empty, because it then states nothing.
fn validate_collections(input: &ProgressionEntryInput) -> Result<(), ProgressionReferenceError> {
    if input.read_state != ProgressionReadState::Locked && is_empty_present(&input.requirements) {
        return Err(ProgressionReferenceError::EmptyPresentCollection(
            "requirements",
        ));
    }
    if is_empty_present(&input.content_references) {
        return Err(ProgressionReferenceError::EmptyPresentCollection(
            "content_references",
        ));
    }
    if is_empty_present(&input.related) {
        return Err(ProgressionReferenceError::EmptyPresentCollection("related"));
    }
    Ok(())
}

fn is_empty_present<T>(field: &ProgressionFieldValue<Vec<T>>) -> bool {
    field.value().is_some_and(Vec::is_empty)
}

fn stated_len<T>(field: &ProgressionFieldValue<Vec<T>>) -> usize {
    field.value().map_or(0, Vec::len)
}

fn validate_requirements(
    requirements: &ProgressionFieldValue<Vec<ProgressionRequirement>>,
) -> Result<(), ProgressionReferenceError> {
    let Some(requirements) = requirements.value() else {
        return Ok(());
    };
    let mut seen = BTreeSet::new();
    for requirement in requirements {
        validate_identity(&requirement.requirement_id, "requirement_id")?;
        if !seen.insert(requirement.requirement_id.clone()) {
            return Err(ProgressionReferenceError::DuplicateRequirement(
                requirement.requirement_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_references(
    input: &ProgressionEntryInput,
    manifest: &ContentManifest,
) -> Result<(), ProgressionReferenceError> {
    let Some(references) = input.content_references.value() else {
        return Ok(());
    };
    for reference in references {
        validate_content_reference(manifest, reference)?;
    }
    if let Some(requirements) = input.requirements.value() {
        for requirement in requirements {
            if let Some(content) = &requirement.content {
                validate_content_reference(manifest, content)?;
            }
        }
    }
    Ok(())
}
