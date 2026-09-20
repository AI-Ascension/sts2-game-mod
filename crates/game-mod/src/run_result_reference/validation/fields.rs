// SPDX-License-Identifier: MIT

//! Shared field checks: stated availability, bounded text, and collection bounds.

use super::super::{
    RUN_RESULT_MAX_TEXT_BYTES, RunResultError, RunResultFieldValue,
    identity::validate_opaque_identity,
};

/// Refuses a field whose status and value disagree.
pub(super) fn require_consistent<T>(
    field: &RunResultFieldValue<T>,
    name: &'static str,
) -> Result<(), RunResultError> {
    if field.is_consistent() {
        Ok(())
    } else {
        Err(RunResultError::InconsistentField(name))
    }
}

/// Validates one bounded text value.
pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), RunResultError> {
    if value.is_empty()
        || value.len() > RUN_RESULT_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(RunResultError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a bounded text field together with its stated availability.
pub(super) fn validate_text_field(
    field: &RunResultFieldValue<String>,
    name: &'static str,
) -> Result<(), RunResultError> {
    require_consistent(field, name)?;
    if let Some(value) = field.value() {
        validate_text(value, name)?;
    }
    Ok(())
}

/// Validates an opaque identifier field together with its stated availability.
pub(super) fn validate_identity_field(
    field: &RunResultFieldValue<String>,
    name: &'static str,
) -> Result<(), RunResultError> {
    require_consistent(field, name)?;
    if let Some(value) = field.value() {
        validate_opaque_identity(value, name)?;
    }
    Ok(())
}

/// Refuses a collection the source states as present while it carries no entries.
pub(super) fn require_non_empty_present<T>(
    field: &RunResultFieldValue<Vec<T>>,
    name: &'static str,
) -> Result<(), RunResultError> {
    require_consistent(field, name)?;
    if field.value().is_some_and(Vec::is_empty) {
        return Err(RunResultError::EmptyPresentCollection(name));
    }
    Ok(())
}

/// Refuses a collection that exceeds its local bound.
pub(super) fn require_bounded<T>(
    entries: &[T],
    limit: usize,
    name: &'static str,
) -> Result<(), RunResultError> {
    if entries.len() > limit {
        return Err(RunResultError::InvalidInput(name));
    }
    Ok(())
}

/// Refuses an ending entry that states a count below one.
pub(super) fn require_entry_count(count: u32, id: &str) -> Result<(), RunResultError> {
    if count == 0 {
        return Err(RunResultError::ZeroCountEntry(id.to_owned()));
    }
    Ok(())
}
