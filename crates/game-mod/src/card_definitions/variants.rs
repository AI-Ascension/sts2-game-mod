// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use crate::{ContentCursorBinding, ContentDefinitionReference, ContentOriginInput};

use super::model::{
    CardCost, CardEffectParameter, CardOptionalText, CardRarity, CardStructuralModifier,
    CardTargeting, CardTextValue, CardType,
};
use super::rules::{CardAcquisitionRule, CardUnlockRule};

/// Distinct owner-defined variant category.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardVariantKind {
    /// The unmodified definition.
    Base,
    /// A supported upgrade variant.
    Upgrade,
    /// A distinct enchantment or modifier variant.
    Enchantment,
    /// A generated card variant.
    Generated,
    /// Another supported alternate variant.
    Alternate,
}

/// One variant copied from a card definition source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardVariantInput {
    /// Stable variant ID scoped by the containing card definition.
    pub variant_id: String,
    /// Variant category; IDs are never inferred from display names.
    pub kind: CardVariantKind,
    /// Upgrade level for an upgrade variant.
    pub upgrade_level: Option<u16>,
    /// Localized title for the snapshot locale.
    pub title: CardTextValue,
    /// Localized description for the snapshot locale.
    pub description: CardTextValue,
    /// Structured cost.
    pub cost: CardCost,
    /// Owner-defined targeting.
    pub targeting: CardTargeting,
    /// Keywords copied as stable owner values.
    pub keywords: Vec<String>,
    /// Typed effect parameters.
    pub effects: Vec<CardEffectParameter>,
    /// Structural modifiers kept separate from rendered text.
    pub structural_modifiers: Vec<CardStructuralModifier>,
}

/// Explicit upgrade path; ordering is the source-defined level order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardUpgradePath {
    /// Stable path identity.
    pub path_id: String,
    /// Variant IDs in increasing upgrade order.
    pub variant_ids: Vec<String>,
}

/// One source-owned card definition input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinitionInput {
    /// Existing manifest namespaced card identity.
    pub namespaced_id: String,
    /// Owner-defined card type.
    pub card_type: CardType,
    /// Owner-defined rarity.
    pub rarity: CardRarity,
    /// Character/pool metadata, including explicit unavailable state.
    pub character_or_pool: CardOptionalText,
    /// Base and supported variant values.
    pub variants: Vec<CardVariantInput>,
    /// Explicit upgrade paths and levels.
    pub upgrade_paths: Vec<CardUpgradePath>,
    /// Acquisition channels.
    pub acquisition: Vec<CardAcquisitionRule>,
    /// Optional unlock condition.
    pub unlock: Option<CardUnlockRule>,
}

/// Manifest provenance copied onto a typed card definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinitionProvenance {
    /// Supplying package identity and version, when known.
    pub origin: ContentOriginInput,
    /// Opaque override references, oldest first.
    pub override_chain: Vec<String>,
    /// Semantic revision from the content manifest.
    pub semantic_revision: String,
    /// Locale-qualified text revision from the content manifest.
    pub localized_text_revision: String,
}

/// A validated card variant in an immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardVariant {
    /// Stable variant ID scoped by its card definition.
    pub variant_id: String,
    /// Variant category.
    pub kind: CardVariantKind,
    /// Upgrade level when this is an upgrade variant.
    pub upgrade_level: Option<u16>,
    /// Localized title.
    pub title: CardTextValue,
    /// Localized description.
    pub description: CardTextValue,
    /// Structured cost.
    pub cost: CardCost,
    /// Targeting.
    pub targeting: CardTargeting,
    /// Keywords.
    pub keywords: Vec<String>,
    /// Typed effect parameters.
    pub effects: Vec<CardEffectParameter>,
    /// Structural modifiers.
    pub structural_modifiers: Vec<CardStructuralModifier>,
}

/// A validated card definition with all source-supported variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinition {
    /// Manifest-bound card identity.
    pub reference: ContentDefinitionReference,
    /// Manifest provenance.
    pub provenance: CardDefinitionProvenance,
    /// Owner-defined card type.
    pub card_type: CardType,
    /// Owner-defined rarity.
    pub rarity: CardRarity,
    /// Character/pool metadata.
    pub character_or_pool: CardOptionalText,
    /// Base and supported variants.
    pub variants: Vec<CardVariant>,
    /// Explicit upgrade paths.
    pub upgrade_paths: Vec<CardUpgradePath>,
    /// Acquisition channels.
    pub acquisition: Vec<CardAcquisitionRule>,
    /// Optional unlock condition.
    pub unlock: Option<CardUnlockRule>,
}

/// Exact immutable reference to one card variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardVariantReference {
    /// Manifest-bound card identity.
    pub card: ContentDefinitionReference,
    /// Variant ID scoped by the card definition.
    pub variant_id: String,
}

/// Immutable typed card catalog bound to one content manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardDefinitionCatalog {
    pub(crate) manifest: ContentCursorBinding,
    pub(crate) locale: String,
    pub(crate) definitions: BTreeMap<String, CardDefinition>,
}

impl CardDefinitionCatalog {
    /// Returns the exact manifest binding used by this catalog.
    #[must_use]
    pub fn manifest_binding(&self) -> &ContentCursorBinding {
        &self.manifest
    }

    /// Returns the exact locale used by localized card text.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Enumerates every card definition in stable ID order.
    pub fn definitions(&self) -> impl Iterator<Item = &CardDefinition> {
        self.definitions.values()
    }

    /// Performs an exact manifest-bound card definition lookup.
    pub fn get(
        &self,
        reference: &ContentDefinitionReference,
    ) -> Result<&CardDefinition, super::CardDefinitionError> {
        super::error::stale_if_mismatched(&self.manifest, reference)?;
        if reference.entity_kind != "card" {
            return Err(super::CardDefinitionError::NotFound);
        }
        self.definitions
            .get(&reference.namespaced_id)
            .ok_or(super::CardDefinitionError::NotFound)
    }

    /// Performs an exact manifest-bound variant lookup.
    pub fn variant(
        &self,
        reference: &CardVariantReference,
    ) -> Result<&CardVariant, super::CardDefinitionError> {
        let definition = self.get(&reference.card)?;
        definition
            .variants
            .iter()
            .find(|variant| variant.variant_id == reference.variant_id)
            .ok_or(super::CardDefinitionError::VariantNotFound)
    }

    pub(crate) fn from_parts(
        manifest: ContentCursorBinding,
        locale: String,
        definitions: BTreeMap<String, CardDefinition>,
    ) -> Self {
        Self {
            manifest,
            locale,
            definitions,
        }
    }
}
