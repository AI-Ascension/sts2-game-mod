// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    field::{RestFieldStatus, RestText},
    model::{
        RestCatalogBinding, RestEvidence, RestFamilyCoverage, RestSemanticReference,
        RestSiteDefinitionReference, RestVisibility, RestVisibilityScope,
    },
    option::{RestCoverageRecord, RestOption, RestOptionInput},
    reader::RestSiteReader,
};

/// Kind of rest site the definition describes.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestSiteKind {
    /// A rest site offering healing and card work.
    RestSite,
    /// Owner-defined site kind.
    Custom(String),
    /// Source could not classify the site.
    Unknown,
}

/// Owner-supplied rest-site definition used to construct one immutable catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestSiteDefinitionInput {
    /// Namespaced rest-site identity.
    pub site_id: String,
    /// Localized site label.
    pub label: RestText,
    /// Localized site description.
    pub description: RestText,
    /// Kind of rest site.
    pub kind: RestSiteKind,
    /// Visibility of the definition.
    pub visibility: RestVisibility,
    /// Evidence label for the definition.
    pub evidence: RestEvidence,
    /// Generation of the option set this definition describes.
    pub option_set_generation: u64,
    /// Option identities the host reports for this option set.
    ///
    /// Every reported identity must be either described by a typed option or named by a coverage
    /// record, so a newly audited button cannot be silently omitted.
    pub observed_options: Vec<String>,
    /// Typed rest options for this site.
    pub options: Vec<RestOptionInput>,
    /// Named coverage records for reported options without a typed record.
    pub coverage: Vec<RestCoverageRecord>,
    /// Definitions the definition refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Immutable rest-site definition with bounded, deterministic option collections.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestSiteDefinition {
    /// Exact static definition reference.
    pub reference: RestSiteDefinitionReference,
    /// Localized site label.
    pub label: RestText,
    /// Localized site description.
    pub description: RestText,
    /// Kind of rest site.
    pub kind: RestSiteKind,
    /// Visibility of the definition.
    pub visibility: RestVisibility,
    /// Evidence label for the definition.
    pub evidence: RestEvidence,
    /// Generation of the option set this definition describes.
    pub option_set_generation: u64,
    /// Number of option identities the host reported for this option set.
    pub observed_option_count: usize,
    /// Typed options keyed by option identity.
    pub options: BTreeMap<String, RestOption>,
    /// Named coverage records for audited options without a typed record.
    pub coverage: Vec<RestCoverageRecord>,
    /// Availability of the option list after scope withholding.
    pub options_status: RestFieldStatus,
    /// Availability of the coverage list after scope withholding.
    pub coverage_status: RestFieldStatus,
    /// Definitions the definition refers to.
    pub references: Vec<RestSemanticReference>,
}

/// Immutable, read-only rest-site reference catalog.
///
/// The catalog owns no setter, no rest command, and no card mutation: it is a bounded copy of
/// static reference data fenced by one content manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestSiteCatalog {
    /// Static identity shared by every record in this catalog.
    pub binding: RestCatalogBinding,
    /// Explicit support state for the rest-site family.
    pub family: RestFamilyCoverage,
    definitions: BTreeMap<String, RestSiteDefinition>,
}

impl RestSiteCatalog {
    /// Binds validated definitions to one catalog identity.
    pub(super) fn from_parts(
        binding: RestCatalogBinding,
        family: RestFamilyCoverage,
        definitions: BTreeMap<String, RestSiteDefinition>,
    ) -> Self {
        Self {
            binding,
            family,
            definitions,
        }
    }

    /// Returns the locale every localized value was copied for.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns the number of rest-site definitions.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns whether the catalog carries no rest-site definition.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Returns one rest-site definition by identity.
    #[must_use]
    pub fn definition(&self, site_id: &str) -> Option<&RestSiteDefinition> {
        self.definitions.get(site_id)
    }

    /// Returns one rest option by its site and option identity.
    #[must_use]
    pub fn option(&self, site_id: &str, option_id: &str) -> Option<&RestOption> {
        self.definitions
            .get(site_id)
            .and_then(|definition| definition.options.get(option_id))
    }

    /// Returns every bound definition for same-catalog resolution.
    #[must_use]
    pub(super) fn definitions(&self) -> &BTreeMap<String, RestSiteDefinition> {
        &self.definitions
    }

    /// Returns an independent reader over this catalog under one visibility scope.
    #[must_use]
    pub fn reader(&self, scope: RestVisibilityScope) -> RestSiteReader<'_> {
        RestSiteReader::new(self, scope)
    }
}
