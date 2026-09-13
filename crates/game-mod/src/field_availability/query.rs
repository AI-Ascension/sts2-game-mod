// SPDX-License-Identifier: MIT

use super::*;

/// Bounded basic collection request for the owner-local read engine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalBasicQuery {
    /// Owner-defined entity kind.
    pub entity_kind: String,
    /// Exact allowlisted fields; empty means all declared fields.
    pub fields: Vec<String>,
    /// Maximum entries requested for this page.
    pub limit: usize,
    /// Opaque continuation returned by an earlier page.
    pub continuation: Option<LocalContinuation>,
}

impl LocalBasicQuery {
    /// Creates a bounded basic query.
    #[must_use]
    pub fn new(
        entity_kind: impl Into<String>,
        fields: impl IntoIterator<Item = impl Into<String>>,
        limit: usize,
        continuation: Option<LocalContinuation>,
    ) -> Self {
        Self {
            entity_kind: entity_kind.into(),
            fields: fields.into_iter().map(Into::into).collect(),
            limit,
            continuation,
        }
    }

    /// Creates a query for every declared field on the basic surface.
    #[must_use]
    pub fn all(entity_kind: impl Into<String>, limit: usize) -> Self {
        Self::new(entity_kind, std::iter::empty::<String>(), limit, None)
    }
}

/// Synthetic kind data used by source-only deterministic tests.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalKindFixture {
    /// Allowlisted schema.
    pub schema: LocalKindSchema,
    /// Synthetic entities in arbitrary input order.
    pub entities: Vec<LocalEntityFixture>,
    /// Whether the source can report a collection total.
    pub total_known: bool,
}

impl LocalKindFixture {
    /// Creates synthetic kind data. Schema and value consistency is checked on registration.
    #[must_use]
    pub fn new(
        schema: LocalKindSchema,
        entities: impl IntoIterator<Item = LocalEntityFixture>,
        total_known: bool,
    ) -> Self {
        Self {
            schema,
            entities: entities.into_iter().collect(),
            total_known,
        }
    }
}
