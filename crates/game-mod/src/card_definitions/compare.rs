// SPDX-License-Identifier: MIT

use super::error::{CardDefinitionError, stale_if_mismatched};
use super::variants::{CardDefinitionCatalog, CardVariantKind, CardVariantReference};
use super::{CardCost, CardEffectParameter, CardStructuralModifier, CardTargeting, CardTextValue};

/// Before/after value for one changed variant field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardFieldChange<T> {
    /// Value from the base/reference variant.
    pub before: T,
    /// Value from the compared variant.
    pub after: T,
}

/// Read-only comparison of two source-owned card variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardVariantComparison {
    /// Reference used as the comparison base.
    pub base: CardVariantReference,
    /// Reference being compared with the base.
    pub variant: CardVariantReference,
    /// Variant category change, when different.
    pub kind: Option<CardFieldChange<CardVariantKind>>,
    /// Upgrade-level change, when different.
    pub upgrade_level: Option<CardFieldChange<Option<u16>>>,
    /// Localized title change, when different.
    pub title: Option<CardFieldChange<CardTextValue>>,
    /// Localized description change, when different.
    pub description: Option<CardFieldChange<CardTextValue>>,
    /// Structured cost change, when different.
    pub cost: Option<CardFieldChange<CardCost>>,
    /// Targeting change, when different.
    pub targeting: Option<CardFieldChange<CardTargeting>>,
    /// Keyword collection change, when different.
    pub keywords: Option<CardFieldChange<Vec<String>>>,
    /// Typed effect parameter change, when different.
    pub effects: Option<CardFieldChange<Vec<CardEffectParameter>>>,
    /// Structural modifier change, when different.
    pub structural_modifiers: Option<CardFieldChange<Vec<CardStructuralModifier>>>,
}

impl CardDefinitionCatalog {
    /// Compares two variants without constructing or mutating a live card.
    pub fn compare(
        &self,
        base: &CardVariantReference,
        variant: &CardVariantReference,
    ) -> Result<CardVariantComparison, CardDefinitionError> {
        stale_if_mismatched(&self.manifest, &base.card)?;
        stale_if_mismatched(&self.manifest, &variant.card)?;
        if base.card.entity_kind != variant.card.entity_kind
            || base.card.namespaced_id != variant.card.namespaced_id
        {
            return Err(CardDefinitionError::VariantDefinitionMismatch);
        }
        let base_value = self.variant(base)?;
        let variant_value = self.variant(variant)?;
        Ok(CardVariantComparison {
            base: base.clone(),
            variant: variant.clone(),
            kind: changed(base_value.kind, variant_value.kind),
            upgrade_level: changed(base_value.upgrade_level, variant_value.upgrade_level),
            title: changed(base_value.title.clone(), variant_value.title.clone()),
            description: changed(
                base_value.description.clone(),
                variant_value.description.clone(),
            ),
            cost: changed(base_value.cost.clone(), variant_value.cost.clone()),
            targeting: changed(
                base_value.targeting.clone(),
                variant_value.targeting.clone(),
            ),
            keywords: changed(base_value.keywords.clone(), variant_value.keywords.clone()),
            effects: changed(base_value.effects.clone(), variant_value.effects.clone()),
            structural_modifiers: changed(
                base_value.structural_modifiers.clone(),
                variant_value.structural_modifiers.clone(),
            ),
        })
    }
}

fn changed<T: Eq>(before: T, after: T) -> Option<CardFieldChange<T>> {
    (before != after).then_some(CardFieldChange { before, after })
}
