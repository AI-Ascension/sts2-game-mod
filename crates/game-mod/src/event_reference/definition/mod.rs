// SPDX-License-Identifier: MIT

mod cost;
mod effect;
mod encoding;
mod option;
mod outcome;
mod requirement;

pub use cost::*;
pub use effect::*;
pub use option::*;
pub use outcome::*;
pub use requirement::*;

pub(super) use encoding::definition_bytes;

use crate::ContentUnlockState;

use super::EventSemanticReference;
use super::model::{
    EventCatalogBinding, EventDefinitionReference, EventFieldStatus, EventText, EventVisibility,
};

/// Coarse event category copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventKind {
    /// Ordinary event node.
    Normal,
    /// Shared/collective or co-op event.
    Shared,
    /// Act-ending or boss-locked event.
    Boss,
    /// Shop or merchant event.
    Shop,
    /// Rest or campfire event.
    Rest,
    /// Curse or shrine event.
    Shrine,
    /// Owner-defined event category.
    Custom(String),
    /// A category is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the event.
    Unknown,
}

/// One localized narrative page in an event.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventNarrativePage {
    /// Stable page identity scoped by the event.
    pub page_id: String,
    /// Localized narrative text.
    pub narrative: EventText,
    /// Typed rule/content links associated with this page.
    pub references: Vec<EventSemanticReference>,
    /// Options this page offers to the reader.
    ///
    /// This is the authoritative page-to-option membership relation: every option is offered by
    /// exactly one page, and a page never offers an option more restricted than itself.
    pub offered_options: Vec<String>,
    /// Visibility of the static page.
    pub visibility: EventVisibility,
}

/// Complete source-owned static event definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDefinitionInput {
    /// Namespaced event identity.
    pub event_id: String,
    /// Localized event title.
    pub title: EventText,
    /// Event category.
    pub kind: EventKind,
    /// Explicit unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition itself.
    pub visibility: EventVisibility,
    /// Bounded narrative pages.
    pub pages: Vec<EventNarrativePage>,
    /// Eligibility predicates for the event itself.
    pub eligibility: Vec<EventRequirement>,
    /// Choices with stable option identities.
    pub options: Vec<EventOptionInput>,
    /// Top-level typed references.
    pub references: Vec<EventSemanticReference>,
}

/// Immutable event definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventDefinition {
    /// Exact static definition reference.
    pub reference: EventDefinitionReference,
    /// Localized event title.
    pub title: EventText,
    /// Event category.
    pub kind: EventKind,
    /// Unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition.
    pub visibility: EventVisibility,
    /// Bounded narrative pages.
    pub pages: Vec<EventNarrativePage>,
    /// Eligibility predicates for the event itself.
    pub eligibility: Vec<EventRequirement>,
    /// Availability of eligibility predicates after scope withholding.
    pub eligibility_status: EventFieldStatus,
    /// Choices with stable option identities.
    pub options: Vec<EventOption>,
    /// Top-level references.
    pub references: Vec<EventSemanticReference>,
}

impl EventDefinition {
    /// Binds an input definition and all of its option/outcome references to one catalog.
    pub(super) fn from_input(binding: &EventCatalogBinding, input: EventDefinitionInput) -> Self {
        let event_id = input.event_id.clone();
        let options = input
            .options
            .into_iter()
            .map(|option| EventOption::from_input(&event_id, binding, option))
            .collect();
        Self {
            reference: EventDefinitionReference {
                catalog: binding.clone(),
                event_id,
            },
            title: input.title,
            kind: input.kind,
            unlock_state: input.unlock_state,
            visibility: input.visibility,
            pages: input.pages,
            eligibility: input.eligibility,
            eligibility_status: EventFieldStatus::Available,
            options,
            references: input.references,
        }
    }
}
