// SPDX-License-Identifier: MIT

//! Shared field checks: stated availability, bounded text, and collection bounds.

use super::super::{
    COOP_MAX_TEXT_BYTES, CoopError, CoopFieldValue, identity::validate_opaque_identity,
};

/// Refuses a field whose status and value disagree.
pub(super) fn require_consistent<T>(
    field: &CoopFieldValue<T>,
    name: &'static str,
) -> Result<(), CoopError> {
    if field.is_consistent() {
        Ok(())
    } else {
        Err(CoopError::InconsistentField(name))
    }
}

/// Validates one bounded text value.
pub(super) fn validate_text(value: &str, field: &'static str) -> Result<(), CoopError> {
    if value.is_empty() || value.len() > COOP_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(CoopError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a bounded text field together with its stated availability.
pub(super) fn validate_text_field(
    field: &CoopFieldValue<String>,
    name: &'static str,
) -> Result<(), CoopError> {
    require_consistent(field, name)?;
    if let Some(value) = field.value() {
        validate_text(value, name)?;
    }
    Ok(())
}

/// Validates an opaque identifier field together with its stated availability.
pub(super) fn validate_identity_field(
    field: &CoopFieldValue<String>,
    name: &'static str,
) -> Result<(), CoopError> {
    require_consistent(field, name)?;
    if let Some(value) = field.value() {
        validate_opaque_identity(value, name)?;
    }
    Ok(())
}

/// Refuses a collection the source states as present while it carries no entries.
pub(super) fn require_non_empty_present<T>(
    field: &CoopFieldValue<Vec<T>>,
    name: &'static str,
) -> Result<(), CoopError> {
    require_consistent(field, name)?;
    if field.value().is_some_and(Vec::is_empty) {
        return Err(CoopError::EmptyPresentCollection(name));
    }
    Ok(())
}

/// Refuses a collection that exceeds its local bound.
pub(super) fn require_bounded<T>(
    entries: &[T],
    limit: usize,
    name: &'static str,
) -> Result<(), CoopError> {
    if entries.len() > limit {
        return Err(CoopError::InvalidInput(name));
    }
    Ok(())
}

/// Refuses an entry that states a count below one.
pub(super) fn require_entry_count(count: u32, id: &str) -> Result<(), CoopError> {
    if count == 0 {
        return Err(CoopError::ZeroCountEntry(id.to_owned()));
    }
    Ok(())
}

/// Refuses a list that repeats one entry identity.
pub(super) fn require_unique(
    seen: &mut std::collections::BTreeSet<String>,
    id: &str,
) -> Result<(), CoopError> {
    if seen.insert(id.to_owned()) {
        Ok(())
    } else {
        Err(CoopError::DuplicateEntry(id.to_owned()))
    }
}
