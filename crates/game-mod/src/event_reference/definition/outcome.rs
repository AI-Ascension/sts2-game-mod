// SPDX-License-Identifier: MIT

use super::super::model::{
    EventCatalogBinding, EventFieldStatus, EventOutcomeReference, EventProbability,
    EventSemanticReference, EventText, EventUnavailableReason, EventVisibility,
};
use super::EventEffect;

/// Explicit follow-up after an outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventFollowUp {
    /// Continue at a narrative page within the same event definition.
    Page(String),
    /// The event ends after this outcome.
    End,
    /// The follow-up is known but not safely available.
    Unavailable(EventUnavailableReason),
}

/// Source-owned possible outcome before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOutcomeInput {
    /// Stable outcome identity scoped by the option.
    pub outcome_id: String,
    /// Localized/source-defined outcome label.
    pub label: EventText,
    /// Evidence-qualified probability, never a sampled result.
    pub probability: EventProbability,
    /// Possible effects of this outcome.
    pub effects: Vec<EventEffect>,
    /// Follow-up page or explicit terminal/unavailable state.
    pub follow_up: EventFollowUp,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Visibility of the static outcome.
    pub visibility: EventVisibility,
}

/// Possible outcome bound to its owning option and catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventOutcome {
    /// Exact static outcome reference.
    pub reference: EventOutcomeReference,
    /// Localized/source-defined outcome label.
    pub label: EventText,
    /// Evidence-qualified probability.
    pub probability: EventProbability,
    /// Possible effects of this outcome.
    pub effects: Vec<EventEffect>,
    /// Availability of this outcome's effects after scope withholding.
    pub effects_status: EventFieldStatus,
    /// Follow-up page or explicit terminal/unavailable state.
    pub follow_up: EventFollowUp,
    /// Typed references.
    pub references: Vec<EventSemanticReference>,
    /// Visibility of the static outcome.
    pub visibility: EventVisibility,
}

impl EventOutcome {
    pub(super) fn from_input(
        event_id: &str,
        option_id: &str,
        binding: &EventCatalogBinding,
        input: EventOutcomeInput,
    ) -> Self {
        Self {
            reference: EventOutcomeReference {
                catalog: binding.clone(),
                event_id: event_id.to_owned(),
                option_id: option_id.to_owned(),
                outcome_id: input.outcome_id,
            },
            label: input.label,
            probability: input.probability,
            effects: input.effects,
            effects_status: EventFieldStatus::Available,
            follow_up: input.follow_up,
            references: input.references,
            visibility: input.visibility,
        }
    }
}
