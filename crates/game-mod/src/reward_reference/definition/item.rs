// SPDX-License-Identifier: MIT

use super::super::model::{
    RewardCatalogBinding, RewardEvidence, RewardField, RewardItemInstanceReference,
    RewardItemReference, RewardQuantity, RewardSemanticReference, RewardText, RewardVisibility,
};

/// One offered item before manifest binding.
///
/// The item carries a typed content definition reference and an optional live instance reference,
/// so an offered card, relic, or potion is inspectable before it is in any deck or inventory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardItemInput {
    /// Stable item identity scoped by the reward definition.
    pub item_id: String,
    /// Localized/source-defined item label.
    pub label: RewardText,
    /// Typed content definition reference for this item.
    pub reference: RewardSemanticReference,
    /// Exact typed quantity, preserving currency units and visible modified values.
    pub quantity: RewardQuantity,
    /// Live item instance when the item is an already-existing instance.
    pub instance: RewardField<RewardItemInstanceReference>,
    /// Evidence label for this item.
    pub evidence: RewardEvidence,
    /// Visibility of the static item.
    pub visibility: RewardVisibility,
}

/// Offered item bound to its owning reward definition and catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardItem {
    /// Exact static item reference.
    pub reference: RewardItemReference,
    /// Localized/source-defined item label.
    pub label: RewardText,
    /// Typed content definition reference for this item.
    pub definition: RewardSemanticReference,
    /// Exact typed quantity.
    pub quantity: RewardQuantity,
    /// Live item instance when the item is an already-existing instance.
    pub instance: RewardField<RewardItemInstanceReference>,
    /// Evidence label for this item.
    pub evidence: RewardEvidence,
    /// Visibility of the static item.
    pub visibility: RewardVisibility,
}

impl RewardItem {
    pub(super) fn from_input(
        reward_id: &str,
        binding: &RewardCatalogBinding,
        input: RewardItemInput,
    ) -> Self {
        Self {
            reference: RewardItemReference {
                catalog: binding.clone(),
                reward_id: reward_id.to_owned(),
                item_id: input.item_id,
            },
            label: input.label,
            definition: input.reference,
            quantity: input.quantity,
            instance: input.instance,
            evidence: input.evidence,
            visibility: input.visibility,
        }
    }
}
