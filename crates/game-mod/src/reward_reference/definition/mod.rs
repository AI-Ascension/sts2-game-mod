// SPDX-License-Identifier: MIT

mod encoding;
mod generation;
mod item;
mod selection;

pub use generation::*;
pub use item::*;
pub use selection::*;

pub(super) use encoding::definition_bytes;

use crate::ContentUnlockState;

use super::model::{
    RewardCatalogBinding, RewardDefinitionReference, RewardFieldStatus, RewardSemanticReference,
    RewardText, RewardVisibility,
};

/// Coarse reward offer category copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardKind {
    /// Gold or another currency quantity.
    Currency,
    /// A choice between card definitions.
    Card,
    /// A relic grant or choice.
    Relic,
    /// A potion grant or choice.
    Potion,
    /// A special or scripted grant outside the named families.
    SpecialGrant,
    /// Owner-defined reward category.
    Custom(String),
    /// A category is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the reward.
    Unknown,
}

/// Complete source-owned static reward offer definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardOfferDefinitionInput {
    /// Namespaced reward offer identity.
    pub reward_id: String,
    /// Localized reward label.
    pub label: RewardText,
    /// Reward category.
    pub kind: RewardKind,
    /// Explicit unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition itself.
    pub visibility: RewardVisibility,
    /// Selection group with choose/skip constraints and legal actions.
    pub selection: RewardSelectionInput,
    /// Offered items with typed definition/instance references.
    pub items: Vec<RewardItemInput>,
    /// Static generation pool/rarity/eligibility/modifier rules.
    pub generation: Vec<RewardGenerationRule>,
    /// Static support for the distinct offer states.
    pub state_policy: RewardStatePolicy,
    /// Top-level typed references.
    pub references: Vec<RewardSemanticReference>,
}

/// Immutable reward offer definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardOfferDefinition {
    /// Exact static definition reference.
    pub reference: RewardDefinitionReference,
    /// Localized reward label.
    pub label: RewardText,
    /// Reward category.
    pub kind: RewardKind,
    /// Unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition.
    pub visibility: RewardVisibility,
    /// Selection group with choose/skip constraints and legal actions.
    pub selection: RewardSelection,
    /// Offered items with typed definition/instance references.
    pub items: Vec<RewardItem>,
    /// Availability of the offered items after scope withholding.
    pub items_status: RewardFieldStatus,
    /// Static generation rules.
    pub generation: Vec<RewardGenerationRule>,
    /// Availability of the generation rules after scope withholding.
    pub generation_status: RewardFieldStatus,
    /// Static support for the distinct offer states.
    pub state_policy: RewardStatePolicy,
    /// Top-level typed references.
    pub references: Vec<RewardSemanticReference>,
}

impl RewardOfferDefinition {
    /// Binds an input definition and all of its item references to one catalog.
    pub(super) fn from_input(
        binding: &RewardCatalogBinding,
        input: RewardOfferDefinitionInput,
    ) -> Self {
        let reward_id = input.reward_id.clone();
        let items = input
            .items
            .into_iter()
            .map(|item| RewardItem::from_input(&reward_id, binding, item))
            .collect();
        Self {
            reference: RewardDefinitionReference {
                catalog: binding.clone(),
                reward_id,
            },
            label: input.label,
            kind: input.kind,
            unlock_state: input.unlock_state,
            visibility: input.visibility,
            selection: RewardSelection::from_input(input.selection),
            items,
            items_status: RewardFieldStatus::Available,
            generation: input.generation,
            generation_status: RewardFieldStatus::Available,
            state_policy: input.state_policy,
            references: input.references,
        }
    }
}
