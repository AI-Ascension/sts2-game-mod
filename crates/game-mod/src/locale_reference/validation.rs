// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use super::model::validate_identity;
use super::model::{LocaleEntityReference, LocaleEntryInput, LocaleInput, LocaleTextSegment};
use super::normalize::{validate_effect_amount, validate_presentation};
use super::{
    LOCALE_REFERENCE_MAX_ENTRIES, LOCALE_REFERENCE_MAX_FALLBACK_DEPTH,
    LOCALE_REFERENCE_MAX_LOCALES, LOCALE_REFERENCE_MAX_PLACEHOLDERS, LOCALE_REFERENCE_MAX_SEGMENTS,
    LOCALE_REFERENCE_MAX_TEXT_BYTES, LocaleCatalogError,
};

/// Validates the declared supported locales and returns them indexed by tag.
pub(super) fn validate_locales(
    locales: &[LocaleInput],
    default_locale: &str,
) -> Result<BTreeMap<String, LocaleInput>, LocaleCatalogError> {
    if locales.is_empty() {
        return Err(LocaleCatalogError::MissingSupportedLocales);
    }
    if locales.len() > LOCALE_REFERENCE_MAX_LOCALES {
        return Err(LocaleCatalogError::InvalidInput("locales"));
    }
    validate_identity(default_locale, "default_locale")?;
    let mut index = BTreeMap::new();
    for input in locales {
        validate_identity(&input.locale, "locale")?;
        if index.insert(input.locale.clone(), input.clone()).is_some() {
            return Err(LocaleCatalogError::DuplicateLocale(input.locale.clone()));
        }
    }
    if !index.contains_key(default_locale) {
        return Err(LocaleCatalogError::MissingDefaultLocale);
    }
    for input in locales {
        validate_fallback_chain(&index, input, default_locale)?;
    }
    Ok(index)
}

fn validate_fallback_chain(
    index: &BTreeMap<String, LocaleInput>,
    input: &LocaleInput,
    default_locale: &str,
) -> Result<(), LocaleCatalogError> {
    let chain = &input.fallback;
    if chain.is_empty() {
        return Err(LocaleCatalogError::InvalidFallbackChain("empty"));
    }
    if chain.len() > LOCALE_REFERENCE_MAX_FALLBACK_DEPTH {
        return Err(LocaleCatalogError::InvalidFallbackChain("too_deep"));
    }
    if chain.first().map(String::as_str) != Some(input.locale.as_str()) {
        return Err(LocaleCatalogError::InvalidFallbackChain("unordered"));
    }
    // Repeats are diagnosed before the terminal check: a chain that revisits a
    // locale is not a simple path to the default, so "ends at the default" is
    // not a meaningful question to ask of it.
    let mut seen = BTreeSet::new();
    for tag in chain {
        validate_identity(tag, "fallback_locale")?;
        if !index.contains_key(tag) {
            return Err(LocaleCatalogError::UnsupportedLocale(tag.clone()));
        }
        if !seen.insert(tag.as_str()) {
            return Err(LocaleCatalogError::InvalidFallbackChain("cyclic"));
        }
    }
    if chain.last().map(String::as_str) != Some(default_locale) {
        return Err(LocaleCatalogError::InvalidFallbackChain("unordered"));
    }
    Ok(())
}

/// Validates one source-owned entry before it enters an immutable catalog.
pub(super) fn validate_entry(
    input: &LocaleEntryInput,
    supported: &BTreeMap<String, LocaleInput>,
) -> Result<(), LocaleCatalogError> {
    validate_identity(&input.entity_kind, "entity_kind")?;
    validate_identity(&input.namespaced_id, "namespaced_id")?;
    validate_identity(&input.revision, "text_revision")?;
    if !supported.contains_key(&input.locale) {
        return Err(LocaleCatalogError::UnsupportedLocale(input.locale.clone()));
    }
    if input.segments.len() > LOCALE_REFERENCE_MAX_SEGMENTS {
        return Err(LocaleCatalogError::InvalidInput("segments"));
    }
    // An entry with no segments would render as an empty answer, which this boundary never
    // produces: an absent translation is reported as unavailable, never as nothing.
    if input.segments.is_empty() {
        return Err(LocaleCatalogError::InvalidInput("empty_segments"));
    }
    if input.requires.len() > LOCALE_REFERENCE_MAX_PLACEHOLDERS {
        return Err(LocaleCatalogError::InvalidInput("placeholders"));
    }
    let mut declared = BTreeSet::new();
    for name in &input.requires {
        validate_identity(name, "placeholder")?;
        if !declared.insert(name.as_str()) {
            return Err(LocaleCatalogError::InvalidInput("duplicate_placeholder"));
        }
    }
    let mut used = BTreeSet::new();
    let mut retained = 0usize;
    for segment in &input.segments {
        retained += segment.retained_len();
        match segment {
            LocaleTextSegment::Text(value) => {
                if value.is_empty() {
                    return Err(LocaleCatalogError::InvalidInput("empty_text"));
                }
                validate_presentation(value, "text")?;
            }
            LocaleTextSegment::Placeholder(name) => {
                validate_identity(name, "placeholder")?;
                if !declared.contains(name.as_str()) {
                    return Err(LocaleCatalogError::InvalidInput("undeclared_placeholder"));
                }
                used.insert(name.as_str());
            }
            LocaleTextSegment::Reference(reference) => validate_reference_shape(reference)?,
            LocaleTextSegment::Effect { kind, amount } => {
                validate_effect_amount(kind, amount)?;
            }
        }
    }
    if declared != used {
        return Err(LocaleCatalogError::InvalidInput("unused_placeholder"));
    }
    if retained > LOCALE_REFERENCE_MAX_TEXT_BYTES {
        return Err(LocaleCatalogError::TextTooLarge {
            limit: LOCALE_REFERENCE_MAX_TEXT_BYTES,
            actual: retained,
        });
    }
    Ok(())
}

/// Validates the shape of one language-independent reference.
pub(super) fn validate_reference_shape(
    reference: &LocaleEntityReference,
) -> Result<(), LocaleCatalogError> {
    validate_identity(&reference.entity_kind, "reference_kind")?;
    validate_identity(&reference.namespaced_id, "reference_id")
}

/// Rejects an entry count above the catalog bound.
pub(super) fn validate_entry_count(count: usize) -> Result<(), LocaleCatalogError> {
    if count > LOCALE_REFERENCE_MAX_ENTRIES {
        return Err(LocaleCatalogError::InvalidInput("entries"));
    }
    Ok(())
}
