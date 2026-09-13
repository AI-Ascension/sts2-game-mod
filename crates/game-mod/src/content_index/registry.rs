// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};

use crate::ContentManifest;

use super::errors::ContentIndexError;
use super::model::{ContentDetailCapabilities, ContentIndexInputError, validate_identity};
use super::query::ContentFilterKind;

/// One family adapter declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentKindAdapter {
    /// Owner-defined family identity.
    pub entity_kind: String,
    /// Whether typed source records can be exposed for this family.
    pub handled: bool,
    /// Adapter-specific filters supported for this family.
    pub supported_filters: BTreeSet<ContentFilterKind>,
    /// Exact-detail and searchable-field capabilities.
    pub detail_capabilities: ContentDetailCapabilities,
}

impl ContentKindAdapter {
    /// Declares a handled family with explicit filter and detail capabilities.
    pub fn supported(
        entity_kind: impl Into<String>,
        supported_filters: impl IntoIterator<Item = ContentFilterKind>,
        detail_capabilities: ContentDetailCapabilities,
    ) -> Result<Self, ContentIndexInputError> {
        let entity_kind = entity_kind.into();
        validate_identity(&entity_kind, "entity_kind")?;
        let mut supported_filters = supported_filters.into_iter().collect::<BTreeSet<_>>();
        supported_filters.insert(ContentFilterKind::Kind);
        supported_filters.insert(ContentFilterKind::OriginPackage);
        Ok(Self {
            entity_kind,
            handled: true,
            supported_filters,
            detail_capabilities,
        })
    }

    /// Declares an explicitly unsupported family entry.
    pub fn unsupported(entity_kind: impl Into<String>) -> Result<Self, ContentIndexInputError> {
        let entity_kind = entity_kind.into();
        validate_identity(&entity_kind, "entity_kind")?;
        Ok(Self {
            entity_kind,
            handled: false,
            supported_filters: BTreeSet::new(),
            detail_capabilities: ContentDetailCapabilities::none(),
        })
    }
}

/// Registry covering every family in one content manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentKindAdapterRegistry {
    adapters: BTreeMap<String, ContentKindAdapter>,
}

impl ContentKindAdapterRegistry {
    /// Builds a registry and inserts explicit unsupported entries for omitted families.
    ///
    /// A supplied adapter for a family absent from the manifest is rejected. Omitted families
    /// are retained as unsupported rather than silently dropped from the inventory.
    pub fn new(
        manifest: &ContentManifest,
        adapters: impl IntoIterator<Item = ContentKindAdapter>,
    ) -> Result<Self, ContentIndexError> {
        let manifest_kinds = manifest
            .families
            .iter()
            .map(|family| family.entity_kind.as_str())
            .collect::<BTreeSet<_>>();
        let mut registry = BTreeMap::new();
        for adapter in adapters {
            if !manifest_kinds.contains(adapter.entity_kind.as_str()) {
                return Err(ContentIndexError::AdapterForUnknownKind);
            }
            if registry
                .insert(adapter.entity_kind.clone(), adapter)
                .is_some()
            {
                return Err(ContentIndexError::DuplicateAdapter);
            }
        }
        for family in &manifest.families {
            registry
                .entry(family.entity_kind.clone())
                .or_insert_with(|| ContentKindAdapter {
                    entity_kind: family.entity_kind.clone(),
                    handled: false,
                    supported_filters: BTreeSet::new(),
                    detail_capabilities: ContentDetailCapabilities::none(),
                });
        }
        Ok(Self { adapters: registry })
    }

    /// Returns the adapter for a manifest family, if present.
    #[must_use]
    pub fn get(&self, entity_kind: &str) -> Option<&ContentKindAdapter> {
        self.adapters.get(entity_kind)
    }

    /// Returns every family entry in deterministic identity order.
    pub fn entries(&self) -> impl Iterator<Item = &ContentKindAdapter> {
        self.adapters.values()
    }

    pub(crate) fn validate_filter(
        &self,
        selected_kind: Option<&str>,
        filter: ContentFilterKind,
    ) -> Result<(), ContentIndexError> {
        if matches!(
            filter,
            ContentFilterKind::Kind | ContentFilterKind::OriginPackage
        ) {
            return Ok(());
        }

        if let Some(entity_kind) = selected_kind {
            let adapter = self
                .get(entity_kind)
                .ok_or(ContentIndexError::UnknownKind)?;
            if !adapter.handled {
                return Err(ContentIndexError::UnsupportedKind);
            }
            if !adapter.supported_filters.contains(&filter) {
                return Err(ContentIndexError::UnsupportedFilter(filter));
            }
            return Ok(());
        }

        if self
            .entries()
            .any(|adapter| !adapter.handled || !adapter.supported_filters.contains(&filter))
        {
            return Err(ContentIndexError::UnsupportedFilter(filter));
        }
        Ok(())
    }

    pub(crate) fn validate_kind(
        &self,
        selected_kind: Option<&str>,
    ) -> Result<(), ContentIndexError> {
        let Some(entity_kind) = selected_kind else {
            return Ok(());
        };
        match self.get(entity_kind) {
            Some(adapter) if adapter.handled => Ok(()),
            Some(_) => Err(ContentIndexError::UnsupportedKind),
            None => Err(ContentIndexError::UnknownKind),
        }
    }
}
