// SPDX-License-Identifier: MIT

mod edges;
mod values;

use std::collections::{BTreeMap, BTreeSet};

use self::edges::{collect_options, validate_membership, validate_reference_edges};
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
    EVENT_MAX_REQUIREMENTS, EventVisibility, validate_identity, visibility_rank,
};

/// Shallow event context shared by scope-aware reference and follow-up checks.
pub(super) struct EventScope<'a> {
    event_id: &'a str,
    pages: &'a BTreeMap<&'a str, EventVisibility>,
    options: &'a BTreeMap<&'a str, EventVisibility>,
}

/// Validates one source-owned event definition before it enters an immutable catalog.
pub(super) fn validate_definition(
    input: &EventDefinitionInput,
    event_visibility: &BTreeMap<String, EventVisibility>,
) -> Result<(), EventCatalogError> {
    validate_identity(&input.event_id, "event_id")?;
    validate_text_value(&input.title, "title")?;
    validate_event_kind(&input.kind)?;
    validate_visibility(input.visibility)?;
    let options = collect_options(&input.options)?;
    let pages = validate_pages(&input.pages)?;
    validate_membership(&input.event_id, &input.pages, &options)?;
    let scope = EventScope {
        event_id: &input.event_id,
        pages: &pages,
        options: &options,
    };
    for page in &input.pages {
        validate_reference_edges(&scope, event_visibility, page.visibility, &page.references)?;
    }
    validate_requirements("eligibility", &input.eligibility, &scope, event_visibility)?;
    for option in &input.options {
        validate_option(option, &scope, event_visibility)?;
    }
    validate_references(&input.references)?;
    validate_reference_edges(
        &scope,
        event_visibility,
        input.visibility,
        &input.references,
    )
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
        let mut offered = BTreeSet::new();
        if page.offered_options.len() > EVENT_MAX_OPTIONS {
            return Err(EventCatalogError::InvalidInput("page_options"));
        }
        for option_id in &page.offered_options {
            validate_identity(option_id, "page_option")?;
            if !offered.insert(option_id.as_str()) {
                return Err(EventCatalogError::InvalidInput("duplicate_page_option"));
            }
        }
        if map.insert(page.page_id.as_str(), page.visibility).is_some() {
            return Err(EventCatalogError::InvalidInput("duplicate_page"));
        }
    }
    Ok(map)
}

fn validate_requirements(
    field: &'static str,
    requirements: &[EventRequirement],
    scope: &EventScope<'_>,
    event_visibility: &BTreeMap<String, EventVisibility>,
) -> Result<(), EventCatalogError> {
    if requirements.len() > EVENT_MAX_REQUIREMENTS {
        return Err(EventCatalogError::InvalidInput(field));
    }
    let mut ids = BTreeSet::new();
    for requirement in requirements {
        validate_requirement(requirement)?;
        validate_reference_edges(
            scope,
            event_visibility,
            requirement.visibility,
            &requirement.references,
        )?;
        if !ids.insert(requirement.requirement_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_requirement"));
        }
    }
    Ok(())
}

fn validate_option(
    option: &EventOptionInput,
    scope: &EventScope<'_>,
    event_visibility: &BTreeMap<String, EventVisibility>,
) -> Result<(), EventCatalogError> {
    validate_identity(&option.option_id, "option_id")?;
    validate_text_value(&option.text, "option_text")?;
    validate_visibility(option.visibility)?;
    validate_requirements(
        "option_requirements",
        &option.requirements,
        scope,
        event_visibility,
    )?;
    validate_costs(&option.costs, scope, event_visibility)?;
    validate_outcomes(&option.outcomes, scope, event_visibility)?;
    validate_references(&option.references)?;
    validate_reference_edges(
        scope,
        event_visibility,
        option.visibility,
        &option.references,
    )
}

fn validate_costs(
    costs: &[EventCost],
    scope: &EventScope<'_>,
    event_visibility: &BTreeMap<String, EventVisibility>,
) -> Result<(), EventCatalogError> {
    if costs.len() > EVENT_MAX_COSTS {
        return Err(EventCatalogError::InvalidInput("costs"));
    }
    let mut ids = BTreeSet::new();
    for cost in costs {
        validate_cost(cost)?;
        validate_reference_edges(scope, event_visibility, cost.visibility, &cost.references)?;
        if !ids.insert(cost.cost_id.as_str()) {
            return Err(EventCatalogError::InvalidInput("duplicate_cost"));
        }
    }
    Ok(())
}

fn validate_outcomes(
    outcomes: &[EventOutcomeInput],
    scope: &EventScope<'_>,
    event_visibility: &BTreeMap<String, EventVisibility>,
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
            validate_reference_edges(
                scope,
                event_visibility,
                effect.visibility,
                &effect.references,
            )?;
        }
        validate_follow_up(scope, outcome)?;
        validate_references(&outcome.references)?;
        validate_reference_edges(
            scope,
            event_visibility,
            outcome.visibility,
            &outcome.references,
        )?;
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
