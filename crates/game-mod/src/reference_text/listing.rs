// SPDX-License-Identifier: MIT

//! Family-partitioned listing, bounded continuation and coverage for the reference catalog.
//!
//! A continuation is bound to one family filter, one catalog revision and one query, so a page
//! walk can never silently mix two families or survive a text revision change.  Coverage is
//! reported with every page so an empty result is distinguishable from a family this producer
//! does not project.

use super::catalog::{DocumentKey, ReferenceTextCatalog};
use super::model::{ReferenceDiscovery, ReferenceFamily, validate_identity};
use super::{REFERENCE_TEXT_DEFAULT_PAGE_ITEMS, REFERENCE_TEXT_MAX_PAGE_ITEMS, ReferenceTextError};

/// Query for a family-partitioned reference listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceListQuery {
    /// Restrict to one family; `None` lists every family.
    pub family: Option<ReferenceFamily>,
    /// Restrict to documents carrying this keyword; `None` does not filter by keyword.
    pub keyword: Option<String>,
    /// Restrict to one locale; `None` lists every locale.
    pub locale: Option<String>,
    /// Requested page size.
    pub page_size: usize,
}

impl Default for ReferenceListQuery {
    fn default() -> Self {
        Self {
            family: None,
            keyword: None,
            locale: None,
            page_size: REFERENCE_TEXT_DEFAULT_PAGE_ITEMS,
        }
    }
}

/// Continuation bound to one family filter, one catalog revision and one query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceContinuation {
    /// Family signature the page was produced for.
    pub family_signature: String,
    /// Catalog revision signature the page was produced under.
    pub revision_signature: String,
    /// Query signature the page was produced for.
    pub query_signature: String,
    /// Next offset into the filtered listing.
    pub offset: usize,
}

/// One summary row of a family-partitioned listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceSummary {
    /// Reference family of this document.
    pub family: ReferenceFamily,
    /// Stable document identity.
    pub namespaced_id: String,
    /// Locale this document is authored in.
    pub locale: String,
    /// Discovery policy that governs reading this document.
    pub discovery: ReferenceDiscovery,
    /// Text revision of this document.
    pub revision: String,
    /// Number of sections this document declares.
    pub section_count: usize,
    /// Searchable keywords, deterministically ordered.
    pub keywords: Vec<String>,
}

/// Per-family coverage of one catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceFamilyCoverage {
    /// Family this row describes.
    pub family: ReferenceFamily,
    /// Number of documents inventoried for this family.
    pub documents: usize,
    /// Number of sections across those documents.
    pub sections: usize,
}

/// Coverage report carried with every page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceCoverage {
    /// One row per family, in canonical order.
    pub families: Vec<ReferenceFamilyCoverage>,
    /// Documents whose family this producer does not project, reported rather than omitted.
    pub unsupported_documents: Vec<String>,
}

/// One page of a family-partitioned listing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferencePage {
    /// Summary rows in deterministic order.
    pub items: Vec<ReferenceSummary>,
    /// Total matching documents across every page.
    pub total: usize,
    /// Continuation for the next page, when another page exists.
    pub next: Option<ReferenceContinuation>,
    /// Coverage of the whole catalog, independent of this page's filter.
    pub coverage: ReferenceCoverage,
}

impl ReferenceTextCatalog {
    /// Lists inventoried reference documents under one bounded page.
    pub fn page(
        &self,
        query: &ReferenceListQuery,
        continuation: Option<&ReferenceContinuation>,
    ) -> Result<ReferencePage, ReferenceTextError> {
        if query.page_size == 0 || query.page_size > REFERENCE_TEXT_MAX_PAGE_ITEMS {
            return Err(ReferenceTextError::InvalidPageSize);
        }
        if let Some(locale) = &query.locale {
            validate_identity(locale, "locale")?;
        }
        if let Some(keyword) = &query.keyword {
            super::hygiene::validate_keyword(keyword)?;
        }
        let family_signature = query
            .family
            .map_or_else(|| "*".to_owned(), |family| family.as_str().to_owned());
        let query_signature = format!(
            "family={family_signature};keyword={};locale={};page_size={}",
            query.keyword.clone().unwrap_or_else(|| "*".to_owned()),
            query.locale.clone().unwrap_or_else(|| "*".to_owned()),
            query.page_size
        );
        let revision_signature = self.revision_signature();
        let filtered: Vec<&DocumentKey> = self
            .ordered
            .iter()
            .filter(|key| query.family.is_none_or(|family| key.0 == family))
            .filter(|key| {
                query.locale.as_ref().is_none_or(|locale| {
                    self.documents
                        .get(*key)
                        .is_some_and(|d| &d.locale == locale)
                })
            })
            .filter(|key| {
                query.keyword.as_ref().is_none_or(|keyword| {
                    self.keywords
                        .get(*key)
                        .is_some_and(|terms| terms.contains(keyword))
                })
            })
            .collect();
        let total = filtered.len();
        let offset = match continuation {
            None => 0,
            Some(token) => {
                if token.family_signature != family_signature {
                    return Err(ReferenceTextError::QueryMismatch);
                }
                if token.revision_signature != revision_signature {
                    return Err(ReferenceTextError::StaleReference);
                }
                if token.query_signature != query_signature {
                    return Err(ReferenceTextError::InvalidContinuation);
                }
                if token.offset > total {
                    return Err(ReferenceTextError::InvalidContinuation);
                }
                token.offset
            }
        };
        let end = offset.saturating_add(query.page_size).min(total);
        let mut items = Vec::with_capacity(end.saturating_sub(offset));
        for key in &filtered[offset..end] {
            let Some(stored) = self.documents.get(*key) else {
                return Err(ReferenceTextError::InvalidContinuation);
            };
            items.push(ReferenceSummary {
                family: key.0,
                namespaced_id: key.1.clone(),
                locale: stored.locale.clone(),
                discovery: stored.discovery,
                revision: stored.revision.clone(),
                section_count: stored.sections.len(),
                keywords: self
                    .keywords
                    .get(*key)
                    .map(|terms| terms.iter().cloned().collect())
                    .unwrap_or_default(),
            });
        }
        let next = if end < total {
            Some(ReferenceContinuation {
                family_signature,
                revision_signature,
                query_signature,
                offset: end,
            })
        } else {
            None
        };
        Ok(ReferencePage {
            items,
            total,
            next,
            coverage: self.coverage(),
        })
    }

    /// Reports per-family coverage, including documents this producer cannot project.
    #[must_use]
    pub fn coverage(&self) -> ReferenceCoverage {
        let families = ReferenceFamily::ALL
            .iter()
            .map(|family| {
                let matching: Vec<&DocumentKey> =
                    self.ordered.iter().filter(|key| key.0 == *family).collect();
                let sections = matching
                    .iter()
                    .filter_map(|key| self.documents.get(*key))
                    .map(|stored| stored.sections.len())
                    .sum();
                ReferenceFamilyCoverage {
                    family: *family,
                    documents: matching.len(),
                    sections,
                }
            })
            .collect();
        ReferenceCoverage {
            families,
            unsupported_documents: self.unsupported_families.iter().cloned().collect(),
        }
    }

    pub(super) fn revision_signature(&self) -> String {
        format!(
            "{}|{}|{}|{}",
            self.binding.producer_version,
            self.binding.manifest.content_set_revision,
            self.binding.manifest.localized_text_revision,
            self.locale
        )
    }
}
