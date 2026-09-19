// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    cursor::ShopCursors,
    definition::{ShopCatalog, ShopDefinition},
    entry::ShopEntry,
    error::ShopCatalogError,
    model::{
        SHOP_MAX_PAGE_ITEMS, ShopCatalogBinding, ShopDefinitionReference, ShopEntryReference,
        ShopFamilyState, ShopServiceReference, ShopStockReference, ShopStockState,
        ShopVisibilityScope,
    },
    page::{
        ContinuationScope, ShopDefinitionPage, ShopEntryListQuery, ShopEntryPage, ShopListQuery,
        ShopServiceListQuery, ShopServicePage,
    },
    projection::{
        entry_summary, map_status, project_definition, service_summary, shop_summary,
        visibility_allowed, visible_entry, visible_service,
    },
    service::ShopService,
};

/// Reader retaining one catalog while enforcing locale, scope, and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use. Call [`ShopCatalog::reader`] for an independent reader instead. Every method here is
/// a bounded read: none buys, sells, restocks, or spends gold.
#[derive(Debug)]
pub struct ShopCatalogReader {
    catalog: ShopCatalog,
    cursors: ShopCursors,
    scope: Arc<ContinuationScope>,
}

impl ShopCatalogReader {
    pub(super) fn new(catalog: ShopCatalog) -> Self {
        Self {
            catalog,
            cursors: ShopCursors::new(),
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &ShopCatalog {
        &self.catalog
    }

    /// Lists visible shop definitions in stable shop-identity order.
    pub fn list(&mut self, query: &ShopListQuery) -> Result<ShopDefinitionPage, ShopCatalogError> {
        if query.locale != self.catalog.binding.locale {
            return Err(ShopCatalogError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > SHOP_MAX_PAGE_ITEMS {
            return Err(ShopCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self
            .cursors
            .shop_start(&self.catalog.binding, &self.scope, query)?;
        let entries = self
            .catalog
            .definitions
            .values()
            .filter(|definition| visibility_allowed(definition.visibility, query.scope))
            .map(|definition| shop_summary(definition, query.scope))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start..end].to_vec();
        let continuation =
            self.cursors
                .next_shop(&self.catalog.binding, &self.scope, query, end, total);
        Ok(ShopDefinitionPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible inventory entries of one exact shop definition.
    pub fn list_entries(
        &mut self,
        query: &ShopEntryListQuery,
    ) -> Result<ShopEntryPage, ShopCatalogError> {
        if query.limit == 0 || query.limit > SHOP_MAX_PAGE_ITEMS {
            return Err(ShopCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self
            .cursors
            .entry_start(&self.catalog.binding, &self.scope, query)?;
        let definition = self.shop(&query.shop, query.scope)?;
        let entries_all = definition
            .entries
            .values()
            .filter(|entry| visible_entry(entry, query.scope))
            .map(entry_summary)
            .collect::<Vec<_>>();
        let entries_status = map_status(definition.entries.values(), query.scope, visible_entry);
        let total = entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries_all[start..end].to_vec();
        let continuation =
            self.cursors
                .next_entry(&self.catalog.binding, &self.scope, query, end, total);
        Ok(ShopEntryPage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            entries_status,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Lists visible services of one exact shop definition.
    pub fn list_services(
        &mut self,
        query: &ShopServiceListQuery,
    ) -> Result<ShopServicePage, ShopCatalogError> {
        if query.limit == 0 || query.limit > SHOP_MAX_PAGE_ITEMS {
            return Err(ShopCatalogError::InvalidPageSize);
        }
        self.validate_family()?;
        let start = self
            .cursors
            .service_start(&self.catalog.binding, &self.scope, query)?;
        let definition = self.shop(&query.shop, query.scope)?;
        let entries_all = definition
            .services
            .values()
            .filter(|service| visible_service(service, query.scope))
            .map(service_summary)
            .collect::<Vec<_>>();
        let services_status =
            map_status(definition.services.values(), query.scope, visible_service);
        let total = entries_all.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries_all[start..end].to_vec();
        let continuation =
            self.cursors
                .next_service(&self.catalog.binding, &self.scope, query, end, total);
        Ok(ShopServicePage {
            binding: self.catalog.binding.clone(),
            entries: page_entries,
            total,
            services_status,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Performs one exact shop definition lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &ShopDefinitionReference,
        scope: ShopVisibilityScope,
    ) -> Result<ShopDefinition, ShopCatalogError> {
        self.validate_family()?;
        Ok(project_definition(self.shop(reference, scope)?, scope))
    }

    /// Performs one exact inventory entry lookup under an explicit visibility scope.
    pub fn get_entry(
        &self,
        reference: &ShopEntryReference,
        scope: ShopVisibilityScope,
    ) -> Result<ShopEntry, ShopCatalogError> {
        self.validate_family()?;
        Ok(self.entry_for_reference(reference, scope)?.clone())
    }

    /// Performs one exact service lookup under an explicit visibility scope.
    pub fn get_service(
        &self,
        reference: &ShopServiceReference,
        scope: ShopVisibilityScope,
    ) -> Result<ShopService, ShopCatalogError> {
        self.validate_family()?;
        Ok(self.service_for_reference(reference, scope)?.clone())
    }

    /// Returns the stock a reference describes, or refuses a reference from an earlier restock.
    pub fn stock_for_reference(
        &self,
        reference: &ShopStockReference,
        scope: ShopVisibilityScope,
    ) -> Result<ShopStockState, ShopCatalogError> {
        self.validate_family()?;
        let entry = self.entry_for_reference(&reference.entry, scope)?;
        let definition = self.shop_of(&reference.entry.shop_id, &reference.entry.catalog, scope)?;
        if reference.generation != definition.generation {
            return Err(ShopCatalogError::StaleStockReference {
                entry_id: reference.entry.entry_id.clone(),
                referenced: reference.generation,
                current: definition.generation,
            });
        }
        Ok(entry.stock.clone())
    }

    fn validate_family(&self) -> Result<(), ShopCatalogError> {
        match self.catalog.family.state {
            ShopFamilyState::Handled => Ok(()),
            ShopFamilyState::Unsupported => Err(ShopCatalogError::UnsupportedFamily),
            ShopFamilyState::Unavailable => Err(ShopCatalogError::UnavailableFamily),
        }
    }

    fn shop(
        &self,
        reference: &ShopDefinitionReference,
        scope: ShopVisibilityScope,
    ) -> Result<&ShopDefinition, ShopCatalogError> {
        self.shop_of(&reference.shop_id, &reference.catalog, scope)
    }

    fn shop_of(
        &self,
        shop_id: &str,
        binding: &ShopCatalogBinding,
        scope: ShopVisibilityScope,
    ) -> Result<&ShopDefinition, ShopCatalogError> {
        if binding != &self.catalog.binding {
            return Err(ShopCatalogError::StaleReference);
        }
        let definition = self
            .catalog
            .definitions
            .get(shop_id)
            .ok_or(ShopCatalogError::NotFound)?;
        if !visibility_allowed(definition.visibility, scope) {
            return Err(ShopCatalogError::ExcludedByScope);
        }
        Ok(definition)
    }

    fn entry_for_reference(
        &self,
        reference: &ShopEntryReference,
        scope: ShopVisibilityScope,
    ) -> Result<&ShopEntry, ShopCatalogError> {
        let definition = self.shop_of(&reference.shop_id, &reference.catalog, scope)?;
        let entry = definition
            .entries
            .get(&reference.entry_id)
            .ok_or(ShopCatalogError::NotFound)?;
        if !visible_entry(entry, scope) {
            return Err(ShopCatalogError::ExcludedByScope);
        }
        Ok(entry)
    }

    fn service_for_reference(
        &self,
        reference: &ShopServiceReference,
        scope: ShopVisibilityScope,
    ) -> Result<&ShopService, ShopCatalogError> {
        let definition = self.shop_of(&reference.shop_id, &reference.catalog, scope)?;
        let service = definition
            .services
            .get(&reference.service_id)
            .ok_or(ShopCatalogError::NotFound)?;
        if !visible_service(service, scope) {
            return Err(ShopCatalogError::ExcludedByScope);
        }
        Ok(service)
    }
}
