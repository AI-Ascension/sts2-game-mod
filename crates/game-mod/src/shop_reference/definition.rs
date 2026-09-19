// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    entry::{ShopEntry, ShopEntryInput},
    error::ShopCatalogError,
    field::{ShopFieldStatus, ShopText},
    model::{
        ShopCatalogBinding, ShopEvidence, ShopFamilyCoverage, ShopSemanticReference,
        ShopVisibility, ShopVisibilityScope,
    },
    reader::ShopCatalogReader,
    restock::ShopRestockRule,
    service::{ShopService, ShopServiceInput},
};

pub use super::model::ShopDefinitionReference;

/// Complete source-owned static shop definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopDefinitionInput {
    /// Namespaced shop definition identity.
    pub shop_id: String,
    /// Localized shop label.
    pub label: ShopText,
    /// Visibility of the definition itself.
    pub visibility: ShopVisibility,
    /// Evidence label for the definition.
    pub evidence: ShopEvidence,
    /// Restock generation the source observed for this shop.
    ///
    /// Every restock rule must produce a strictly greater generation, so a stock reference can be
    /// fenced to the generation that produced it.
    pub generation: u64,
    /// Complete inventory entries.
    pub entries: Vec<ShopEntryInput>,
    /// Complete services.
    pub services: Vec<ShopServiceInput>,
    /// Restock rules and the generations they produce.
    pub restock: Vec<ShopRestockRule>,
    /// Top-level typed references.
    pub references: Vec<ShopSemanticReference>,
}

/// Immutable shop definition bound to one manifest, locale, and producer identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopDefinition {
    /// Exact static definition reference.
    pub reference: ShopDefinitionReference,
    /// Localized shop label.
    pub label: ShopText,
    /// Visibility of the definition itself.
    pub visibility: ShopVisibility,
    /// Evidence label for the definition.
    pub evidence: ShopEvidence,
    /// Restock generation the source observed for this shop.
    pub generation: u64,
    /// Inventory entries keyed by stable entry identity.
    pub entries: BTreeMap<String, ShopEntry>,
    /// Services keyed by exact-build service identity.
    pub services: BTreeMap<String, ShopService>,
    /// Restock rules and the generations they produce.
    pub restock: Vec<ShopRestockRule>,
    /// Availability of the restock collection after scope withholding.
    pub restock_status: ShopFieldStatus,
    /// Top-level typed references.
    pub references: Vec<ShopSemanticReference>,
}

impl ShopDefinition {
    pub(super) fn from_input(binding: &ShopCatalogBinding, input: ShopDefinitionInput) -> Self {
        let reference = ShopDefinitionReference {
            catalog: binding.clone(),
            shop_id: input.shop_id.clone(),
        };
        let entries = input
            .entries
            .into_iter()
            .map(|entry| {
                let entry_id = entry.entry_id.clone();
                (
                    entry_id,
                    ShopEntry::from_input(binding, &input.shop_id, entry),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let services = input
            .services
            .into_iter()
            .map(|service| {
                let service_id = service.service_id.clone();
                (
                    service_id,
                    ShopService::from_input(binding, &input.shop_id, service),
                )
            })
            .collect::<BTreeMap<_, _>>();
        Self {
            reference,
            label: input.label,
            visibility: input.visibility,
            evidence: input.evidence,
            generation: input.generation,
            entries,
            services,
            restock: input.restock,
            restock_status: ShopFieldStatus::Available,
            references: input.references,
        }
    }

    /// Returns one entry by stable identity.
    #[must_use]
    pub fn entry(&self, entry_id: &str) -> Option<&ShopEntry> {
        self.entries.get(entry_id)
    }

    /// Returns one service by exact-build identity.
    #[must_use]
    pub fn service(&self, service_id: &str) -> Option<&ShopService> {
        self.services.get(service_id)
    }

    /// Returns the number of inventory entries.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the number of services.
    #[must_use]
    pub fn service_count(&self) -> usize {
        self.services.len()
    }

    /// Returns the observed restock generation that fences stock references.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }
}

/// Immutable shop catalog fenced by one content manifest, locale, and producer identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopCatalog {
    pub(super) binding: ShopCatalogBinding,
    pub(super) family: ShopFamilyCoverage,
    pub(super) definitions: BTreeMap<String, ShopDefinition>,
}

impl ShopCatalog {
    pub(crate) fn from_parts(
        binding: ShopCatalogBinding,
        family: ShopFamilyCoverage,
        definitions: BTreeMap<String, ShopDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &ShopCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns explicit support coverage for the shop family.
    #[must_use]
    pub fn family(&self) -> &ShopFamilyCoverage {
        &self.family
    }

    /// Returns the number of retained shop definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns whether no shop definition was retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Returns one retained shop definition by identity.
    #[must_use]
    pub fn definition(&self, shop_id: &str) -> Option<&ShopDefinition> {
        self.definitions.get(shop_id)
    }

    /// Returns one visible shop projection under an explicit scope.
    pub fn get(
        &self,
        reference: &ShopDefinitionReference,
        scope: ShopVisibilityScope,
    ) -> Result<ShopDefinition, ShopCatalogError> {
        self.reader().get(reference, scope)
    }

    /// Consumes the catalog into an independent reader with its own bounded cursors.
    #[must_use]
    pub fn reader(&self) -> ShopCatalogReader {
        ShopCatalogReader::new(self.clone())
    }
}
