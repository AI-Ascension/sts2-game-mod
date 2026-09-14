// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::ContentUnlockState;

use super::super::{
    EnemyCatalogBinding, EnemyDefinitionReference, EnemyFieldStatus, EnemyKind, EnemyMoveReference,
    EnemyProbability, EnemyText, EnemyVisibilityScope,
};

/// Typed summary returned by one bounded enemy definition page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyDefinitionSummary {
    /// Exact static definition reference.
    pub reference: EnemyDefinitionReference,
    /// Localized enemy name or explicit unavailable state.
    pub name: EnemyText,
    /// Enemy role.
    pub kind: EnemyKind,
    /// Explicit unlock state.
    pub unlock_state: ContentUnlockState,
    /// Number of source-owned moves.
    pub move_count: usize,
    /// Number of behavior phases.
    pub phase_count: usize,
    /// Number of stable tags.
    pub tag_count: usize,
    /// Availability of spawn conditions.
    pub spawn_conditions: EnemyFieldStatus,
    /// Availability of encounter references.
    pub encounters: EnemyFieldStatus,
}

/// Bounded enemy page request.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: EnemyVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<EnemyContinuation>,
}

/// Complete or partial enemy definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: EnemyCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<EnemyDefinitionSummary>,
    /// Number of visible definitions.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<EnemyContinuation>,
}

/// Bounded move page request scoped to one exact enemy definition.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyMoveListQuery {
    /// Exact enemy definition whose moves are listed.
    pub enemy: EnemyDefinitionReference,
    /// Visibility scope.
    pub scope: EnemyVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<EnemyMoveContinuation>,
}

/// Summary returned by one bounded move page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyMoveSummary {
    /// Exact static move reference.
    pub reference: EnemyMoveReference,
    /// Localized move name.
    pub name: EnemyText,
    /// Number of ordered effects.
    pub effect_count: usize,
    /// Phase identities in which the move is available.
    pub phase_ids: Vec<String>,
    /// Selection probability/weight with explicit evidence.
    pub probability: EnemyProbability,
}

/// Complete or partial move page.
#[derive(Debug, Eq, PartialEq)]
pub struct EnemyMoveDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: EnemyCatalogBinding,
    /// Deterministically ordered move summaries.
    pub entries: Vec<EnemyMoveSummary>,
    /// Number of visible moves.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<EnemyMoveContinuation>,
}

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use enemy continuation.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl EnemyContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use move continuation.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyMoveContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl EnemyMoveContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Clone, Debug)]
pub(super) struct EnemyCursorState {
    pub(super) binding: EnemyCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: EnemyVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct MoveCursorState {
    pub(super) binding: EnemyCatalogBinding,
    pub(super) enemy_id: String,
    pub(super) scope: EnemyVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
