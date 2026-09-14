// SPDX-License-Identifier: MIT

use super::super::model::{
    EventCatalogBinding, EventEvidence, EventField, EventNumericValue, EventOptionReference,
    EventOutcomeReference, EventProbability, EventSemanticReference, EventText,
    EventUnavailableReason, EventVisibility,
};

/// One visible parameter retained without evaluating hidden state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventParameter {
    /// Stable parameter identity.
    pub parameter_id: String,
    /// Localized/source-defined label.
    pub label: EventText,
    /// Optional unit.
    pub unit: Option<String>,
    /// Fixed, formula-backed, or unavailable value.
    pub value: EventNumericValue,
}

/// Coarse option/event requirement category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventRequirementKind {
    /// A minimum or exact resource amount (gold, HP, potion slots).
    Resource,
    /// Character or loadout identity.
    Character,
    /// Progression or act-order requirement.
    Progression,
    /// Progression/unlock requirement.
    Unlock,
    /// Content-configuration flag.
    ContentFlag,
    /// Game mode identity.
    Mode,
    /// Owner rule reference.
    Rule,
    /// Owner-defined predicate.
    Custom(String),
    /// Source could not classify the predicate.
    Unknown,
}

/// One typed requirement for an event or one of its options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventRequirement {
    /// Stable requirement identity.
    pub requirement_id: String,
    /// Predicate category.
    pub kind: EventRequirementKind,
    /// Localized/source-defined requirement label.
    pub label: EventText,
    /// Typed parameters used by the requirement.
    pub parameters: Vec<EventParameter>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Visibility of the static requirement.
    pub visibility: EventVisibility,
}

/// Coarse structured-cost category.
///
/// Costs distinguish HP loss, max-HP change, gold, and item removal from owner-defined
/// resources, and never collapse an unknown resource into a named one.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventCostKind {
    /// Lose current HP.
    HpLoss,
    /// Change maximum HP.
    MaxHpChange,
    /// Spend or lose gold.
    Gold,
    /// Remove a card.
    CardRemoval,
    /// Remove or consume a potion.
    PotionRemoval,
    /// Remove or lose a relic.
    RelicRemoval,
    /// Owner-defined item or resource removal.
    ItemRemoval,
    /// Owner rule reference.
    Rule,
    /// Owner-defined cost.
    Custom(String),
    /// A cost is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the cost.
    Unknown,
}

/// One structured cost for an option, with a rule reference and evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCost {
    /// Stable cost identity scoped by the option.
    pub cost_id: String,
    /// Cost category.
    pub kind: EventCostKind,
    /// Localized/source-defined cost label.
    pub label: EventText,
    /// Fixed, formula-backed, or unavailable amount.
    pub amount: EventNumericValue,
    /// Optional resource identity the amount applies to.
    pub resource: EventField<String>,
    /// Owner rule reference, preserving unavailable/empty distinctions.
    pub rule_reference: EventField<String>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Evidence label for this cost.
    pub evidence: EventEvidence,
    /// Visibility of the static cost.
    pub visibility: EventVisibility,
}

/// Coarse possible-effect/outcome category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventEffectKind {
    /// Gain a card.
    AddCard,
    /// Remove a card.
    RemoveCard,
    /// Upgrade or transform a card.
    ModifyCard,
    /// Gain a relic.
    GainRelic,
    /// Lose a relic.
    LoseRelic,
    /// Gain a potion.
    GainPotion,
    /// Lose or consume a potion.
    LosePotion,
    /// Gain gold.
    GainGold,
    /// Lose gold.
    LoseGold,
    /// Heal HP.
    Heal,
    /// Take damage.
    Damage,
    /// Change maximum HP.
    MaxHpChange,
    /// Advance to another narrative page.
    FollowUp,
    /// Owner rule reference.
    Rule,
    /// Owner-defined effect.
    Custom(String),
    /// An effect is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the effect.
    Unknown,
}

/// One possible effect of an outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEffect {
    /// Stable effect identity scoped by the outcome.
    pub effect_id: String,
    /// Effect category.
    pub kind: EventEffectKind,
    /// Localized/source-defined effect label.
    pub label: EventText,
    /// Fixed, formula-backed, or unavailable amount.
    pub amount: EventNumericValue,
    /// Optional target/resource identity.
    pub target: EventField<String>,
    /// Owner rule reference.
    pub rule_reference: EventField<String>,
    /// Typed rule/content links.
    pub references: Vec<EventSemanticReference>,
    /// Evidence label for this effect.
    pub evidence: EventEvidence,
    /// Visibility of the static effect.
    pub visibility: EventVisibility,
}

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
            follow_up: input.follow_up,
            references: input.references,
            visibility: input.visibility,
        }
    }
}

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
    /// Structured costs for this option.
    pub costs: Vec<EventCost>,
    /// Possible outcomes of this option.
    pub outcomes: Vec<EventOutcome>,
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
            costs: input.costs,
            outcomes,
            references: input.references,
            visibility: input.visibility,
        }
    }
}
