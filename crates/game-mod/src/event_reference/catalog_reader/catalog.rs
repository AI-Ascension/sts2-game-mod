// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    EventCatalogBinding, EventCatalogError, EventDefinition, EventDefinitionReference,
    EventFamilyCoverage, EventNarrativePage, EventOption, EventOptionReference, EventPageReference,
    EventVisibilityScope,
};
use super::reader::EventCatalogReader;

/// Immutable event definitions keyed by namespaced event identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventCatalog {
    pub(super) binding: EventCatalogBinding,
    pub(super) family: EventFamilyCoverage,
    pub(super) definitions: BTreeMap<String, EventDefinition>,
}

impl EventCatalog {
    pub(crate) fn from_parts(
        binding: EventCatalogBinding,
        family: EventFamilyCoverage,
        definitions: BTreeMap<String, EventDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &EventCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the event family.
    #[must_use]
    pub fn family(&self) -> &EventFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with independent bounded cursors.
    #[must_use]
    pub fn reader(&self) -> EventCatalogReader {
        EventCatalogReader::new(self.clone())
    }

    /// Performs one exact event lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &EventDefinitionReference,
        scope: EventVisibilityScope,
    ) -> Result<EventDefinition, EventCatalogError> {
        self.reader().get(reference, scope)
    }

    /// Performs one exact narrative-page lookup.
    pub fn get_page(
        &self,
        reference: &EventPageReference,
        scope: EventVisibilityScope,
    ) -> Result<EventNarrativePage, EventCatalogError> {
        self.reader().get_page(reference, scope)
    }

    /// Performs one exact option lookup.
    pub fn get_option(
        &self,
        reference: &EventOptionReference,
        scope: EventVisibilityScope,
    ) -> Result<EventOption, EventCatalogError> {
        self.reader().get_option(reference, scope)
    }
}
