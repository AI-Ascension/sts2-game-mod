// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    cursor::RestCursors,
    definition::{RestSiteCatalog, RestSiteDefinition},
    error::RestSiteError,
    model::{
        REST_MAX_PAGE_ITEMS, RestFamilyState, RestOptionReference, RestOptionSetReference,
        RestSiteDefinitionReference, RestVisibilityScope,
    },
    option::{RestOption, RestOptionState},
    page::{
        ContinuationScope, RestOptionListQuery, RestOptionPage, RestSiteDefinitionPage,
        RestSiteListQuery,
    },
    projection::{
        map_status, option_summary, project_definition, site_summary, visibility_allowed,
        visible_option,
    },
};

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`RestSiteCatalog::reader`] for an independent reader instead. Every method
/// here is a bounded read: none rests, heals, smiths, upgrades, transforms, or mends anything.
#[derive(Debug)]
pub struct RestSiteReader<'a> {
    catalog: &'a RestSiteCatalog,
    cursors: RestCursors,
    scope: RestVisibilityScope,
    continuations: Arc<ContinuationScope>,
}

impl<'a> RestSiteReader<'a> {
    pub(super) fn new(catalog: &'a RestSiteCatalog, scope: RestVisibilityScope) -> Self {
        Self {
            catalog,
            cursors: RestCursors::new(),
            scope,
            continuations: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub const fn catalog(&self) -> &RestSiteCatalog {
        self.catalog
    }

    /// Returns the default visibility scope this reader was created with.
    #[must_use]
    pub const fn scope(&self) -> RestVisibilityScope {
        self.scope
    }

    /// Lists visible rest-site definitions in stable site-identity order.
    pub fn list(
        &mut self,
        query: &RestSiteListQuery,
    ) -> Result<RestSiteDefinitionPage, RestSiteError> {
        if query.locale != self.catalog.binding.locale {
            return Err(RestSiteError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > REST_MAX_PAGE_ITEMS {
            return Err(RestSiteError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self
            .cursors
            .site_start(&self.catalog.binding, &self.continuations, query)?;
        let entries = self
            .catalog
            .definitions()
            .values()
            .filter(|definition| visibility_allowed(definition.visibility, query.scope))
            .map(|definition| site_summary(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries
            .get(start..end)
            .map_or_else(Vec::new, |page| page.to_vec());
        let continuation = self.cursors.next_site(
            &self.catalog.binding,
            &self.continuations,
            query,
            end,
            total,
        );
        Ok(RestSiteDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible rest options of one exact rest-site definition.
    pub fn list_options(
        &mut self,
        query: &RestOptionListQuery,
    ) -> Result<RestOptionPage, RestSiteError> {
        if query.limit == 0 || query.limit > REST_MAX_PAGE_ITEMS {
            return Err(RestSiteError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self
            .cursors
            .option_start(&self.catalog.binding, &self.continuations, query)?;
        let definition = self.site(&query.site, query.scope)?;
        let options_status = map_status(definition.options.values(), query.scope, visible_option);
        let entries_all = definition
            .options
            .values()
            .filter(|option| visible_option(option, query.scope))
            .map(option_summary)
            .collect::<Vec<_>>();
        let total = entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries_all
            .get(start..end)
            .map_or_else(Vec::new, |page| page.to_vec());
        let continuation = self.cursors.next_option(
            &self.catalog.binding,
            &self.continuations,
            query,
            end,
            total,
        );
        Ok(RestOptionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            options_status,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact rest-site definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &RestSiteDefinitionReference,
        scope: RestVisibilityScope,
    ) -> Result<RestSiteDefinition, RestSiteError> {
        self.validate_family()?;
        Ok(project_definition(self.site(reference, scope)?, scope))
    }

    /// Performs one exact rest-option lookup under an explicit visibility scope.
    pub fn get_option(
        &self,
        reference: &RestOptionReference,
        scope: RestVisibilityScope,
    ) -> Result<RestOption, RestSiteError> {
        self.validate_family()?;
        Ok(self.option_for_reference(reference, scope)?.clone())
    }

    /// Returns the availability a reference describes, or refuses an earlier option set.
    ///
    /// The host renders a rest menu for one option set; a reference bound to an earlier generation
    /// no longer describes the current site, so it is refused rather than answered with the newer
    /// options.
    pub fn option_availability(
        &self,
        reference: &RestOptionSetReference,
        scope: RestVisibilityScope,
    ) -> Result<RestOptionState, RestSiteError> {
        self.validate_family()?;
        let option = self.option_for_reference(&reference.option, scope)?;
        let definition =
            self.site_of(&reference.option.site_id, &reference.option.catalog, scope)?;
        if reference.option_set_generation != definition.option_set_generation {
            return Err(RestSiteError::StaleOptionSetReference {
                option_id: reference.option.option_id.clone(),
                referenced: reference.option_set_generation,
                current: definition.option_set_generation,
            });
        }
        Ok(option.availability.state)
    }

    fn validate_family(&self) -> Result<(), RestSiteError> {
        match self.catalog.family.state {
            RestFamilyState::Handled => Ok(()),
            RestFamilyState::Unsupported => Err(RestSiteError::UnsupportedFamily),
            RestFamilyState::Unavailable => Err(RestSiteError::UnavailableFamily),
        }
    }

    fn site(
        &self,
        reference: &RestSiteDefinitionReference,
        scope: RestVisibilityScope,
    ) -> Result<&'a RestSiteDefinition, RestSiteError> {
        self.site_of(&reference.site_id, &reference.catalog, scope)
    }

    fn site_of(
        &self,
        site_id: &str,
        binding: &super::model::RestCatalogBinding,
        scope: RestVisibilityScope,
    ) -> Result<&'a RestSiteDefinition, RestSiteError> {
        if binding != &self.catalog.binding {
            return Err(RestSiteError::StaleReference);
        }
        let definition = self
            .catalog
            .definition(site_id)
            .ok_or(RestSiteError::NotFound)?;
        if !visibility_allowed(definition.visibility, scope) {
            return Err(RestSiteError::ExcludedByScope);
        }
        Ok(definition)
    }

    fn option_for_reference(
        &self,
        reference: &RestOptionReference,
        scope: RestVisibilityScope,
    ) -> Result<&'a RestOption, RestSiteError> {
        let definition = self.site_of(&reference.site_id, &reference.catalog, scope)?;
        let option = definition
            .options
            .get(&reference.option_id)
            .ok_or(RestSiteError::NotFound)?;
        if !visible_option(option, scope) {
            return Err(RestSiteError::ExcludedByScope);
        }
        Ok(option)
    }
}
