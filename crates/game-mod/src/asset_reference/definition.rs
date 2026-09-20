// SPDX-License-Identifier: MIT

//! Typed source-owned asset entries and the bound entries they become.

use super::{
    ASSET_MAX_FIELD_ROWS, AssetByteSize, AssetCatalogBinding, AssetField, AssetFieldAvailability,
    AssetFieldStatus, AssetFieldValue, AssetHandle, AssetMediaKind, AssetMediaProperties,
    AssetOrigin, AssetReferenceError, AssetRenditionDescriptor, AssetRenditionPayload,
    AssetRetrievalState, AssetRevision, AssetVisibility, AssetVisibilityScope, validate_identity,
};

/// Reference to a manifest definition one asset is linked to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetDefinitionReference {
    /// Manifest entity kind, such as `card`.
    pub entity_kind: String,
    /// Namespaced manifest identity.
    pub namespaced_id: String,
}

impl AssetDefinitionReference {
    /// Creates one bounded definition reference.
    pub fn new(entity_kind: &str, namespaced_id: &str) -> Result<Self, AssetReferenceError> {
        validate_identity(entity_kind, "definition_kind")?;
        validate_identity(namespaced_id, "definition_id")?;
        Ok(Self {
            entity_kind: entity_kind.to_owned(),
            namespaced_id: namespaced_id.to_owned(),
        })
    }
}

/// Typed source-owned asset entry before it is bound to a catalog witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetEntryInput {
    /// Opaque asset handle.
    pub handle: String,
    /// Content definition the asset is linked to.
    pub definition: AssetDefinitionReference,
    /// Media kind the source declares.
    pub media_kind: AssetMediaKind,
    /// Whether the asset is retrievable, metadata-only, unavailable or missing.
    pub retrieval_state: AssetRetrievalState,
    /// Media type, dimensions and duration, or their stated absence.
    pub media_properties: AssetFieldValue<AssetMediaProperties>,
    /// Stored and decoded byte sizes, or their stated absence.
    pub byte_size: AssetFieldValue<AssetByteSize>,
    /// Package and content-revision provenance.
    pub origin: AssetOrigin,
    /// Bounded rendition bytes, a metadata-only statement, or their stated absence.
    pub rendition: AssetFieldValue<AssetRenditionPayload>,
    /// Owner-defined visibility.
    pub visibility: AssetVisibility,
    /// Catalog generation at which this reference stops resolving.
    pub expires_at_generation: u64,
    /// Declared availability of every field this entry inventories.
    pub fields: Vec<AssetFieldAvailability>,
}

/// Immutable asset entry bound to one catalog witness.
///
/// This type carries a descriptor only: bytes never reach a lookup or listing surface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetEntry {
    /// Catalog witness that owns this asset identity.
    pub binding: AssetCatalogBinding,
    /// Opaque asset handle.
    pub handle: AssetHandle,
    /// Content definition the asset is linked to.
    pub definition: AssetDefinitionReference,
    /// Media kind the source declares.
    pub media_kind: AssetMediaKind,
    /// Whether the asset is retrievable, metadata-only, unavailable or missing.
    pub retrieval_state: AssetRetrievalState,
    /// Media type, dimensions and duration, or their stated absence.
    pub media_properties: AssetFieldValue<AssetMediaProperties>,
    /// Stored and decoded byte sizes, or their stated absence.
    pub byte_size: AssetFieldValue<AssetByteSize>,
    /// Package and content-revision provenance.
    pub origin: AssetOrigin,
    /// Metadata-only statement of rendition availability.
    pub rendition: AssetFieldValue<AssetRenditionDescriptor>,
    /// Owner-defined visibility.
    pub visibility: AssetVisibility,
    /// Catalog generation at which this reference stops resolving.
    pub expires_at_generation: u64,
    /// Declared availability of every field this entry inventories.
    pub fields: Vec<AssetFieldAvailability>,
}

impl AssetEntry {
    pub(crate) fn from_input(
        binding: &AssetCatalogBinding,
        handle: AssetHandle,
        input: AssetEntryInput,
        rendition: AssetFieldValue<AssetRenditionDescriptor>,
    ) -> Self {
        Self {
            binding: binding.clone(),
            handle,
            definition: input.definition,
            media_kind: input.media_kind,
            retrieval_state: input.retrieval_state,
            media_properties: input.media_properties,
            byte_size: input.byte_size,
            origin: input.origin,
            rendition,
            visibility: input.visibility,
            expires_at_generation: input.expires_at_generation,
            fields: input.fields,
        }
    }

    /// Returns the declared status of one field, when this entry states a row for it.
    #[must_use]
    pub fn field_status(&self, field: AssetField) -> Option<AssetFieldStatus> {
        self.fields
            .iter()
            .find(|row| row.field == field)
            .map(|row| row.status)
    }

    /// Returns whether every field this entry inventories is either projected or a declared failure.
    #[must_use]
    pub fn fields_are_declared(&self) -> bool {
        AssetField::all()
            .iter()
            .all(|field| self.field_status(*field).is_some())
    }
}

/// Exact request for one retained asset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetEntryReference {
    /// Catalog witness that owns this asset identity.
    pub catalog: AssetCatalogBinding,
    /// Opaque asset handle.
    pub handle: AssetHandle,
}

/// Bounded request for one asset's metadata.
#[derive(Debug, Eq, PartialEq)]
pub struct AssetDetailQuery {
    /// Exact asset whose metadata is requested.
    pub entry: AssetEntryReference,
    /// Requested visibility scope.
    pub scope: AssetVisibilityScope,
    /// Content revision the caller believes it is reading, when it states one.
    pub revision: Option<AssetRevision>,
}

/// Estimates the aggregate bytes retained for one entry, excluding rendition bytes.
pub(super) fn entry_bytes(input: &AssetEntryInput) -> usize {
    let mut total = input.handle.len()
        + input.definition.entity_kind.len()
        + input.definition.namespaced_id.len()
        + input.origin.package_id.len()
        + input.origin.package_version.as_ref().map_or(0, String::len)
        + input.origin.content_revision.len();
    if let Some(properties) = input.media_properties.value() {
        total += properties.media_type().len();
    }
    if let Some(payload) = input.rendition.value() {
        total += payload.media_type().map_or(0, str::len);
    }
    total
}

/// Validates the declared field-row inventory of one entry.
pub(super) fn validate_field_rows(input: &AssetEntryInput) -> Result<(), AssetReferenceError> {
    if input.fields.len() > ASSET_MAX_FIELD_ROWS {
        return Err(AssetReferenceError::InvalidInput("fields"));
    }
    for field in AssetField::all() {
        let rows = input.fields.iter().filter(|row| row.field == field).count();
        if rows == 0 {
            return Err(AssetReferenceError::MissingFieldCoverage(field.name()));
        }
        if rows > 1 {
            return Err(AssetReferenceError::DuplicateFieldCoverage(field.name()));
        }
    }
    for row in &input.fields {
        if row.status != carried_status(input, row.field) {
            return Err(AssetReferenceError::InconsistentField(row.field.name()));
        }
    }
    Ok(())
}

/// Returns the status the entry's own carrier states for one field.
fn carried_status(input: &AssetEntryInput, field: AssetField) -> AssetFieldStatus {
    match field {
        AssetField::MediaProperties => input.media_properties.status(),
        AssetField::ByteSize => input.byte_size.status(),
        AssetField::Rendition => input.rendition.status(),
        AssetField::Handle
        | AssetField::Definition
        | AssetField::MediaKind
        | AssetField::RetrievalState
        | AssetField::Origin => AssetFieldStatus::Available,
    }
}
