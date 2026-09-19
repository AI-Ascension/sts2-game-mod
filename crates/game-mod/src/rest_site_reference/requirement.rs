// SPDX-License-Identifier: MIT

use super::{
    field::{RestField, RestText},
    model::RestSemanticReference,
};

/// Resolved state of one requirement or candidate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestRequirementState {
    /// The requirement is currently met.
    Satisfied,
    /// The requirement is currently unmet.
    Unsatisfied,
    /// The host could not resolve the requirement.
    Unknown,
    /// The requirement has no meaning for the observed site or mode.
    NotApplicable,
}

/// One requirement a rest option or selection imposes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestRequirement {
    /// Stable requirement identity within its option.
    pub requirement_id: String,
    /// Localized requirement text.
    pub label: RestText,
    /// Resolved requirement state.
    pub state: RestRequirementState,
    /// Resolved public parameter, or an explicit non-value.
    pub detail: RestField<String>,
    /// Definitions this requirement refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Unit one rest cost is denominated in.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestCostUnit {
    /// Current health.
    Health,
    /// Maximum health.
    MaxHealth,
    /// Gold.
    Gold,
    /// A card in the deck.
    Card,
    /// A held relic.
    Relic,
    /// A held potion.
    Potion,
    /// A charge or counter on a held object.
    Charge,
    /// Owner-defined cost unit.
    Custom(String),
    /// Source could not classify the cost unit.
    Unknown,
}

/// One cost a rest option charges without being performed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestCost {
    /// Stable cost identity within its option.
    pub cost_id: String,
    /// Localized cost text.
    pub label: RestText,
    /// Unit the amount is denominated in.
    pub unit: RestCostUnit,
    /// Resolved amount, or an explicit non-value.
    pub amount: RestField<i64>,
    /// Definitions this cost refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Scope one rest limit applies to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestLimitUnit {
    /// The limit applies once per rest site.
    PerSite,
    /// The limit applies once per run.
    PerRun,
    /// The limit applies once per option set.
    PerOptionSet,
    /// Owner-defined limit unit.
    Custom(String),
    /// Source could not classify the limit unit.
    Unknown,
}

/// One limit that bounds a rest option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestLimit {
    /// Stable limit identity within its option.
    pub limit_id: String,
    /// Localized limit text.
    pub label: RestText,
    /// Scope the limit applies to.
    pub unit: RestLimitUnit,
    /// Remaining uses, or an explicit non-value.
    pub remaining: RestField<u32>,
    /// Definitions this limit refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Domain a rest option selects a target from.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestSelectionDomain {
    /// The option selects nothing.
    None,
    /// The option selects a card in the deck.
    OwnedCards,
    /// The option selects a card that may be upgraded.
    UpgradeCandidates,
    /// The option selects a held relic.
    RelicsHeld,
    /// The option selects a potion slot.
    PotionSlots,
    /// The option selects a co-op player.
    CoopPlayers,
    /// Owner-defined selection domain.
    Custom(String),
    /// Source could not classify the selection domain.
    Unknown,
}

/// One selectable candidate for a rest option, resolved without performing the option.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestCandidate {
    /// Stable candidate identity within its selection.
    pub candidate_id: String,
    /// Localized candidate label.
    pub label: RestText,
    /// Definition the candidate resolves to.
    pub reference: RestSemanticReference,
    /// Resolved eligibility of the candidate.
    pub eligibility: RestRequirementState,
    /// Resolved public parameter, or an explicit non-value.
    pub detail: RestField<String>,
    /// Definitions the candidate refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Selection one rest option requires, including its bounded candidate list.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestSelectionRequirement {
    /// Domain the option selects from.
    pub domain: RestSelectionDomain,
    /// Whether a selection is required to perform the option.
    pub required: bool,
    /// Minimum number of selections, or an explicit non-value.
    pub minimum: RestField<u32>,
    /// Maximum number of selections, or an explicit non-value.
    pub maximum: RestField<u32>,
    /// Resolved candidates for this selection.
    pub candidates: Vec<RestCandidate>,
    /// Definitions this selection refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Whether a selection domain selects nothing.
pub(super) const fn domain_is_empty(domain: &RestSelectionDomain) -> bool {
    matches!(domain, RestSelectionDomain::None)
}
