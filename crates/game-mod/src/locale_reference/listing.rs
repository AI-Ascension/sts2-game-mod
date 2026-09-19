// SPDX-License-Identifier: MIT

//! Locale-partitioned listing and bounded continuation for the locale reference catalog.
//!
//! A continuation is bound to one locale, one catalog revision and one query, so a page walk
//! can never silently mix two languages or survive a text revision change.

use super::catalog::{
    EntryKey, LocaleCatalog, LocaleContinuation, LocaleEntryListQuery, LocaleEntryPage,
    LocaleEntrySummary, StoredEntry,
};
use super::model::{LocalePluralCategory, validate_identity};
use super::{LOCALE_REFERENCE_MAX_PAGE_ITEMS, LocaleCatalogError};

impl LocaleCatalog {
    pub fn page(
        &self,
        query: &LocaleEntryListQuery,
        continuation: Option<&LocaleContinuation>,
    ) -> Result<LocaleEntryPage, LocaleCatalogError> {
        if query.page_size == 0 || query.page_size > LOCALE_REFERENCE_MAX_PAGE_ITEMS {
            return Err(LocaleCatalogError::InvalidPageSize);
        }
        if let Some(locale) = &query.locale {
            validate_identity(locale, "locale")?;
            if !self.locales.contains_key(locale) {
                return Err(LocaleCatalogError::UnsupportedLocale(locale.clone()));
            }
        }
        if let Some(kind) = &query.entity_kind {
            validate_identity(kind, "entity_kind")?;
        }
        let locale_signature = query.locale.clone().unwrap_or_else(|| "*".to_owned());
        let query_signature = format!(
            "locale={locale_signature};kind={}",
            query.entity_kind.clone().unwrap_or_else(|| "*".to_owned())
        );
        let revision_signature = self.revision_signature();
        let filtered: Vec<&EntryKey> = self
            .ordered_keys
            .iter()
            .filter(|key| query.locale.as_ref().is_none_or(|locale| &key.2 == locale))
            .filter(|key| query.entity_kind.as_ref().is_none_or(|kind| &key.0 == kind))
            .collect();
        let total = filtered.len();
        let offset = match continuation {
            None => 0,
            Some(token) => {
                if token.locale_signature != locale_signature {
                    return Err(LocaleCatalogError::LocaleMismatch);
                }
                if token.revision_signature != revision_signature {
                    return Err(LocaleCatalogError::StaleReference);
                }
                if token.query_signature != query_signature {
                    return Err(LocaleCatalogError::InvalidContinuation);
                }
                if token.offset > total {
                    return Err(LocaleCatalogError::InvalidContinuation);
                }
                token.offset
            }
        };
        let end = offset.saturating_add(query.page_size).min(total);
        let mut items = Vec::with_capacity(end.saturating_sub(offset));
        for key in &filtered[offset..end] {
            let stored = match self.entries.get(*key) {
                Some(stored) => stored,
                None => return Err(LocaleCatalogError::InvalidContinuation),
            };
            items.push(LocaleEntrySummary {
                entity_kind: key.0.clone(),
                namespaced_id: key.1.clone(),
                locale: key.2.clone(),
                plural: key.3,
                revision: stored.revision.clone(),
                segment_count: stored.segments.len(),
            });
        }
        let next = if end < total {
            Some(LocaleContinuation {
                locale_signature,
                revision_signature,
                query_signature,
                offset: end,
            })
        } else {
            None
        };
        Ok(LocaleEntryPage { items, total, next })
    }

    pub(super) fn revision_signature(&self) -> String {
        format!(
            "{}|{}",
            self.binding.manifest.content_set_revision,
            self.binding.manifest.localized_text_revision
        )
    }

    pub(super) fn find_entry(
        &self,
        entity_kind: &str,
        namespaced_id: &str,
        locale: &str,
        plural: Option<LocalePluralCategory>,
    ) -> Option<&StoredEntry> {
        let key_for = |category: LocalePluralCategory| {
            (
                entity_kind.to_owned(),
                namespaced_id.to_owned(),
                locale.to_owned(),
                category,
            )
        };
        if let Some(category) = plural
            && let Some(stored) = self.entries.get(&key_for(category))
        {
            return Some(stored);
        }
        if let Some(stored) = self.entries.get(&key_for(LocalePluralCategory::Other)) {
            return Some(stored);
        }
        if let Some(stored) = self.entries.get(&key_for(LocalePluralCategory::Unknown)) {
            return Some(stored);
        }
        self.entries
            .range(key_for(LocalePluralCategory::Zero)..=key_for(LocalePluralCategory::Unknown))
            .next()
            .map(|(_, stored)| stored)
    }
}
