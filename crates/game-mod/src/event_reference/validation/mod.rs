// SPDX-License-Identifier: MIT

mod values;

use std::collections::{BTreeMap, BTreeSet};

use self::values::{
    validate_cost, validate_effects, validate_probability, validate_references,
    validate_requirement, validate_text_value, validate_visibility,
};

use super::EventCatalogError;
use super::definition::{
    EventCost, EventDefinitionInput, EventFollowUp, EventKind, EventNarrativePage,
    EventOptionInput, EventOutcomeInput, EventRequirement,
};
use super::model::{
    EVENT_MAX_COSTS, EVENT_MAX_OPTIONS, EVENT_MAX_OUTCOMES, EVENT_MAX_PAGES,
    EVENT_MAX_REQUIREMENTS, EventSemanticReference, EventSemanticReferenceKind, EventVisibility,
    validate_identity, visibility_rank,
};

/// Shallow event context shared by scope-aware reference and follow-up checks.
struct EventScope<'a> {
    event_id: &'a str,
    pages: &'a BTreeMap<&'a str, EventVisibility>,
    options: &'a BTreeSet<&'a str>,
}

/// Validates one source-owned event definition before it enters an immutable catalog.
pub(super) fn validate_definition(input: &EventDefinitionInput) -> Result<(), EventCatalogError> {
    validate_identity(&input.event_id, "event_id")?;
    validate_text_value(&input.title, "title")?;
    validate_event_kind(&input.kind)?;
    validate_visibility(input.visibility)?;
    let pages = validate_pages(&input.pages)?;
    let options = collect_option_ids(&input.options)?;
    let scope = EventScope {
        event_id: &input.event_id,
        pages: &pages,
        options: &options,
    };
    for page in &input.pages {
        validate_intra_event_references(&scope, &page.references)?;
    }
    validate_requirements("eligibility", &input.eligibility, &scope)?;
    for option in &input.options {
        validate_option(option, &scope)?;
    }
    validate_references(&input.references)?;
    validate_intra_event_references(&scope, &input.references)
}

fn validate_event_kind(kind: &EventKind) -> Result<(), EventCatalogError> {
    if let EventKind::Custom(value) | EventKind::Unsupported(value) = kind {
        validate_identity(value, "event_kind")?;
    }
    Ok(())
}

fn validate_pages(
    pages: &[EventNarrativePage],
) -> Result<BTreeMap<&str, EventVisibility>, EventCatalogError> {
    if pages.len() > EVENT_MAX_PAGES {
        return Err(EventCatalogError::InvalidInput("pages"));
    }
    let mut map = BTreeMap::new();
    for page in pages {
        validate_identity(&page.page_id, "page_id")?;
        validate_text_value(&page.narrative, "narrative")?;
        validate_references(&page.references)?;
        validate_visibility(page.visibility)?;
        if map.insert(page.page_id.as_str(), page.visibility).is_some() {
            return Err(EventCatalogError::InvalidInput("duplicate_page"));
        }
    }
    Ok(map)
}

fn collect_option_ids(options: &[EventOptionInput]) -> Result<BTreeSet<&str>, EventCatalogError> {
    if options.len() > EVENT_MAX_OPTIONS {
        return Err(EventCatalogError::InvalidInput("options"));
    }
    let mut ids = BTreeSet::new();
    for option in options {
        validate_identity(&option.option_id, "option_id")?;
        if !ids.insert(option.option_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_option"));
        }
    }
    Ok(ids)
}

fn validate_requirements(
    field: &'static str,
    requirements: &[EventRequirement],
    scope: &EventScope<'_>,
) -> Result<(), EventCatalogError> {
    if requirements.len() > EVENT_MAX_REQUIREMENTS {
        return Err(EventCatalogError::InvalidInput(field));
    }
    let mut ids = BTreeSet::new();
    for requirement in requirements {
        validate_requirement(requirement)?;
        validate_intra_event_references(scope, &requirement.references)?;
        if !ids.insert(requirement.requirement_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_requirement"));
        }
    }
    Ok(())
}

fn validate_option(
    option: &EventOptionInput,
    scope: &EventScope<'_>,
) -> Result<(), EventCatalogError> {
    validate_identity(&option.option_id, "option_id")?;
    validate_text_value(&option.text, "option_text")?;
    validate_visibility(option.visibility)?;
    validate_requirements("option_requirements", &option.requirements, scope)?;
    validate_costs(&option.costs, scope)?;
    validate_outcomes(&option.outcomes, scope)?;
    validate_references(&option.references)?;
    validate_intra_event_references(scope, &option.references)
}

fn validate_costs(costs: &[EventCost], scope: &EventScope<'_>) -> Result<(), EventCatalogError> {
    if costs.len() > EVENT_MAX_COSTS {
        return Err(EventCatalogError::InvalidInput("costs"));
    }
    let mut ids = BTreeSet::new();
    for cost in costs {
        validate_cost(cost)?;
        validate_intra_event_references(scope, &cost.references)?;
        if !ids.insert(cost.cost_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_cost"));
        }
    }
    Ok(())
}

fn validate_outcomes(
    outcomes: &[EventOutcomeInput],
    scope: &EventScope<'_>,
) -> Result<(), EventCatalogError> {
    if outcomes.len() > EVENT_MAX_OUTCOMES {
        return Err(EventCatalogError::InvalidInput("outcomes"));
    }
    let mut ids = BTreeSet::new();
    for outcome in outcomes {
        validate_identity(&outcome.outcome_id, "outcome_id")?;
        validate_text_value(&outcome.label, "outcome_label")?;
        validate_visibility(outcome.visibility)?;
        validate_probability(&outcome.probability)?;
        validate_effects(&outcome.effects)?;
        for effect in &outcome.effects {
            validate_intra_event_references(scope, &effect.references)?;
        }
        validate_follow_up(scope, outcome)?;
        validate_references(&outcome.references)?;
        validate_intra_event_references(scope, &outcome.references)?;
        if !ids.insert(outcome.outcome_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_outcome"));
        }
    }
    Ok(())
}

fn validate_follow_up(
    scope: &EventScope<'_>,
    outcome: &EventOutcomeInput,
) -> Result<(), EventCatalogError> {
    match &outcome.follow_up {
        EventFollowUp::Page(page_id) => {
            let Some(page_visibility) = scope.pages.get(page_id.as_str()) else {
                return Err(EventCatalogError::UnknownPageReference {
                    event_id: scope.event_id.to_owned(),
                    page_id: page_id.clone(),
                });
            };
            if visibility_rank(outcome.visibility) > visibility_rank(*page_visibility) {
                return Err(EventCatalogError::HiddenFutureLeak {
                    event_id: scope.event_id.to_owned(),
                    page_id: page_id.clone(),
                });
            }
            Ok(())
        }
        EventFollowUp::End | EventFollowUp::Unavailable(_) => Ok(()),
    }
}

fn validate_intra_event_references(
    scope: &EventScope<'_>,
    references: &[EventSemanticReference],
) -> Result<(), EventCatalogError> {
    for reference in references {
        if matches!(reference.kind, EventSemanticReferenceKind::Page)
            && !scope.pages.contains_key(reference.id.as_str())
        {
            return Err(EventCatalogError::UnknownPageReference {
                event_id: scope.event_id.to_owned(),
                page_id: reference.id.clone(),
            });
        }
        if matches!(reference.kind, EventSemanticReferenceKind::Option)
            && !scope.options.contains(reference.id.as_str())
        {
            return Err(EventCatalogError::UnknownOptionReference {
                event_id: scope.event_id.to_owned(),
                option_id: reference.id.clone(),
            });
        }
    }
    Ok(())
}
