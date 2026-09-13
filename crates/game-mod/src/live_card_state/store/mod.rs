// SPDX-License-Identifier: MIT

use std::{collections::BTreeMap, sync::Arc};

use super::model::{LiveCardContinuationScope, LiveCardDetail};
use super::{
    CardCost, CardCostAmount, CardCostContributor, CardExpiration, CardInstanceReference,
    CardLocation, CardModifier, CardModifierValue, CardPile, LIVE_CARD_MAX_DETAIL_BYTES,
    LIVE_CARD_MAX_EFFECT_OVERRIDES, LIVE_CARD_MAX_ID_BYTES, LIVE_CARD_MAX_MODIFIERS,
    LIVE_CARD_MAX_PAGE_ITEMS, LIVE_CARD_MAX_TEXT_BYTES, LiveCardCollection, LiveCardContinuation,
    LiveCardError, LiveCardField, LiveCardFixture, LiveCardPage, LiveCardPageCompleteness,
    LiveCardProjection, LiveCardQuery, LiveCardReadReference, LiveCardSnapshot, LiveCardValue,
};
mod validation;

use validation::{
    collection_matches, validate_collection, validate_fixture_for_read, validate_snapshot,
    value_for_field,
};

#[derive(Clone, Debug)]
struct CursorState {
    reference: LiveCardReadReference,
    collection: LiveCardCollection,
    instance_ids: Vec<String>,
    next_index: usize,
}

/// Host-independent owner-local store for bounded live card projections.
#[derive(Debug)]
pub struct LiveCardStore {
    reference: LiveCardReadReference,
    cards: Vec<LiveCardFixture>,
    total_known: bool,
    max_page_items: usize,
    max_detail_bytes: usize,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<LiveCardContinuationScope>,
}

impl LiveCardStore {
    /// Adopts an owned snapshot under explicit page and detail bounds.
    pub fn new(
        snapshot: LiveCardSnapshot,
        max_page_items: usize,
        max_detail_bytes: usize,
    ) -> Result<Self, LiveCardError> {
        if max_page_items == 0 || max_page_items > LIVE_CARD_MAX_PAGE_ITEMS {
            return Err(LiveCardError::InvalidPageSize);
        }
        if max_detail_bytes == 0 || max_detail_bytes > LIVE_CARD_MAX_DETAIL_BYTES {
            return Err(LiveCardError::DetailTooLarge {
                limit: LIVE_CARD_MAX_DETAIL_BYTES,
                actual: max_detail_bytes,
            });
        }
        validate_snapshot(&snapshot)?;
        Ok(Self {
            reference: snapshot.reference,
            cards: snapshot.cards,
            total_known: snapshot.total_known,
            max_page_items,
            max_detail_bytes,
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(LiveCardContinuationScope),
        })
    }

    /// Reads and adopts a snapshot from a source, preserving fail-closed errors.
    pub fn from_source<S: super::LiveCardSource>(
        source: &S,
        max_page_items: usize,
        max_detail_bytes: usize,
    ) -> Result<Self, LiveCardError> {
        if let super::LiveCardCapability::Unavailable(reason) = source.capability() {
            return Err(LiveCardError::Unavailable(reason));
        }
        let snapshot = source.read_snapshot()?;
        Self::new(snapshot, max_page_items, max_detail_bytes)
    }

    /// Returns the fence shared by all current projections.
    #[must_use]
    pub fn reference(&self) -> &LiveCardReadReference {
        &self.reference
    }

    /// Replaces the coherent source snapshot and expires every older page/detail reference.
    pub fn replace_snapshot(&mut self, snapshot: LiveCardSnapshot) -> Result<(), LiveCardError> {
        if snapshot.reference.epoch <= self.reference.epoch {
            return Err(LiveCardError::NonMonotonicEpoch {
                current: self.reference.epoch,
                supplied: snapshot.reference.epoch,
            });
        }
        validate_snapshot(&snapshot)?;
        self.reference = snapshot.reference;
        self.cards = snapshot.cards;
        self.total_known = snapshot.total_known;
        self.cursors.clear();
        Ok(())
    }

    /// Reads one bounded page in source order.
    pub fn read_page(&mut self, query: &LiveCardQuery) -> Result<LiveCardPage, LiveCardError> {
        if query.limit == 0 || query.limit > self.max_page_items {
            return Err(LiveCardError::InvalidPageSize);
        }
        validate_collection(&query.collection)?;
        let matching_indices = self.matching_indices(&query.collection);
        let matching_ids = matching_indices
            .iter()
            .map(|index| self.cards[*index].projection.instance_id().to_owned())
            .collect::<Vec<_>>();
        let start = self.cursor_start(query, &matching_ids)?;
        let end = start
            .saturating_add(query.limit)
            .min(matching_indices.len());
        let indices = &matching_indices[start..end];
        let mut entries = Vec::with_capacity(indices.len());
        for index in indices {
            let fixture = &self.cards[*index];
            validate_fixture_for_read(fixture, self.max_detail_bytes)?;
            if let Some(field) = fixture.unsupported_fields.iter().next() {
                return Err(LiveCardError::UnsupportedField(*field));
            }
            entries.push(fixture.projection.clone());
        }

        let continuation = if end < matching_indices.len() {
            let token = format!("live-card-cursor-{:08}", self.next_cursor);
            self.next_cursor = self.next_cursor.saturating_add(1);
            self.cursors.insert(
                token.clone(),
                CursorState {
                    reference: self.reference.clone(),
                    collection: query.collection.clone(),
                    instance_ids: matching_ids,
                    next_index: end,
                },
            );
            Some(LiveCardContinuation::scoped(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(LiveCardPage {
            collection: query.collection.clone(),
            entries,
            total: self.total_known.then_some(matching_indices.len()),
            completeness: if continuation.is_some() {
                LiveCardPageCompleteness::Partial
            } else {
                LiveCardPageCompleteness::Complete
            },
            continuation,
            reference: self.reference.clone(),
        })
    }

    /// Reads all supported fields for one stable instance reference.
    pub fn read_card(
        &self,
        instance: &CardInstanceReference,
    ) -> Result<LiveCardProjection, LiveCardError> {
        let detail = self.read_detail(instance, LiveCardField::all())?;
        let fixture = self.find_instance(instance)?;
        validate_fixture_for_read(fixture, self.max_detail_bytes)?;
        if detail.fields.len() != LiveCardField::all().len() {
            return Err(LiveCardError::InvalidProjection(
                "complete card detail did not contain every field",
            ));
        }
        Ok(fixture.projection.clone())
    }

    /// Reads an allowlisted field subset without changing gameplay state.
    pub fn read_detail(
        &self,
        instance: &CardInstanceReference,
        fields: &[LiveCardField],
    ) -> Result<LiveCardDetail, LiveCardError> {
        let fixture = self.find_instance(instance)?;
        validate_fixture_for_read(fixture, self.max_detail_bytes)?;
        let requested = if fields.is_empty() {
            LiveCardField::all().to_vec()
        } else {
            fields.to_vec()
        };
        let mut values = BTreeMap::new();
        for field in requested {
            if fixture.unsupported_fields.contains(&field) {
                return Err(LiveCardError::UnsupportedField(field));
            }
            if values
                .insert(field, value_for_field(&fixture.projection, field))
                .is_some()
            {
                return Err(LiveCardError::InvalidProjection(
                    "duplicate card detail field",
                ));
            }
        }
        Ok(LiveCardDetail {
            instance: instance.clone(),
            fields: values,
            reference: self.reference.clone(),
        })
    }

    fn find_instance(
        &self,
        instance: &CardInstanceReference,
    ) -> Result<&LiveCardFixture, LiveCardError> {
        if instance.read != self.reference {
            return Err(LiveCardError::StaleReference);
        }
        self.cards
            .iter()
            .find(|fixture| fixture.projection.instance.instance_id == instance.instance_id)
            .ok_or(LiveCardError::InstanceNotFound)
    }

    fn matching_indices(&self, collection: &LiveCardCollection) -> Vec<usize> {
        self.cards
            .iter()
            .enumerate()
            .filter_map(|(index, fixture)| {
                collection_matches(collection, &fixture.projection.location).then_some(index)
            })
            .collect()
    }

    fn cursor_start(
        &mut self,
        query: &LiveCardQuery,
        instance_ids: &[String],
    ) -> Result<usize, LiveCardError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !continuation.scope_matches(&self.scope) {
            return Err(LiveCardError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(LiveCardError::InvalidContinuation)?;
        if cursor.reference != self.reference
            || cursor.collection != query.collection
            || cursor.instance_ids != instance_ids
        {
            return Err(LiveCardError::StaleReference);
        }
        Ok(cursor.next_index)
    }
}
