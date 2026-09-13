// SPDX-License-Identifier: MIT

use super::{
    CONTENT_INDEX_MAX_DEFINITION_BYTES, CONTENT_INDEX_MAX_TERM_REFERENCES, ContentDefinition,
    ContentDefinitionSummary, ContentIndexError, ContentQueryFilters,
};

pub(super) fn summary(definition: &ContentDefinition) -> ContentDefinitionSummary {
    ContentDefinitionSummary {
        reference: definition.reference.clone(),
        origin: definition.origin.clone(),
        semantic_revision: definition.semantic_revision.clone(),
        localized_text_revision: definition.localized_text_revision.clone(),
        display_name: definition.display_name.clone(),
        character_or_pool: definition.character_or_pool.clone(),
        rarity: definition.rarity.clone(),
        unlock_state: definition.unlock_state,
        detail_capabilities: definition.detail_capabilities.clone(),
        term_references: definition.term_references.clone(),
    }
}

pub(super) fn identity(definition: &ContentDefinition) -> (&str, &str) {
    (
        definition.reference.entity_kind.as_str(),
        definition.reference.namespaced_id.as_str(),
    )
}

pub(super) fn matches_filters(
    definition: &ContentDefinition,
    filters: &ContentQueryFilters,
) -> bool {
    filters
        .entity_kind
        .as_deref()
        .is_none_or(|kind| definition.reference.entity_kind == kind)
        && filters
            .origin_package
            .as_deref()
            .is_none_or(|package| definition.origin.package_id.as_deref() == Some(package))
        && filters
            .character_or_pool
            .as_deref()
            .is_none_or(|value| definition.character_or_pool.as_deref() == Some(value))
        && filters
            .rarity
            .as_ref()
            .is_none_or(|rarity| definition.rarity.as_ref() == Some(rarity))
        && filters
            .unlock_state
            .is_none_or(|state| definition.unlock_state == state)
}

pub(super) fn search_rank(definition: &ContentDefinition, literal: &str) -> Option<u8> {
    if literal.is_empty() {
        return Some(0);
    }
    let query = fold(literal);
    let capabilities = &definition.detail_capabilities;
    let mut rank = None;
    if capabilities.display_name
        && let Some(name) = &definition.display_name
    {
        rank = merge_rank(rank, match_text(name, &query, 0));
    }
    if capabilities.aliases {
        for alias in &definition.aliases {
            rank = merge_rank(rank, match_text(alias, &query, 3));
        }
    }
    if capabilities.rendered_description
        && let Some(description) = &definition.rendered_description
    {
        rank = merge_rank(rank, match_text(description, &query, 6));
    }
    rank
}

fn merge_rank(current: Option<u8>, candidate: Option<u8>) -> Option<u8> {
    match (current, candidate) {
        (Some(current), Some(candidate)) => Some(current.min(candidate)),
        (None, Some(candidate)) => Some(candidate),
        (current, None) => current,
    }
}

fn match_text(value: &str, query: &str, base: u8) -> Option<u8> {
    let folded = fold(value);
    if folded == query {
        return Some(base);
    }
    if folded.starts_with(query) {
        return Some(base.saturating_add(1));
    }
    folded.contains(query).then_some(base.saturating_add(2))
}

fn fold(value: &str) -> String {
    value.chars().flat_map(char::to_lowercase).collect()
}

pub(super) fn validate_optional_identity(
    value: Option<&str>,
    field: &'static str,
) -> Result<(), ContentIndexError> {
    if let Some(value) = value {
        super::model::validate_identity(value, field).map_err(|error| match error {
            super::model::ContentIndexInputError::InvalidIdentity(field) => {
                ContentIndexError::InvalidIdentity(field)
            }
        })?;
    }
    Ok(())
}

pub(super) fn validate_term_references(
    term_references: &[String],
) -> Result<(), ContentIndexError> {
    if term_references.len() > CONTENT_INDEX_MAX_TERM_REFERENCES {
        return Err(ContentIndexError::CollectionTooLarge {
            field: "term_references",
            limit: CONTENT_INDEX_MAX_TERM_REFERENCES,
            actual: term_references.len(),
        });
    }
    let mut unique = std::collections::BTreeSet::new();
    for term_reference in term_references {
        super::model::validate_identity(term_reference, "term_reference").map_err(|error| {
            match error {
                super::model::ContentIndexInputError::InvalidIdentity(field) => {
                    ContentIndexError::InvalidIdentity(field)
                }
            }
        })?;
        if !unique.insert(term_reference) {
            return Err(ContentIndexError::DuplicateTermReference);
        }
    }
    Ok(())
}

pub(super) fn definition_bytes(definition: &ContentDefinition) -> Result<usize, ContentIndexError> {
    let mut actual = 0usize;
    let mut add = |value: &str| {
        let next = actual
            .checked_add(value.len())
            .ok_or(ContentIndexError::DetailTooLarge {
                limit: CONTENT_INDEX_MAX_DEFINITION_BYTES,
                actual: usize::MAX,
            })?;
        if next > CONTENT_INDEX_MAX_DEFINITION_BYTES {
            return Err(ContentIndexError::DetailTooLarge {
                limit: CONTENT_INDEX_MAX_DEFINITION_BYTES,
                actual: next,
            });
        }
        actual = next;
        Ok(())
    };
    add(&definition.reference.manifest.adapter_compatibility)?;
    add(&definition.reference.manifest.content_set_revision)?;
    add(&definition.reference.manifest.localized_text_revision)?;
    add(&definition.reference.manifest.inventory_revision)?;
    add(&definition.reference.entity_kind)?;
    add(&definition.reference.namespaced_id)?;
    if let Some(value) = &definition.origin.package_id {
        add(value)?;
    }
    if let Some(value) = &definition.origin.package_version {
        add(value)?;
    }
    for reference in &definition.override_chain {
        add(reference)?;
    }
    for term_reference in &definition.term_references {
        add(term_reference)?;
    }
    add(&definition.semantic_revision)?;
    add(&definition.localized_text_revision)?;
    if let Some(value) = &definition.display_name {
        add(value)?;
    }
    for alias in &definition.aliases {
        add(alias)?;
    }
    if let Some(value) = &definition.rendered_description {
        add(value)?;
    }
    if let Some(value) = &definition.character_or_pool {
        add(value)?;
    }
    if let Some(value) = &definition.rarity {
        add(value.as_str())?;
    }
    Ok(actual)
}
