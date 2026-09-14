// SPDX-License-Identifier: MIT

mod catalog;
mod lookup;
mod page;
mod reader;
mod summary;

pub use catalog::EventCatalog;
pub use page::*;
pub use reader::EventCatalogReader;

use crate::ContentUnlockState;

use super::{
    EventCost, EventDefinition, EventEffect, EventFieldStatus, EventNarrativePage, EventOption,
    EventOutcome, EventRequirement, EventUnavailableReason, EventVisibility, EventVisibilityScope,
};

fn visible_event(definition: &EventDefinition, scope: EventVisibilityScope) -> bool {
    let unlocked = match definition.unlock_state {
        ContentUnlockState::Unlocked => true,
        ContentUnlockState::Locked => {
            matches!(
                scope,
                EventVisibilityScope::Reference | EventVisibilityScope::Owner
            )
        }
        ContentUnlockState::Unknown => false,
    };
    unlocked && visibility_allowed(definition.visibility, scope)
}

fn visible_page(page: &EventNarrativePage, scope: EventVisibilityScope) -> bool {
    visibility_allowed(page.visibility, scope)
}

fn visible_option(option: &EventOption, scope: EventVisibilityScope) -> bool {
    visibility_allowed(option.visibility, scope)
}

fn visible_outcome(outcome: &EventOutcome, scope: EventVisibilityScope) -> bool {
    visibility_allowed(outcome.visibility, scope)
}

fn visible_requirement(requirement: &EventRequirement, scope: EventVisibilityScope) -> bool {
    visibility_allowed(requirement.visibility, scope)
}

fn visible_cost(cost: &EventCost, scope: EventVisibilityScope) -> bool {
    visibility_allowed(cost.visibility, scope)
}

fn visible_effect(effect: &EventEffect, scope: EventVisibilityScope) -> bool {
    visibility_allowed(effect.visibility, scope)
}

fn visibility_allowed(visibility: EventVisibility, scope: EventVisibilityScope) -> bool {
    match visibility {
        EventVisibility::Visible => true,
        EventVisibility::OwnerOnly => matches!(scope, EventVisibilityScope::Owner),
        EventVisibility::Hidden | EventVisibility::Unknown => false,
    }
}

/// Reports a collection as available, partially withheld, or fully withheld.
///
/// An observed empty collection is `Available`; a collection with only restricted entries is
/// `Denied`; a mixed collection is `Partial`. The visible remainder never includes restricted data.
fn collection_status<T>(
    items: &[T],
    scope: EventVisibilityScope,
    visible: fn(&T, EventVisibilityScope) -> bool,
) -> EventFieldStatus {
    let visible_count = items.iter().filter(|item| visible(item, scope)).count();
    if visible_count == items.len() {
        EventFieldStatus::Available
    } else if visible_count == 0 {
        EventUnavailableReason::Denied.status()
    } else {
        EventFieldStatus::Partial
    }
}

/// Projects a definition to the requested scope, withholding hidden nested records.
pub(super) fn project_definition(
    definition: &EventDefinition,
    scope: EventVisibilityScope,
) -> EventDefinition {
    EventDefinition {
        reference: definition.reference.clone(),
        title: definition.title.clone(),
        kind: definition.kind.clone(),
        unlock_state: definition.unlock_state,
        visibility: definition.visibility,
        pages: definition
            .pages
            .iter()
            .filter(|page| visible_page(page, scope))
            .cloned()
            .collect(),
        eligibility: definition
            .eligibility
            .iter()
            .filter(|requirement| visible_requirement(requirement, scope))
            .cloned()
            .collect(),
        eligibility_status: collection_status(&definition.eligibility, scope, visible_requirement),
        options: definition
            .options
            .iter()
            .filter(|option| visible_option(option, scope))
            .map(|option| project_option(option, scope))
            .collect(),
        references: definition.references.clone(),
    }
}

/// Projects an option to the requested scope, withholding hidden costs and outcomes.
pub(super) fn project_option(option: &EventOption, scope: EventVisibilityScope) -> EventOption {
    EventOption {
        reference: option.reference.clone(),
        text: option.text.clone(),
        requirements: option
            .requirements
            .iter()
            .filter(|requirement| visible_requirement(requirement, scope))
            .cloned()
            .collect(),
        requirements_status: collection_status(&option.requirements, scope, visible_requirement),
        costs: option
            .costs
            .iter()
            .filter(|cost| visible_cost(cost, scope))
            .cloned()
            .collect(),
        costs_status: collection_status(&option.costs, scope, visible_cost),
        outcomes: option
            .outcomes
            .iter()
            .filter(|outcome| visible_outcome(outcome, scope))
            .map(|outcome| project_outcome(outcome, scope))
            .collect(),
        outcomes_status: collection_status(&option.outcomes, scope, visible_outcome),
        references: option.references.clone(),
        visibility: option.visibility,
    }
}

fn project_outcome(outcome: &EventOutcome, scope: EventVisibilityScope) -> EventOutcome {
    EventOutcome {
        reference: outcome.reference.clone(),
        label: outcome.label.clone(),
        probability: outcome.probability.clone(),
        effects: outcome
            .effects
            .iter()
            .filter(|effect| visible_effect(effect, scope))
            .cloned()
            .collect(),
        effects_status: collection_status(&outcome.effects, scope, visible_effect),
        follow_up: outcome.follow_up.clone(),
        references: outcome.references.clone(),
        visibility: outcome.visibility,
    }
}
