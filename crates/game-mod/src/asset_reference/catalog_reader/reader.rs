// SPDX-License-Identifier: MIT

//! The non-clonable reader: scoped listing, exact lookup and the one bounded rendition call.

use std::collections::BTreeMap;
use std::sync::Arc;

use super::super::{
    ASSET_MAX_PAGE_ITEMS, AssetCatalogBinding, AssetDetailQuery, AssetEntry, AssetMediaClass,
    AssetReferenceError, AssetRendition, AssetRenditionAuthority, AssetRenditionAvailability,
    AssetRenditionPayload, AssetRenditionRequest, AssetRevision, AssetVisibilityScope,
};
use super::catalog::AssetCatalog;
use super::page::{AssetContinuation, AssetListQuery, AssetPage, ContinuationScope};
use super::projection::{availability_of, observable, summary};

#[derive(Debug)]
struct CursorState {
    binding: AssetCatalogBinding,
    locale: String,
    scope: AssetVisibilityScope,
    media_kind: Option<super::super::AssetMediaKind>,
    retrieval_state: Option<super::super::AssetRetrievalState>,
    limit: usize,
    offset: usize,
}

/// Non-clonable reader over one immutable asset catalog.
///
/// A reader is intentionally not clonable: its cursor registry is mutable and continuations are
/// single-use.  Call [`AssetCatalog::reader`] for an independent reader instead.  It exposes no
/// install, extraction, execution or game-data mutation entry point, and bytes leave it only
/// through [`AssetCatalogReader::retrieve`].
#[derive(Debug)]
pub struct AssetCatalogReader {
    pub(super) catalog: AssetCatalog,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<ContinuationScope>,
}

impl AssetCatalogReader {
    pub(super) fn new(catalog: AssetCatalog) -> Self {
        Self {
            catalog,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(ContinuationScope),
        }
    }

    /// Returns the immutable catalog retained by this reader.
    #[must_use]
    pub fn catalog(&self) -> &AssetCatalog {
        &self.catalog
    }

    /// Returns the capability this read withholds.
    #[must_use]
    pub const fn authority(&self) -> AssetRenditionAuthority {
        AssetRenditionAuthority::NotGranted
    }

    /// Returns the documented freshness of the retained snapshot.
    #[must_use]
    pub fn revision(&self) -> AssetRevision {
        self.catalog.revision()
    }

    /// Lists one bounded page of visible assets in stable handle order.
    pub fn list(&mut self, query: &AssetListQuery) -> Result<AssetPage, AssetReferenceError> {
        if query.locale != self.catalog.binding.locale {
            return Err(AssetReferenceError::LocaleMismatch);
        }
        if query.limit == 0 || query.limit > ASSET_MAX_PAGE_ITEMS {
            return Err(AssetReferenceError::InvalidPageSize);
        }
        self.validate_revision(query.revision.as_ref())?;
        if let Some(kind) = query.media_kind {
            self.validate_class(kind.media_class())?;
        }
        let start = self.take_cursor(query)?;
        let revision = self.catalog.revision();
        let assets = self
            .catalog
            .assets
            .values()
            .filter(|entry| observable(entry, query))
            .map(|entry| summary(entry, &revision))
            .collect::<Vec<_>>();
        let total = assets.len();
        let end = start.saturating_add(query.limit).min(total);
        let page_assets = assets[start.min(end)..end].to_vec();
        let continuation = if end < total {
            let token = self.make_token();
            self.cursors.insert(
                token.clone(),
                CursorState {
                    binding: self.catalog.binding.clone(),
                    locale: query.locale.clone(),
                    scope: query.scope,
                    media_kind: query.media_kind,
                    retrieval_state: query.retrieval_state,
                    limit: query.limit,
                    offset: end,
                },
            );
            Some(AssetContinuation::new(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(AssetPage {
            binding: self.catalog.binding.clone(),
            revision,
            authority: AssetRenditionAuthority::NotGranted,
            assets: page_assets,
            total,
            complete: continuation.is_none(),
            continuation,
        })
    }

    /// Reads one exact asset's retained metadata, never its bytes.
    pub fn get(&mut self, query: &AssetDetailQuery) -> Result<AssetEntry, AssetReferenceError> {
        if query.entry.catalog != self.catalog.binding {
            return Err(AssetReferenceError::StaleReference);
        }
        self.validate_revision(query.revision.as_ref())?;
        let entry = self
            .catalog
            .assets
            .get(query.entry.handle.as_str())
            .ok_or(AssetReferenceError::NotFound)?;
        self.validate_class(entry.media_kind.media_class())?;
        if !query.scope.observes(entry.visibility) {
            return Err(AssetReferenceError::ExcludedByScope);
        }
        Ok(entry.clone())
    }

    /// Serves one explicitly bounded rendition, or states why it cannot.
    ///
    /// Bytes travel on this surface alone, and only when the asset is retrievable, visible at this
    /// scope, unexpired, inside the owner-controlled bounds and of the exact requested media type.
    pub fn retrieve(
        &self,
        request: &AssetRenditionRequest,
        scope: AssetVisibilityScope,
    ) -> Result<AssetRendition, AssetReferenceError> {
        let key = request.handle.as_str();
        let entry = self
            .catalog
            .assets
            .get(key)
            .ok_or_else(|| AssetReferenceError::ForgedHandle(key.to_owned()))?;
        if self.catalog.binding.generation >= entry.expires_at_generation {
            return Err(AssetReferenceError::ExpiredHandle);
        }
        self.validate_class(entry.media_kind.media_class())?;
        if !scope.observes(entry.visibility) {
            return Err(AssetReferenceError::ExcludedByScope);
        }
        let declared = entry
            .media_properties
            .value()
            .map(super::super::AssetMediaProperties::media_type);
        if let (Some(requested), Some(declared)) =
            (request.requested_media_type.as_deref(), declared)
            && requested != declared
        {
            return Err(AssetReferenceError::UnsupportedMediaType(
                requested.to_owned(),
            ));
        }
        enforce_limits(entry, request)?;
        let availability = availability_of(entry);
        let payload = match availability {
            AssetRenditionAvailability::Available => self
                .catalog
                .renditions
                .get(key)
                .cloned()
                .ok_or(AssetReferenceError::RetrievalUnavailable)?,
            _ => AssetRenditionPayload::MetadataOnly,
        };
        Ok(AssetRendition {
            handle: entry.handle.clone(),
            availability,
            payload,
            authority: AssetRenditionAuthority::NotGranted,
        })
    }

    /// Refuses a read whose stated revision is not this snapshot's revision.
    fn validate_revision(
        &self,
        revision: Option<&AssetRevision>,
    ) -> Result<(), AssetReferenceError> {
        match revision {
            None => Ok(()),
            Some(revision) if *revision == self.catalog.revision() => Ok(()),
            Some(_) => Err(AssetReferenceError::ContentRevisionMismatch),
        }
    }

    /// Refuses an asset in a media class the source does not project.
    fn validate_class(&self, class: AssetMediaClass) -> Result<(), AssetReferenceError> {
        let coverage = self
            .catalog
            .class_state(class)
            .ok_or(AssetReferenceError::ClassCoverageIncomplete)?;
        if matches!(coverage.state, super::super::AssetClassState::Projected) {
            Ok(())
        } else {
            Err(AssetReferenceError::UnavailableClass)
        }
    }

    fn take_cursor(&mut self, query: &AssetListQuery) -> Result<usize, AssetReferenceError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !Arc::ptr_eq(&continuation.scope, &self.scope) {
            return Err(AssetReferenceError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(AssetReferenceError::InvalidContinuation)?;
        if cursor.binding != self.catalog.binding
            || cursor.locale != query.locale
            || cursor.scope != query.scope
            || cursor.media_kind != query.media_kind
            || cursor.retrieval_state != query.retrieval_state
            || cursor.limit != query.limit
        {
            return Err(AssetReferenceError::InvalidContinuation);
        }
        Ok(cursor.offset)
    }

    fn make_token(&mut self) -> String {
        let token = format!("asset-cursor-{:08}", self.next_cursor);
        self.next_cursor = self.next_cursor.saturating_add(1);
        token
    }
}

/// Enforces the request's bounds against the asset's retained declared properties.
fn enforce_limits(
    entry: &AssetEntry,
    request: &AssetRenditionRequest,
) -> Result<(), AssetReferenceError> {
    let limits = request.limits;
    if let Some(size) = entry.byte_size.value() {
        if size.stored_bytes() > limits.max_bytes() {
            return Err(AssetReferenceError::OversizedRendition {
                limit: limits.max_bytes(),
                actual: size.stored_bytes(),
            });
        }
        if size.decoded_bytes() > limits.max_decoded_bytes() {
            return Err(AssetReferenceError::InflatedRendition {
                limit: limits.max_decoded_bytes(),
                actual: size.decoded_bytes(),
            });
        }
        if size.decode_ratio() > limits.max_decode_ratio() {
            return Err(AssetReferenceError::InflatedRendition {
                limit: limits.max_decode_ratio(),
                actual: size.decode_ratio(),
            });
        }
    }
    if let Some(properties) = entry.media_properties.value() {
        if let Some(dimension) = properties.max_dimension()
            && dimension > limits.max_dimension()
        {
            return Err(AssetReferenceError::ExcessiveDimensions {
                limit: limits.max_dimension(),
                actual: dimension,
            });
        }
        if let Some(duration) = properties.duration_ms()
            && duration > limits.max_duration_ms()
        {
            return Err(AssetReferenceError::ExcessiveDuration {
                limit: limits.max_duration_ms(),
                actual: duration,
            });
        }
    }
    Ok(())
}
