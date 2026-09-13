// SPDX-License-Identifier: MIT

//! Source-only typed card definition extraction.
//!
//! This module composes the existing [`crate::ContentManifest`] identity with card-owned
//! values. It is deliberately not a second content index and does not define a route, wire
//! schema, native capability, or live-card mutation. A future transport adapter may map these
//! owner-local values only after its versioned contract is accepted.

mod compare;
mod error;
mod model;
mod producer;
mod rules;
mod validation;
mod variants;

pub use compare::{CardFieldChange, CardVariantComparison};
pub use error::{CardDefinitionError, CardDefinitionInputError, CardDefinitionSourceError};
pub use model::{
    CardCost, CardCostComponent, CardCostUnit, CardEffectParameter, CardEffectValue, CardFormula,
    CardNumericValue, CardOptionalText, CardRarity, CardStructuralModifier, CardTargeting,
    CardTextValue, CardType, CardUnavailableReason,
};
pub use producer::{CardDefinitionProducer, CardDefinitionSnapshot, CardDefinitionSource};
pub use rules::{
    CardAcquisitionKind, CardAcquisitionRule, CardRuleCondition, CardRuleProvenance, CardUnlockRule,
};
pub use variants::{
    CardDefinition, CardDefinitionCatalog, CardDefinitionInput, CardDefinitionProvenance,
    CardUpgradePath, CardVariant, CardVariantInput, CardVariantKind, CardVariantReference,
};

/// Owner-local producer identity; this is not a protocol version.
pub const CARD_DEFINITION_PRODUCER_VERSION: &str = "game-card-definition-producer-v1";

/// Maximum card definitions accepted from one source snapshot.
pub const CARD_DEFINITION_MAX_DEFINITIONS: usize = 16 * 1024;
/// Maximum variants, including the base variant, retained for one card.
pub const CARD_DEFINITION_MAX_VARIANTS: usize = 64;
/// Maximum upgrade paths retained for one card.
pub const CARD_DEFINITION_MAX_UPGRADE_PATHS: usize = 16;
/// Maximum variant IDs in one upgrade path.
pub const CARD_DEFINITION_MAX_UPGRADE_LEVELS: usize = 32;
/// Maximum effect parameters on one variant.
pub const CARD_DEFINITION_MAX_EFFECTS: usize = 64;
/// Maximum keywords on one variant.
pub const CARD_DEFINITION_MAX_KEYWORDS: usize = 64;
/// Maximum structural modifiers on one variant.
pub const CARD_DEFINITION_MAX_MODIFIERS: usize = 64;
/// Maximum acquisition rules on one card.
pub const CARD_DEFINITION_MAX_ACQUISITION_RULES: usize = 64;
/// Maximum unresolved inputs carried by one dynamic formula.
pub const CARD_DEFINITION_MAX_FORMULA_INPUTS: usize = 16;
/// Maximum bytes accepted for one local text value.
pub const CARD_DEFINITION_MAX_TEXT_BYTES: usize = 64 * 1024;
/// Maximum bytes accepted for one owner-defined identity.
pub const CARD_DEFINITION_MAX_IDENTITY_BYTES: usize = 256;
