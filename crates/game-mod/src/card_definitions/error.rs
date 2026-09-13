// SPDX-License-Identifier: MIT

use crate::ContentDefinitionReference;

/// Failure before a bounded owned card snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardDefinitionSourceError {
    /// No supported card registry is active for the selected host/build.
    NoActiveSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for CardDefinitionSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CardDefinitionSourceError {}

/// Validation failures for one source-owned card record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardDefinitionInputError {
    /// An identity is empty, oversized, or contains controls/disallowed punctuation.
    InvalidIdentity(&'static str),
    /// A localized or owner-defined text value is oversized or contains controls.
    InvalidText,
    /// A bounded collection exceeds its local limit.
    CollectionTooLarge(&'static str),
    /// The same identity occurs twice in one bounded collection.
    DuplicateIdentity(&'static str),
    /// A required base variant is absent or more than one base is supplied.
    InvalidBaseVariant,
    /// An upgrade variant has an invalid level or path relationship.
    InvalidUpgradeVariant,
    /// An upgrade path refers to an unknown or non-upgrade variant.
    InvalidUpgradePath,
    /// Upgrade levels in one path are reversed or duplicated.
    NonIncreasingUpgradeLevel,
    /// A dynamic formula has no rule reference or too many unresolved inputs.
    InvalidFormula,
}

impl std::fmt::Display for CardDefinitionInputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CardDefinitionInputError {}

/// Sanitized producer, lookup, and comparison failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardDefinitionError {
    /// The source failed before an owned snapshot was available.
    Source(CardDefinitionSourceError),
    /// The snapshot names another content manifest.
    ManifestMismatch,
    /// The snapshot uses another locale than the manifest.
    LocaleMismatch,
    /// The manifest does not inventory a card family.
    NoCardFamily,
    /// The manifest inventories cards but no typed card adapter handles them.
    UnsupportedCardFamily,
    /// The source omitted a card present in the manifest.
    MissingDefinition { namespaced_id: String },
    /// The source returned an ID absent from the manifest card family.
    UnknownDefinition { namespaced_id: String },
    /// The source repeated a card ID.
    DuplicateDefinition,
    /// A card record failed bounded validation.
    InvalidInput(CardDefinitionInputError),
    /// A caller supplied a reference from another manifest.
    StaleReference,
    /// A card reference is not in this catalog.
    NotFound,
    /// A variant reference names a card from another definition.
    VariantDefinitionMismatch,
    /// A variant ID is absent from the selected card.
    VariantNotFound,
}

impl std::fmt::Display for CardDefinitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CardDefinitionError {}

impl From<CardDefinitionInputError> for CardDefinitionError {
    fn from(error: CardDefinitionInputError) -> Self {
        Self::InvalidInput(error)
    }
}

/// Returns a stale-reference error without exposing the referenced identity.
pub(crate) fn stale_if_mismatched(
    expected: &crate::ContentCursorBinding,
    reference: &ContentDefinitionReference,
) -> Result<(), CardDefinitionError> {
    if reference.manifest != *expected {
        return Err(CardDefinitionError::StaleReference);
    }
    Ok(())
}
