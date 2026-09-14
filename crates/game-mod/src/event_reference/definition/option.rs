// SPDX-License-Identifier: MIT

use super::super::model::{
    EventCatalogBinding, EventFieldStatus, EventOptionReference, EventSemanticReference, EventText,
    EventVisibility,
};
use super::{EventCost, EventOutcome, EventOutcomeInput, EventRequirement};

/// Source-owned choice before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOptionInput {
    /// Stable option identity scoped by the event.
    pub option_id: String,
    /// Localized option text.
    pub text: EventText,
    /// Typed requirements for this option.
    pub requirements: Vec<EventRequirement>,
    /// Structured costs for this option.
    pub costs: Vec<EventCost>,
    /// Possible outcomes of this option.
    pub outcomes: Vec<EventOutcomeInput>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Visibility of the static option.
    pub visibility: EventVisibility,
}

/// Choice bound to its owning event and catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOption {
    /// Exact static option reference.
    pub reference: EventOptionReference,
    /// Localized option text.
    pub text: EventText,
    /// Typed requirements for this option.
    pub requirements: Vec<EventRequirement>,
    /// Availability of this option's requirements after scope withholding.
    pub requirements_status: EventFieldStatus,
    /// Structured costs for this option.
    pub costs: Vec<EventCost>,
    /// Availability of this option's costs after scope withholding.
    pub costs_status: EventFieldStatus,
    /// Possible outcomes of this option.
    pub outcomes: Vec<EventOutcome>,
    /// Availability of this option's outcomes after scope withholding.
    pub outcomes_status: EventFieldStatus,
    /// Typed references.
    pub references: Vec<EventSemanticReference>,
    /// Visibility of the static option.
    pub visibility: EventVisibility,
}

impl EventOption {
    pub(super) fn from_input(
        event_id: &str,
        binding: &EventCatalogBinding,
        input: EventOptionInput,
    ) -> Self {
        let option_id = input.option_id.clone();
        let outcomes = input
            .outcomes
            .into_iter()
            .map(|outcome| EventOutcome::from_input(event_id, &option_id, binding, outcome))
            .collect();
        Self {
            reference: EventOptionReference {
                catalog: binding.clone(),
                event_id: event_id.to_owned(),
                option_id,
            },
            text: input.text,
            requirements: input.requirements,
            requirements_status: EventFieldStatus::Available,
            costs: input.costs,
            costs_status: EventFieldStatus::Available,
            outcomes,
            outcomes_status: EventFieldStatus::Available,
            references: input.references,
            visibility: input.visibility,
        }
    }
}
