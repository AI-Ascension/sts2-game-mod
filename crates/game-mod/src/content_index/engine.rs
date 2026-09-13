// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::ContentKindAdapterRegistry;
use super::helpers::definition_bytes;
use super::{
    ContentDefinition, ContentDefinitionReference, ContentIndexError, ContentIndexFamily,
    ContentQueryScope, ContentReferenceVisibilityPolicy, ContentUnlockState,
};
use crate::ContentCursorBinding;

/// Immutable owner-local index keyed by one content manifest and namespaced definition IDs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentIndex {
    pub(crate) manifest: ContentCursorBinding,
    pub(crate) locale: String,
    pub(crate) registry: ContentKindAdapterRegistry,
    pub(crate) locked_visibility: ContentReferenceVisibilityPolicy,
    pub(crate) families: Vec<ContentIndexFamily>,
    pub(crate) definitions: BTreeMap<(String, String), ContentDefinition>,
}

impl ContentIndex {
    pub(crate) fn from_parts(
        manifest: ContentCursorBinding,
        locale: String,
        registry: ContentKindAdapterRegistry,
        locked_visibility: ContentReferenceVisibilityPolicy,
        families: Vec<ContentIndexFamily>,
        definitions: BTreeMap<(String, String), ContentDefinition>,
    ) -> Self {
        Self {
            manifest,
            locale,
            registry,
            locked_visibility,
            families,
            definitions,
        }
    }

    /// Returns the exact immutable manifest witness used by this index.
    #[must_use]
    pub fn manifest_binding(&self) -> &ContentCursorBinding {
        &self.manifest
    }

    /// Returns the exact locale used by all localized source values.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Returns every manifest family, including explicitly unsupported entries.
    #[must_use]
    pub fn families(&self) -> &[ContentIndexFamily] {
        &self.families
    }

    /// Returns every immutable definition in deterministic family/identity order.
    ///
    /// The records are the same source-owned values used by list and exact lookup. This
    /// accessor is intentionally read-only so another source-only projection can compose
    /// bounded cross-reference metadata without reconstructing or mutating content objects.
    pub fn definitions(&self) -> impl Iterator<Item = &ContentDefinition> {
        self.definitions.values()
    }

    /// Creates a cursor-owning reader over this immutable index.
    #[must_use]
    pub fn reader(&self) -> super::reader::ContentIndexReader {
        super::reader::ContentIndexReader::new(self.clone())
    }

    /// Performs an exact lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &ContentDefinitionReference,
        scope: ContentQueryScope,
    ) -> Result<ContentDefinition, ContentIndexError> {
        self.get_internal(reference, scope)
    }

    pub(crate) fn get_internal(
        &self,
        reference: &ContentDefinitionReference,
        scope: ContentQueryScope,
    ) -> Result<ContentDefinition, ContentIndexError> {
        if reference.manifest != self.manifest {
            return Err(ContentIndexError::StaleReference);
        }
        let Some(adapter) = self.registry.get(&reference.entity_kind) else {
            return Err(ContentIndexError::UnknownKind);
        };
        if !adapter.handled {
            return Err(ContentIndexError::UnsupportedKind);
        }
        let Some(definition) = self.definitions.get(&(
            reference.entity_kind.clone(),
            reference.namespaced_id.clone(),
        )) else {
            return Err(ContentIndexError::NotFound);
        };
        if !self.is_visible(definition, scope) {
            return Err(ContentIndexError::ExcludedByScope);
        }
        if !definition.detail_capabilities.full_definition {
            return Err(ContentIndexError::DetailUnavailable);
        }
        definition_bytes(definition)?;
        Ok(definition.clone())
    }

    pub(crate) fn is_visible(
        &self,
        definition: &ContentDefinition,
        scope: ContentQueryScope,
    ) -> bool {
        match definition.unlock_state {
            ContentUnlockState::Unlocked => true,
            ContentUnlockState::Locked => {
                matches!(
                    (scope, self.locked_visibility),
                    (
                        ContentQueryScope::Reference,
                        ContentReferenceVisibilityPolicy::AllowLockedReferences
                    )
                )
            }
            ContentUnlockState::Unknown => false,
        }
    }
}
