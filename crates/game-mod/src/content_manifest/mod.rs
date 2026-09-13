// SPDX-License-Identifier: MIT

mod producer;
mod revisions;
mod validation;

pub use producer::ContentManifestProducer;

/// The owner-local producer contract for a source-only content manifest.
pub const CONTENT_MANIFEST_PRODUCER_VERSION: &str = "game-content-manifest-producer-v1";
/// Maximum length of an identity token carried by the producer contract.
pub const CONTENT_MANIFEST_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum length of the non-localized semantic input accepted from a source.
pub const CONTENT_MANIFEST_MAX_SEMANTIC_BYTES: usize = 16 * 1024;
/// Maximum length of one localized value accepted from a source.
pub const CONTENT_MANIFEST_MAX_TEXT_BYTES: usize = 64 * 1024;

/// Errors reported by the owner-controlled catalog access seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentSourceError {
    /// The host catalog is not currently available.
    Unavailable,
    /// The source declined the read without exposing host details.
    AccessDenied,
    /// The source could not produce an owned catalog snapshot.
    Malformed,
}

impl std::fmt::Display for ContentSourceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContentSourceError {}

/// A package in the active content order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentPackageInput {
    /// Stable package identity supplied by the owner registry.
    pub package_id: String,
    /// Package version when the source supplies one; `None` is explicitly unknown.
    pub package_version: Option<String>,
    /// Owner-defined active package order. Lower values load first.
    pub order: u32,
}

/// Definition provenance supplied by the owner registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentOriginInput {
    /// The active package supplying this definition, when known.
    pub package_id: Option<String>,
    /// The supplying package version, when known.
    pub package_version: Option<String>,
}

/// One definition copied from an owner-controlled catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDefinitionInput {
    /// Stable owner-defined entity family.
    pub entity_kind: String,
    /// Opaque ID scoped by `entity_kind`.
    pub namespaced_id: String,
    /// Canonical non-localized semantic inputs used for revisioning.
    pub semantic_inputs: String,
    /// Rendered text for the requested locale, when supplied by the source.
    pub localized_text: Option<String>,
    /// Package and override provenance.
    pub origin: ContentOriginInput,
    /// Opaque references to the definition's override chain, oldest first.
    pub override_chain: Vec<String>,
}

/// An owned catalog read captured by a supported source.
///
/// A source is responsible for reading owner metadata without instantiating playable objects or
/// mutating unlocks. The two generation witnesses let the producer reject a catalog that changed
/// during extraction instead of publishing a mixed manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentCatalogSnapshot {
    /// Generation observed before the owner catalog was copied.
    pub generation_before: u64,
    /// Generation observed after the owner catalog was copied.
    pub generation_after: u64,
    /// Exact game/build identity supplied by the owner.
    pub game_build: String,
    /// Locale used for localized values in this snapshot.
    pub locale: String,
    /// Active packages in owner order.
    pub packages: Vec<ContentPackageInput>,
    /// Full available model-registry family list.
    pub available_entity_kinds: Vec<String>,
    /// Definitions discovered through supported owner access.
    pub definitions: Vec<ContentDefinitionInput>,
}

/// The only source boundary needed by the producer.
///
/// Implementations must return owned values and must not use this contract to expose install
/// paths, assemblies, account identifiers, save data, or raw host exceptions.
pub trait ContentCatalogSource {
    /// Copies one owner catalog without constructing playable game objects.
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError>;
}

/// A producer-owned package record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentPackage {
    /// Stable package identity.
    pub package_id: String,
    /// Package version, or `None` when the owner supplied no version.
    pub package_version: Option<String>,
    /// Active package order.
    pub order: u32,
}

/// A family inventory entry, including families the producer cannot handle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentFamily {
    /// Owner-defined family identity.
    pub entity_kind: String,
    /// Whether this producer knows how to project the family.
    pub handled: bool,
    /// Number of definitions present in the owner registry for this family.
    pub definition_count: usize,
}

/// A producer-owned definition with independently scoped revisions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDefinition {
    /// Stable owner-defined entity family.
    pub entity_kind: String,
    /// Opaque ID scoped by `entity_kind`.
    pub namespaced_id: String,
    /// Whether this producer knows how to project the family.
    pub handled: bool,
    /// Package and override provenance.
    pub origin: ContentOriginInput,
    /// Opaque references to the definition's override chain, oldest first.
    pub override_chain: Vec<String>,
    /// Revision of semantic inputs and provenance, independent of locale text.
    pub semantic_revision: String,
    /// Revision of rendered text for the manifest locale.
    pub localized_text_revision: String,
}

/// The immutable source-only manifest produced from one coherent catalog read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentManifest {
    /// Exact game/build identity.
    pub game_build: String,
    /// Adapter compatibility selected by the producer owner.
    pub adapter_compatibility: String,
    /// Catalog generation used for invalidation, not content identity.
    pub catalog_generation: u64,
    /// Locale of the localized revision.
    pub locale: String,
    /// Active content packages in load order.
    pub packages: Vec<ContentPackage>,
    /// Per-family coverage and counts.
    pub families: Vec<ContentFamily>,
    /// Owned definitions, deterministically ordered by kind and ID.
    pub definitions: Vec<ContentDefinition>,
    /// Semantic content-set revision, independent of locale text.
    pub content_set_revision: String,
    /// Locale-qualified rendered-text revision.
    pub localized_text_revision: String,
    /// Inventory revision covering package order, family coverage, and counts.
    pub inventory_revision: String,
}

/// The owner-local invalidation witness for a query cursor or cached page.
///
/// This is intentionally not a transport cursor. Protocol-owned query binding must add its own
/// query, scope, and snapshot fields after the shared contract is accepted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentCursorBinding {
    /// Catalog generation observed by the producer.
    pub catalog_generation: u64,
    /// Adapter compatibility used to create the manifest.
    pub adapter_compatibility: String,
    /// Semantic content revision.
    pub content_set_revision: String,
    /// Locale-qualified text revision.
    pub localized_text_revision: String,
    /// Inventory revision.
    pub inventory_revision: String,
}

/// Producer validation and extraction failures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentManifestError {
    /// Source access failed before an owned snapshot was available.
    Source(ContentSourceError),
    /// The owner catalog changed during extraction.
    CatalogChanged { before: u64, after: u64 },
    /// A required identity field is malformed.
    InvalidIdentity(&'static str),
    /// A non-localized semantic input is too large or contains controls.
    InvalidSemanticInput,
    /// A localized value is too large or contains controls.
    InvalidLocalizedText,
    /// A package version is malformed.
    InvalidPackageVersion,
    /// Two active packages share an ID or order.
    DuplicatePackage,
    /// A package order is not unique.
    InvalidPackageOrder,
    /// A registry family is duplicated.
    DuplicateEntityKind,
    /// A definition family is absent from the owner registry.
    UnknownEntityKind,
    /// A definition ID is duplicated in one family.
    DuplicateDefinition,
    /// A known provenance package is not active.
    UnknownOriginPackage,
    /// A definition provenance version disagrees with its active package version.
    OriginPackageVersionMismatch,
    /// A definition override chain repeats a reference.
    DuplicateOverrideReference,
}

impl std::fmt::Display for ContentManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContentManifestError {}

impl ContentManifest {
    /// Returns the invalidation binding for this manifest.
    #[must_use]
    pub fn cursor_binding(&self) -> ContentCursorBinding {
        ContentCursorBinding {
            catalog_generation: self.catalog_generation,
            adapter_compatibility: self.adapter_compatibility.clone(),
            content_set_revision: self.content_set_revision.clone(),
            localized_text_revision: self.localized_text_revision.clone(),
            inventory_revision: self.inventory_revision.clone(),
        }
    }

    /// Returns whether an owner-local cursor/cache witness still names this manifest.
    #[must_use]
    pub fn accepts_cursor(&self, binding: &ContentCursorBinding) -> bool {
        self.cursor_binding() == *binding
    }
}

fn validate_identity(value: &str, field: &'static str) -> Result<(), ContentManifestError> {
    if value.is_empty()
        || value.len() > CONTENT_MANIFEST_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(ContentManifestError::InvalidIdentity(field));
    }
    Ok(())
}
