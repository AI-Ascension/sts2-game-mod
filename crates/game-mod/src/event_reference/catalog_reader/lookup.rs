// SPDX-License-Identifier: MIT

use super::super::{
    EventCatalogError, EventDefinition, EventNarrativePage, EventOption, EventPageReference,
    EventVisibilityScope,
};
use super::reader::EventCatalogReader;
use super::{project_definition, project_option, visible_option, visible_page};

impl EventCatalogReader {
    /// Performs one exact event lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &super::super::EventDefinitionReference,
        scope: EventVisibilityScope,
    ) -> Result<EventDefinition, EventCatalogError> {
        self.validate_family()?;
        let definition = self.definition_for_reference(reference, scope)?;
        Ok(project_definition(definition, scope))
    }

    /// Performs one exact narrative-page lookup.
    pub fn get_page(
        &self,
        reference: &EventPageReference,
        scope: EventVisibilityScope,
    ) -> Result<EventNarrativePage, EventCatalogError> {
        self.validate_family()?;
        let definition =
            self.definition_for_identity(&reference.catalog, &reference.event_id, scope)?;
        if let Some(page) = definition
            .pages
            .iter()
            .find(|page| page.page_id == reference.page_id)
        {
            if visible_page(page, scope) {
                return Ok(page.clone());
            }
            return Err(EventCatalogError::ExcludedByScope);
        }
        Err(EventCatalogError::NotFound)
    }

    /// Performs one exact option lookup, withholding hidden nested records.
    pub fn get_option(
        &self,
        reference: &super::super::EventOptionReference,
        scope: EventVisibilityScope,
    ) -> Result<EventOption, EventCatalogError> {
        self.validate_family()?;
        let definition =
            self.definition_for_identity(&reference.catalog, &reference.event_id, scope)?;
        if let Some(option) = definition
            .options
            .iter()
            .find(|option| option.reference.option_id == reference.option_id)
        {
            if visible_option(option, scope) {
                return Ok(project_option(option, scope));
            }
            return Err(EventCatalogError::ExcludedByScope);
        }
        Err(EventCatalogError::NotFound)
    }
}
