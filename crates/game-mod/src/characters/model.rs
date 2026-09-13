// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const CHARACTER_ENTITY_KIND: &str = "character";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const CHARACTER_PRODUCER_VERSION: &str = "game-character-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const CHARACTER_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const CHARACTER_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one character definition.
pub const CHARACTER_MAX_DEFINITION_BYTES: usize = 96 * 1024;
/// Maximum character definitions in one source snapshot.
pub const CHARACTER_MAX_DEFINITIONS: usize = 256;
/// Maximum loadout/mode variants on one character.
pub const CHARACTER_MAX_LOADOUTS: usize = 64;
/// Maximum content references in one loadout.
pub const CHARACTER_MAX_REFERENCES: usize = 512;
/// Maximum pool references in one loadout.
pub const CHARACTER_MAX_POOLS: usize = 128;
/// Maximum mechanic references in one loadout.
pub const CHARACTER_MAX_MECHANIC_REFERENCES: usize = 128;
/// Maximum resource entries in one starting configuration.
pub const CHARACTER_MAX_RESOURCES: usize = 64;
/// Maximum requirements on one character or loadout.
pub const CHARACTER_MAX_REQUIREMENTS: usize = 64;
/// Maximum unresolved formula inputs.
pub const CHARACTER_MAX_FORMULA_INPUTS: usize = 32;
/// Maximum entries returned by one bounded definition page.
pub const CHARACTER_MAX_PAGE_ITEMS: usize = 64;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized character value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static character definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterDefinitionReference {
    /// Catalog witness that owns this character identity.
    pub catalog: CharacterCatalogBinding,
    /// Namespaced content definition identity.
    pub character_id: String,
}

/// Generic content identity with a required positive quantity.
///
/// `entity_kind` remains owner-defined so this slice does not assume that every game uses one
/// card, relic, or resource vocabulary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterContentReference {
    /// Manifest family containing the referenced content.
    pub entity_kind: String,
    /// Namespaced manifest definition identity.
    pub namespaced_id: String,
    /// Number of starting copies or instances.
    pub quantity: u32,
}

/// A source-owned dynamic formula without an executable expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterFormula {
    /// Stable owner rule identity.
    pub rule_reference: String,
    /// Inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterUnavailableReason {
    /// The source supports the field but did not observe it.
    NotObserved,
    /// The supported host/build has no extractor for the field.
    Unsupported,
    /// The caller's source scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
    /// The field has no meaning for the selected configuration.
    NotApplicable,
}

/// A source field that distinguishes an observed empty value from an unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(CharacterUnavailableReason),
}

impl<T> CharacterField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> CharacterFieldStatus {
        match self {
            Self::Available(_) => CharacterFieldStatus::Available,
            Self::Unavailable(reason) => reason.status(),
        }
    }

    /// Returns the observed value, including an observed empty collection.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            Self::Unavailable(_) => None,
        }
    }

    /// Creates an explicit unavailable field value.
    #[must_use]
    pub const fn unavailable(reason: CharacterUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// Coarse availability status for a source-owned character field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterFieldStatus {
    /// The source observed the field, including a known empty value.
    Available,
    /// The field has no meaning for the selected configuration.
    NotApplicable,
    /// The field is supported but was not observed.
    NotObserved,
    /// No extractor is supported for the field.
    Unsupported,
    /// The source denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
}

impl CharacterUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> CharacterFieldStatus {
        match self {
            Self::NotObserved => CharacterFieldStatus::NotObserved,
            Self::Unsupported => CharacterFieldStatus::Unsupported,
            Self::Denied => CharacterFieldStatus::Denied,
            Self::Failed => CharacterFieldStatus::Failed,
            Self::Unknown => CharacterFieldStatus::Unknown,
            Self::NotApplicable => CharacterFieldStatus::NotApplicable,
        }
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(CharacterUnavailableReason),
}

impl CharacterText {
    /// Creates a bounded available localized text value.
    pub fn available(value: impl Into<String>) -> Result<Self, super::CharacterCatalogError> {
        let value = value.into();
        if value.is_empty() {
            return Err(super::CharacterCatalogError::InvalidInput("text"));
        }
        validate_text(&value, "text")?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: CharacterUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// Numeric starting value retained as fixed, formula-backed, or unavailable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterNumericValue {
    /// Value observed directly in the owner definition.
    Fixed(i64),
    /// Dynamic value linked to a non-executable owner rule.
    Formula(CharacterFormula),
    /// Value was not safely available.
    Unavailable(CharacterUnavailableReason),
}

impl CharacterFormula {
    /// Creates a bounded formula reference with explicit unresolved inputs.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, super::CharacterCatalogError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        validate_identity(&formula.rule_reference, "formula_rule")?;
        if formula.unresolved_inputs.len() > CHARACTER_MAX_FORMULA_INPUTS {
            return Err(super::CharacterCatalogError::InvalidInput("formula_inputs"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for input in &formula.unresolved_inputs {
            validate_identity(input, "formula_input")?;
            if !seen.insert(input.as_str()) {
                return Err(super::CharacterCatalogError::InvalidInput(
                    "duplicate_formula_input",
                ));
            }
        }
        Ok(formula)
    }
}

/// Owner-defined character origin/provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterOrigin {
    /// Source-defined origin kind, such as a package or registry.
    pub kind: String,
    /// Optional active package identity.
    pub package_id: Option<String>,
    /// Optional active package version.
    pub package_version: Option<String>,
}

/// Visibility policy for locked or unknown character definitions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterVisibilityScope {
    /// Only definitions currently observed as unlocked.
    Public,
    /// Public definitions plus locked references and their requirements.
    Reference,
    /// Explicit owner-authorized scope; unknown unlock state remains excluded.
    Owner,
}

/// Returns whether an identity is bounded and safe to expose.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::CharacterCatalogError> {
    if value.is_empty()
        || value.len() > CHARACTER_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::CharacterCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Returns whether a localized or owner-defined text value is bounded and safe to expose.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::CharacterCatalogError> {
    if value.len() > CHARACTER_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(super::CharacterCatalogError::InvalidInput(field));
    }
    Ok(())
}
