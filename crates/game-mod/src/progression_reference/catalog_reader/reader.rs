// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::sync::Arc;

use super::super::{
    PROGRESSION_MAX_PAGE_ITEMS, ProgressionDetailQuery, ProgressionDomain, ProgressionDomainState,
    ProgressionEntry, ProgressionEntryReference, ProgressionProfileKind, ProgressionProfileQuery,
    ProgressionReadAuthority, ProgressionReferenceError, ProgressionRevision,
};
use super::catalog::ProgressionCatalog;
use super::page::{
    ContinuationScope, CursorState, ProgressionContinuation, ProgressionEntryPage,
    ProgressionEntrySummary, ProgressionListQuery,
};

/// Reader retaining one catalog while enforcing locale, scope, profile, revision and cursor fences.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use.  Call [`ProgressionCatalog::reader`] for an independent reader instead.  It exposes
/// no unlock, purchase, save-write or profile-selection entry point, so a progression read cannot
/// change state and cannot reach another profile.
#[derive(Debug)]
pub struct ProgressionCatalogReader {
    pub(super) catalog: ProgressionCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl ProgressionCatalogReader {
    pub(super) fn new(catalog: ProgressionCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &ProgressionCatalog {
        &self.catalog
    }

    /// Returns the capability this read withholds.
    #[must_use]
    pub const fn authority(&self) -> ProgressionReadAuthority {
        ProgressionReadAuthority::NotGranted
    }

    /// Returns the documented freshness of the retained snapshot.
    #[must_use]
    pub fn revision(&self) -> ProgressionRevision {
        self.catalog.revision()
    }

    /// Lists one bounded page of visible entries in stable identity order.
    pub fn list(
        &mut self,
        query: &ProgressionListQuery,
    ) -> Result<ProgressionEntryPage, ProgressionReferenceError> {
        if query.locale != self.catalog.binding.locale {
            return Err(ProgressionReferenceError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > PROGRESSION_MAX_PAGE_ITEMS {
            return Err(ProgressionReferenceError::InvalidPageSize);
        }
        self.validate_revision(query.revision.as_ref())?;
        self.validate_profile(&query.profile)?;
        if let Some(domain) = query.domain {
            self.validate_domain(domain)?;
        }
        let start = self.take_cursor(query)?;
        let revision = self.catalog.revision();
        let entries = self
            .catalog
            .entries
            .values()
            .filter(|entry| observable(entry, query))
            .map(|entry| summary(entry, &revision))
            .collect::<Vec<_>>();
        let total = entries.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_entries = entries[start.min(end)..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token();
            self.cursors.insert(
                token.clone(),
                CursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    profile: query.profile.clone(),
                    scope: query.scope,
                    domain: query.domain,
                    state: query.state,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(ProgressionContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(ProgressionEntryPage {
            binding: self.catalog.binding.clone(),
            revision,
            authority: ProgressionReadAuthority::NotGranted,
            entries: page_entries,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Reads one exact entry's complete retained detail.
    pub fn get(
        &mut self,
        query: &ProgressionDetailQuery,
    ) -> Result<ProgressionEntry, ProgressionReferenceError> {
        if query.entry.catalog != self.catalog.binding {
            return Err(ProgressionReferenceError::StaleReference);
        }
        self.validate_revision(query.revision.as_ref())?;
        self.validate_profile(&query.profile)?;
        let entry = self
            .catalog
            .entries
            .get(&query.entry.entry_id)
            .ok_or(ProgressionReferenceError::NotFound)?;
        self.validate_domain(entry.domain)?;
        if !query.scope.observes(entry.visibility) {
            return Err(ProgressionReferenceError::ExcludedByScope);
        }
        Ok(entry.clone())
    }

    /// Refuses a read whose stated revision is not this snapshot's revision.
    fn validate_revision(
        &self,
        revision: Option<&ProgressionRevision>,
    ) -> Result<(), ProgressionReferenceError> {
        match revision {
            None => Ok(()),
            Some(revision) if *revision == self.catalog.revision() => Ok(()),
            Some(_) => Err(ProgressionReferenceError::ProfileRevisionMismatch),
        }
    }

    /// Refuses a profile this boundary cannot read without switching the active one.
    fn validate_profile(
        &self,
        query: &ProgressionProfileQuery,
    ) -> Result<(), ProgressionReferenceError> {
        match query {
            ProgressionProfileQuery::Active => Ok(()),
            ProgressionProfileQuery::Named { user_data_id, kind } => {
                if !matches!(kind, ProgressionProfileKind::Active) {
                    return Err(ProgressionReferenceError::ForeignProfileRequiresReadPort);
                }
                if *user_data_id != self.catalog.binding.profile.user_data_id {
                    return Err(ProgressionReferenceError::ImplicitProfileSwitch);
                }
                Ok(())
            }
        }
    }

    /// Refuses an entry in a domain the source does not project.
    fn validate_domain(&self, domain: ProgressionDomain) -> Result<(), ProgressionReferenceError> {
        let coverage = self
            .catalog
            .domain_state(domain)
            .ok_or(ProgressionReferenceError::DomainCoverageIncomplete)?;
        if matches!(coverage.state, ProgressionDomainState::Projected) {
            Ok(())
        } else {
            Err(ProgressionReferenceError::UnavailableDomain)
        }
    }

    fn take_cursor(
        &mut self,
        query: &ProgressionListQuery,
    ) -> Result<usize, ProgressionReferenceError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(ProgressionReferenceError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(ProgressionReferenceError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.profile != query.profile
            || cursor.scope != query.scope
            || cursor.domain != query.domain
            || cursor.state != query.state
            || cursor.limit != query.limit
        {
            return Err(ProgressionReferenceError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self) -> String {
        let token = format!("progression-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

fn summary(entry: &ProgressionEntry, revision: &ProgressionRevision) -> ProgressionEntrySummary {
    ProgressionEntrySummary {
        reference: ProgressionEntryReference {
            catalog: entry.binding.clone(),
            entry_id: entry.entry_id.clone(),
        },
        revision: revision.clone(),
        domain: entry.domain,
        read_state: entry.read_state,
        title: entry.title.clone(),
        description: entry.description.clone(),
        progress: entry.progress.clone(),
        best: entry.best.clone(),
        stated_requirements: entry.requirements.value().map_or(0, std::vec::Vec::len),
        visibility: entry.visibility,
    }
}

fn observable(entry: &ProgressionEntry, query: &ProgressionListQuery) -> bool {
    query.scope.observes(entry.visibility)
        && query.domain.is_none_or(|domain| entry.domain == domain)
        && query.state.is_none_or(|state| entry.read_state == state)
}
