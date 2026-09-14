// SPDX-License-Identifier: MIT

use super::super::{EventDefinition, EventOption, EventVisibilityScope};
use super::collection_status;
use super::page::{EventDefinitionSummary, EventOptionSummary};
use super::{visible_cost, visible_option, visible_outcome, visible_page, visible_requirement};

/// Builds a bounded event summary that preserves eligibility availability.
pub(super) fn event_summary(
    definition: &EventDefinition,
    scope: EventVisibilityScope,
) -> EventDefinitionSummary {
    EventDefinitionSummary {
        reference: definition.reference.clone(),
        title: definition.title.clone(),
        kind: definition.kind.clone(),
        page_count: definition
            .pages
            .iter()
            .filter(|page| visible_page(page, scope))
            .count(),
        option_count: definition
            .options
            .iter()
            .filter(|option| visible_option(option, scope))
            .count(),
        eligibility: collection_status(&definition.eligibility, scope, visible_requirement),
    }
}

/// Builds a bounded option summary that preserves collection availability.
pub(super) fn option_summary(
    option: &EventOption,
    scope: EventVisibilityScope,
) -> EventOptionSummary {
    EventOptionSummary {
        reference: option.reference.clone(),
        text: option.text.clone(),
        requirement_count: option
            .requirements
            .iter()
            .filter(|requirement| visible_requirement(requirement, scope))
            .count(),
        requirements_status: collection_status(&option.requirements, scope, visible_requirement),
        cost_count: option
            .costs
            .iter()
            .filter(|cost| visible_cost(cost, scope))
            .count(),
        costs_status: collection_status(&option.costs, scope, visible_cost),
        outcome_count: option
            .outcomes
            .iter()
            .filter(|outcome| visible_outcome(outcome, scope))
            .count(),
        outcomes_status: collection_status(&option.outcomes, scope, visible_outcome),
        visibility: option.visibility,
    }
}
