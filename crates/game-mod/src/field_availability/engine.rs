// SPDX-License-Identifier: MIT

use super::helpers::{
    aggregate_status, build_summary, field_status, result_for_fixture, unavailable,
    validate_fixture,
};
use super::*;
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Debug)]
struct KindState {
    schema: LocalKindSchema,
    entities: Vec<LocalEntityFixture>,
    total_known: bool,
}

#[derive(Clone, Debug)]
struct CursorState {
    reference: LocalReadReference,
    entity_kind: String,
    fields: Vec<String>,
    next_index: usize,
}

/// Host-independent owner-local bounded read engine for synthetic contract tests.
#[derive(Debug)]
pub struct LocalAvailabilityStore {
    reference: LocalReadReference,
    max_page_items: usize,
    max_detail_bytes: usize,
    kinds: BTreeMap<String, KindState>,
    cursors: BTreeMap<String, CursorState>,
    next_cursor: u64,
    scope: Arc<LocalContinuationScope>,
}

impl LocalAvailabilityStore {
    /// Creates an engine with explicit page and detail bounds.
    pub fn new(
        reference: LocalReadReference,
        max_page_items: usize,
        max_detail_bytes: usize,
    ) -> Result<Self, LocalReadError> {
        if max_page_items == 0 || max_detail_bytes == 0 {
            return Err(LocalReadError::InvalidPageSize);
        }
        Ok(Self {
            reference,
            max_page_items,
            max_detail_bytes,
            kinds: BTreeMap::new(),
            cursors: BTreeMap::new(),
            next_cursor: 0,
            scope: Arc::new(LocalContinuationScope),
        })
    }

    /// Returns the identity all current reads must repeat.
    #[must_use]
    pub fn reference(&self) -> &LocalReadReference {
        &self.reference
    }

    /// Replaces the synthetic observation and expires all outstanding cursors.
    pub fn replace_reference(&mut self, reference: LocalReadReference) {
        if self.reference != reference {
            self.reference = reference;
            self.cursors.clear();
        }
    }

    /// Registers one allowlisted entity kind.
    pub fn add_kind(&mut self, fixture: LocalKindFixture) -> Result<(), LocalReadError> {
        validate_fixture(&fixture)?;
        let mut entities = fixture.entities;
        entities.sort_by(|left, right| left.entity_id.cmp(&right.entity_id));
        let kind = fixture.schema.kind.clone();
        if self.kinds.contains_key(&kind) {
            self.reference.epoch = self
                .reference
                .epoch
                .checked_add(1)
                .ok_or(LocalReadError::StaleReference)?;
            self.cursors.clear();
        }
        self.kinds.insert(
            kind,
            KindState {
                schema: fixture.schema,
                entities,
                total_known: fixture.total_known,
            },
        );
        Ok(())
    }

    /// Reports allowlisted fields and required-field coverage without reading host objects.
    pub fn support_report(&self, entity_kind: &str) -> Result<LocalSupportReport, LocalReadError> {
        let state = self
            .kinds
            .get(entity_kind)
            .ok_or(LocalReadError::UnknownKind)?;
        let fields = state
            .schema
            .fields
            .values()
            .map(|definition| LocalFieldSupport {
                field: definition.name.clone(),
                value_kind: definition.value_kind,
                required: definition.required,
                basic: definition.basic,
                supported: definition.supported,
                protected: definition.protected,
                detail_group: definition.detail_group.clone(),
            })
            .collect::<Vec<_>>();
        let required_fields = state
            .schema
            .fields
            .values()
            .filter(|definition| definition.required)
            .map(|definition| {
                let statuses = state.entities.iter().map(|entity| {
                    field_status(
                        definition,
                        entity.values.get(&definition.name),
                        definition.basic,
                    )
                });
                let status = aggregate_status(statuses);
                LocalRequiredFieldCoverage {
                    field: definition.name.clone(),
                    entity_count: state.entities.len(),
                    available_count: state
                        .entities
                        .iter()
                        .filter(|entity| {
                            field_status(
                                definition,
                                entity.values.get(&definition.name),
                                definition.basic,
                            ) == LocalFieldStatus::Available
                        })
                        .count(),
                    status,
                }
            })
            .collect::<Vec<_>>();
        Ok(LocalSupportReport {
            entity_kind: entity_kind.to_owned(),
            count_known: state.total_known,
            entity_count: state.entities.len(),
            fields,
            required_fields,
        })
    }

    /// Reads one bounded basic page and emits opaque links for supported detail groups.
    pub fn read_basic(
        &mut self,
        query: &LocalBasicQuery,
    ) -> Result<LocalCollectionPage, LocalReadError> {
        if query.limit == 0 || query.limit > self.max_page_items {
            return Err(LocalReadError::InvalidPageSize);
        }
        let state = self
            .kinds
            .get(&query.entity_kind)
            .cloned()
            .ok_or(LocalReadError::UnknownKind)?;
        let fields = if query.fields.is_empty() {
            state.schema.fields.keys().cloned().collect()
        } else {
            for field in &query.fields {
                let definition = state
                    .schema
                    .fields
                    .get(field)
                    .ok_or(LocalReadError::UnknownField)?;
                if definition.protected {
                    return Err(LocalReadError::DeniedField);
                }
            }
            query.fields.clone()
        };
        let start = self.cursor_start(query, &fields)?;
        let end = start.saturating_add(query.limit).min(state.entities.len());
        let origin = LocalFieldOrigin::new(query.entity_kind.clone(), self.reference.clone());
        let entries = state.entities[start..end]
            .iter()
            .map(|entity| build_summary(&state.schema, entity, &fields, &origin))
            .collect::<Vec<_>>();
        let continuation = if end < state.entities.len() {
            let token = format!("local-cursor-{:08}", self.next_cursor);
            self.next_cursor = self.next_cursor.saturating_add(1);
            self.cursors.insert(
                token.clone(),
                CursorState {
                    reference: self.reference.clone(),
                    entity_kind: query.entity_kind.clone(),
                    fields,
                    next_index: end,
                },
            );
            Some(LocalContinuation::scoped(token, Arc::clone(&self.scope)))
        } else {
            None
        };
        Ok(LocalCollectionPage {
            entity_kind: query.entity_kind.clone(),
            entries,
            total: state.total_known.then_some(state.entities.len()),
            completeness: if continuation.is_some() {
                LocalCompleteness::Partial
            } else {
                LocalCompleteness::Complete
            },
            continuation,
            origin,
        })
    }

    /// Recovers one allowlisted detail group under the exact originating identity.
    pub fn read_detail(
        &self,
        link: &LocalDetailLink,
    ) -> Result<LocalDetailResponse, LocalReadError> {
        if link.origin.reference != self.reference || link.origin.source_kind != link.entity_kind {
            return Err(LocalReadError::StaleReference);
        }
        let state = self
            .kinds
            .get(&link.entity_kind)
            .ok_or(LocalReadError::UnknownKind)?;
        let group_fields = state
            .schema
            .groups
            .get(&link.field_group)
            .ok_or(LocalReadError::UnknownFieldGroup)?;
        let entity = state
            .entities
            .iter()
            .find(|entity| entity.entity_id == link.entity_id)
            .ok_or(LocalReadError::EntityNotFound)?;
        let estimated_bytes = entity
            .detail_bytes
            .get(&link.field_group)
            .copied()
            .filter(|bytes| *bytes > 0)
            .ok_or(LocalReadError::DetailSizeUnavailable)?;
        if estimated_bytes > self.max_detail_bytes {
            return Err(LocalReadError::DetailTooLarge {
                limit: self.max_detail_bytes,
                actual: estimated_bytes,
            });
        }
        let origin = LocalFieldOrigin::new(link.entity_kind.clone(), self.reference.clone());
        let fields = group_fields
            .iter()
            .filter_map(|field| state.schema.fields.get(field))
            .map(|definition| {
                let result = if definition.protected {
                    unavailable(
                        LocalFieldStatus::Denied,
                        LocalReasonCode::ScopeDenied,
                        &origin,
                    )
                } else {
                    result_for_fixture(
                        definition,
                        entity.values.get(&definition.name),
                        &origin,
                        true,
                    )
                };
                (definition.name.clone(), result)
            })
            .collect::<BTreeMap<_, _>>();
        Ok(LocalDetailResponse {
            entity_kind: link.entity_kind.clone(),
            entity_id: link.entity_id.clone(),
            field_group: link.field_group.clone(),
            fields,
            estimated_bytes,
            origin,
        })
    }

    fn cursor_start(
        &mut self,
        query: &LocalBasicQuery,
        fields: &[String],
    ) -> Result<usize, LocalReadError> {
        let Some(continuation) = &query.continuation else {
            return Ok(0);
        };
        if !continuation.scope_matches(&self.scope) {
            return Err(LocalReadError::InvalidContinuation);
        }
        let cursor = self
            .cursors
            .remove(continuation.token())
            .ok_or(LocalReadError::InvalidContinuation)?;
        if cursor.reference != self.reference {
            return Err(LocalReadError::StaleReference);
        }
        if cursor.entity_kind != query.entity_kind || cursor.fields != fields {
            return Err(LocalReadError::InvalidContinuation);
        }
        Ok(cursor.next_index)
    }
}
