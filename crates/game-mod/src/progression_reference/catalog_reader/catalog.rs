// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::{
    ProgressionCatalogBinding, ProgressionDetailQuery, ProgressionDomain,
    ProgressionDomainCoverage, ProgressionEntry, ProgressionEntryReference, ProgressionProfile,
    ProgressionProfileQuery, ProgressionReferenceError, ProgressionRevision,
    ProgressionVisibilityScope,
};
use super::page::{ProgressionEntryPage, ProgressionListQuery};
use super::reader::ProgressionCatalogReader;

/// Immutable progression entries keyed by stable identity, with their declared coverage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressionCatalog {
    pub(super) binding: ProgressionCatalogBinding,
    pub(super) domains: Vec<ProgressionDomainCoverage>,
    pub(super) entries: BTreeMap<String, ProgressionEntry>,
}

impl ProgressionCatalog {
    pub(crate) fn from_parts(
        binding: ProgressionCatalogBinding,
        domains: Vec<ProgressionDomainCoverage>,
        entries: BTreeMap<String, ProgressionEntry>,
    ) -> Self {
        Self {
            binding,
            domains,
            entries,
        }
    }

    /// Returns the manifest, locale, profile, revision and producer identity fence.
    #[must_use]
    pub fn binding(&self) -> &ProgressionCatalogBinding {
        &self.binding
    }

    /// Returns the exact locale used by this catalog.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.binding.locale
    }

    /// Returns the profile the entries were read under.
    #[must_use]
    pub fn profile(&self) -> &ProgressionProfile {
        &self.binding.profile
    }

    /// Returns the documented freshness of this read.
    #[must_use]
    pub fn revision(&self) -> ProgressionRevision {
        self.binding.revision()
    }

    /// Returns per-domain support state, including unsupported and unavailable domains.
    #[must_use]
    pub fn domains(&self) -> &[ProgressionDomainCoverage] {
        &self.domains
    }

    /// Returns the declared state of one domain.
    #[must_use]
    pub fn domain_state(&self, domain: ProgressionDomain) -> Option<&ProgressionDomainCoverage> {
        self.domains.iter().find(|row| row.domain == domain)
    }

    /// Returns an immutable catalog reader with an independent bounded cursor registry.
    #[must_use]
    pub fn reader(&self) -> ProgressionCatalogReader {
        ProgressionCatalogReader::new(self.clone())
    }

    /// Performs one exact entry lookup under an explicit profile and visibility scope.
    pub fn get(
        &self,
        reference: &ProgressionEntryReference,
        scope: ProgressionVisibilityScope,
        revision: Option<&ProgressionRevision>,
    ) -> Result<ProgressionEntry, ProgressionReferenceError> {
        self.reader().get(&ProgressionDetailQuery {
            entry: reference.clone(),
            profile: ProgressionProfileQuery::Active,
            scope,
            revision: revision.cloned(),
        })
    }

    /// Lists one complete page of visible entries from a fresh reader.
    ///
    /// This convenience holds no cursor registry, so it cannot serve a page the matching entries do
    /// not fit into: a partial result is refused rather than returned with a continuation no reader
    /// can consume.  Hold the reader from [`ProgressionCatalog::reader`] to walk every page.
    pub fn list(
        &self,
        query: &ProgressionListQuery,
    ) -> Result<ProgressionEntryPage, ProgressionReferenceError> {
        let page = self.reader().list(query)?;
        if page.is_partial() {
            return Err(ProgressionReferenceError::PartialPageRequiresReader);
        }
        Ok(page)
    }
}
