// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::query::ContentFilterKind;
use crate::{ContentCursorBinding, ContentOriginInput};

/// Maximum bytes accepted for one owner-defined identity in this local producer.
pub const CONTENT_INDEX_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum entries requested for one local page.
pub const CONTENT_INDEX_MAX_PAGE_ITEMS: usize = 64;
/// Maximum UTF-8 bytes accepted for one searchable localized value.
pub const CONTENT_INDEX_MAX_TEXT_BYTES: usize = 64 * 1024;
/// Maximum aliases retained for one definition.
pub const CONTENT_INDEX_MAX_ALIAS_COUNT: usize = 64;
/// Maximum aggregate bytes returned by one exact definition lookup.
pub const CONTENT_INDEX_MAX_DEFINITION_BYTES: usize = 64 * 1024;
/// Maximum stable glossary term IDs attached to one content definition.
pub const CONTENT_INDEX_MAX_TERM_REFERENCES: usize = 64;

/// A caller-selected locale for a local query.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentQueryLocale(String);

impl ContentQueryLocale {
    /// Creates a locale token. Locale matching is exact and locale-independent.
    pub fn new(value: impl Into<String>) -> Result<Self, ContentIndexInputError> {
        let value = value.into();
        validate_identity(&value, "locale")?;
        Ok(Self(value))
    }

    /// Returns the exact locale token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Owner-defined rarity vocabulary carried as a typed value.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContentRarity(String);

impl ContentRarity {
    /// Creates a bounded owner-defined rarity value without guessing a game vocabulary.
    pub fn new(value: impl Into<String>) -> Result<Self, ContentIndexInputError> {
        let value = value.into();
        validate_identity(&value, "rarity")?;
        Ok(Self(value))
    }

    /// Returns the owner-defined rarity spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Explicit unlock state copied from the owner source.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ContentUnlockState {
    /// The source observed that the definition is unlocked.
    Unlocked,
    /// The source observed that the definition is locked.
    Locked,
    /// The source could not establish either state.
    Unknown,
}

/// Detail fields an adapter can expose through exact local lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDetailCapabilities {
    /// Whether an exact lookup can return the full typed definition.
    pub full_definition: bool,
    /// Whether a display name is observable/searchable.
    pub display_name: bool,
    /// Whether aliases are observable/searchable.
    pub aliases: bool,
    /// Whether a rendered description is observable/searchable.
    pub rendered_description: bool,
    /// Whether character or pool metadata is observable.
    pub character_or_pool: bool,
    /// Whether rarity metadata is observable.
    pub rarity: bool,
    /// Whether unlock state is observable.
    pub unlock_state: bool,
}

impl ContentDetailCapabilities {
    /// Creates a capability set with every field disabled.
    #[must_use]
    pub const fn none() -> Self {
        Self {
            full_definition: false,
            display_name: false,
            aliases: false,
            rendered_description: false,
            character_or_pool: false,
            rarity: false,
            unlock_state: false,
        }
    }

    /// Creates a capability set for a complete typed fixture.
    #[must_use]
    pub const fn full() -> Self {
        Self {
            full_definition: true,
            display_name: true,
            aliases: true,
            rendered_description: true,
            character_or_pool: true,
            rarity: true,
            unlock_state: true,
        }
    }
}

/// A source record keyed by an existing manifest definition ID.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentIndexDefinitionInput {
    /// Manifest family identity.
    pub entity_kind: String,
    /// Manifest namespaced definition ID.
    pub namespaced_id: String,
    /// Localized display name, when observable in the selected locale.
    pub display_name: Option<String>,
    /// Localized aliases, in owner-provided order.
    pub aliases: Vec<String>,
    /// Localized rendered description, when observable.
    pub rendered_description: Option<String>,
    /// Character or pool filter value, when defined for this family.
    pub character_or_pool: Option<String>,
    /// Owner-defined rarity, when defined for this family.
    pub rarity: Option<ContentRarity>,
    /// Explicit unlock state, without mutating profile state.
    pub unlock_state: ContentUnlockState,
    /// Stable glossary term IDs attached to this definition or its rendered text.
    ///
    /// The glossary producer resolves these IDs against its own immutable catalog. Keeping the
    /// IDs here links vocabulary without importing a transport or recursively expanding terms.
    pub term_references: Vec<String>,
}

impl ContentIndexDefinitionInput {
    /// Creates an input with no optional values and an unknown unlock state.
    #[must_use]
    pub fn new(entity_kind: impl Into<String>, namespaced_id: impl Into<String>) -> Self {
        Self {
            entity_kind: entity_kind.into(),
            namespaced_id: namespaced_id.into(),
            display_name: None,
            aliases: Vec::new(),
            rendered_description: None,
            character_or_pool: None,
            rarity: None,
            unlock_state: ContentUnlockState::Unknown,
            term_references: Vec::new(),
        }
    }
}

/// A complete source snapshot associated with one immutable manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentIndexSnapshot {
    /// Cursor/manifest witness that all records must match.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized field in the records.
    pub locale: String,
    /// Typed records copied from an owner-controlled source.
    pub definitions: Vec<ContentIndexDefinitionInput>,
}

/// Failure before an owned source snapshot was available.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentIndexSourceError {
    /// No active content source exists for the selected host/build.
    NoActiveContentSource,
    /// The source denied a read without exposing host details.
    AccessDenied,
    /// The source could not produce a bounded owned snapshot.
    Malformed,
}

impl std::fmt::Display for ContentIndexSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContentIndexSourceError {}

/// Owner-local source boundary for typed content metadata.
pub trait ContentIndexSource {
    /// Copies records without constructing playable objects or mutating profile progress.
    fn read_index(
        &self,
        manifest: &crate::ContentManifest,
    ) -> Result<ContentIndexSnapshot, ContentIndexSourceError>;
}

/// An opaque identity reference shared by list summaries and exact lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDefinitionReference {
    /// Manifest witness that owns the definition ID.
    pub manifest: ContentCursorBinding,
    /// Owner-defined family.
    pub entity_kind: String,
    /// Namespaced definition ID.
    pub namespaced_id: String,
}

/// Bounded summary returned by list and search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDefinitionSummary {
    /// Exact lookup reference.
    pub reference: ContentDefinitionReference,
    /// Manifest provenance.
    pub origin: ContentOriginInput,
    /// Semantic revision from the manifest.
    pub semantic_revision: String,
    /// Locale-qualified text revision from the manifest.
    pub localized_text_revision: String,
    /// Display name when the adapter observes it.
    pub display_name: Option<String>,
    /// Character or pool when defined.
    pub character_or_pool: Option<String>,
    /// Owner-defined rarity when defined.
    pub rarity: Option<ContentRarity>,
    /// Explicit unlock observation.
    pub unlock_state: ContentUnlockState,
    /// Fields recoverable through exact lookup.
    pub detail_capabilities: ContentDetailCapabilities,
    /// Stable glossary term IDs attached to this definition.
    pub term_references: Vec<String>,
}

/// Full typed definition returned by exact lookup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDefinition {
    /// Exact lookup reference.
    pub reference: ContentDefinitionReference,
    /// Manifest provenance.
    pub origin: ContentOriginInput,
    /// Override references copied from the manifest.
    pub override_chain: Vec<String>,
    /// Semantic revision from the manifest.
    pub semantic_revision: String,
    /// Locale-qualified text revision from the manifest.
    pub localized_text_revision: String,
    /// Display name when observed.
    pub display_name: Option<String>,
    /// Aliases when observed.
    pub aliases: Vec<String>,
    /// Rendered description when observed.
    pub rendered_description: Option<String>,
    /// Character or pool when defined.
    pub character_or_pool: Option<String>,
    /// Owner-defined rarity when defined.
    pub rarity: Option<ContentRarity>,
    /// Explicit unlock observation.
    pub unlock_state: ContentUnlockState,
    /// Detail fields the adapter can expose.
    pub detail_capabilities: ContentDetailCapabilities,
    /// Stable glossary term IDs attached to this definition.
    pub term_references: Vec<String>,
}

/// One family in the immutable index, including unsupported families.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentIndexFamily {
    /// Owner-defined family identity.
    pub entity_kind: String,
    /// Whether a typed adapter exists for this family.
    pub handled: bool,
    /// Number of definitions inventoried by the manifest.
    pub definition_count: usize,
    /// Filters and exact-detail capabilities available for this family.
    pub detail_capabilities: ContentDetailCapabilities,
    /// Family-specific filters accepted by the adapter.
    pub supported_filters: BTreeSet<ContentFilterKind>,
}

/// Errors returned while validating one source record before it enters the index.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentIndexInputError {
    /// Identity is empty, oversized, or contains disallowed characters.
    InvalidIdentity(&'static str),
}

impl std::fmt::Display for ContentIndexInputError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContentIndexInputError {}

pub(crate) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), ContentIndexInputError> {
    if value.is_empty()
        || value.len() > CONTENT_INDEX_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(ContentIndexInputError::InvalidIdentity(field));
    }
    Ok(())
}
