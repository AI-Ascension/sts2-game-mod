// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    SettingDefinition, SettingsCatalogBinding, SettingsDefinitionReference,
    SettingsDefinitionSummary, SettingsFamilyCoverage, SettingsProfile, SettingsReferenceError,
    SettingsVisibilityScope,
};
use super::reader::SettingsCatalogReader;

/// Immutable setting definitions keyed by namespaced setting identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsCatalog {
    pub(super) binding: SettingsCatalogBinding,
    pub(super) family: SettingsFamilyCoverage,
    pub(super) definitions: BTreeMap<String, SettingDefinition>,
}

impl SettingsCatalog {
    pub(crate) fn from_parts(
        binding: SettingsCatalogBinding,
        family: SettingsFamilyCoverage,
        definitions: BTreeMap<String, SettingDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the manifest, locale, profile, and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &SettingsCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns the profile the values were read under.
    #[must_use]
    pub fn profile(&self) -> &SettingsProfile {
        &self.binding.profile
    }

    /// Returns explicit support coverage for the settings family.
    #[must_use]
    pub fn family(&self) -> &SettingsFamilyCoverage {
        &self.family
    }

    /// Returns an immutable catalog reader with an independent bounded cursor registry.
    #[must_use]
    pub fn reader(&self) -> SettingsCatalogReader {
        SettingsCatalogReader::new(self.clone())
    }

    /// Performs one exact setting lookup under an explicit visibility scope.
    pub fn get(
        &self,
        reference: &SettingsDefinitionReference,
        scope: SettingsVisibilityScope,
    ) -> Result<SettingDefinition, SettingsReferenceError> {
        self.reader().get(reference, scope)
    }

    /// Lists one bounded page of visible settings.
    pub fn list(
        &self,
        query: &super::page::SettingsListQuery,
    ) -> Result<super::page::SettingsDefinitionPage, SettingsReferenceError> {
        self.reader().list(query)
    }

    /// Returns every visible summary, ignoring pagination, for bounded owner-side consumers.
    pub fn summaries(
        &self,
        locale: &str,
        scope: SettingsVisibilityScope,
    ) -> Result<Vec<SettingsDefinitionSummary>, SettingsReferenceError> {
        let mut reader = self.reader();
        let mut entries = Vec::new();
        let mut continuation = None;
        loop {
            let page = reader.list(&super::page::SettingsListQuery {
                locale: locale.to_owned(),
                category: None,
                level: None,
                scope,
                limit: super::super::model::SETTINGS_MAX_PAGE_ITEMS,
                continuation,
            })?;
            entries.extend(page.entries);
            match page.continuation {
                Some(next) => continuation = Some(next),
                None => return Ok(entries),
            }
        }
    }
}
