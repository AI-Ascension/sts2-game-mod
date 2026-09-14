// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::ContentUnlockState;

use super::super::ActText;
use super::super::{
    ActCatalogBinding, ActDefinitionReference, ActEncounterReference, ActFieldStatus,
    ActVisibilityScope, EncounterKind, GenerationWeight,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use act-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it
/// on first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ActContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Opaque single-use encounter-list continuation.
///
/// Like [`ActContinuation`], a clone is rejected after the token is consumed once.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActEncounterContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl ActEncounterContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded act page request.
#[derive(Debug, Eq, PartialEq)]
pub struct ActListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Visibility scope.
    pub scope: ActVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ActContinuation>,
}

/// Typed summary returned by one bounded act page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActDefinitionSummary {
    /// Exact static definition reference.
    pub reference: ActDefinitionReference,
    /// Localized act name or explicit unavailable state.
    pub name: ActText,
    /// Owner-defined act order.
    pub order: u16,
    /// Explicit unlock state.
    pub unlock_state: ContentUnlockState,
    /// Number of room/node categories.
    pub room_category_count: usize,
    /// Number of encounter definitions.
    pub encounter_count: usize,
    /// Availability of encounter pools.
    pub pools: ActFieldStatus,
    /// Availability of map-generation constraints.
    pub constraints: ActFieldStatus,
}

/// Complete or partial act definition page.
#[derive(Debug, Eq, PartialEq)]
pub struct ActDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: ActCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<ActDefinitionSummary>,
    /// Number of visible acts.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ActContinuation>,
}

/// Bounded encounter page request scoped to one exact act definition.
#[derive(Debug, Eq, PartialEq)]
pub struct ActEncounterListQuery {
    /// Exact act definition whose encounters are listed.
    pub act: ActDefinitionReference,
    /// Optional encounter-category filter.
    pub kind: Option<EncounterKind>,
    /// Visibility scope.
    pub scope: ActVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<ActEncounterContinuation>,
}

/// Typed summary returned by one bounded encounter page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActEncounterSummary {
    /// Exact static encounter reference.
    pub reference: ActEncounterReference,
    /// Localized encounter name.
    pub name: ActText,
    /// Encounter category.
    pub kind: EncounterKind,
    /// Number of enemy groups.
    pub group_count: usize,
    /// Total number of enemy slots across groups.
    pub enemy_count: usize,
    /// Availability of eligibility conditions.
    pub eligibility: ActFieldStatus,
    /// Generation weight with evidence.
    pub weight: GenerationWeight,
    /// Optional room/node category identity.
    pub room_category_id: Option<String>,
}

/// Complete or partial encounter page.
#[derive(Debug, Eq, PartialEq)]
pub struct ActEncounterDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: ActCatalogBinding,
    /// Deterministically ordered encounter summaries.
    pub entries: Vec<ActEncounterSummary>,
    /// Number of visible encounters.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<ActEncounterContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct ActCursorState {
    pub(super) binding: ActCatalogBinding,
    pub(super) locale: String,
    pub(super) scope: ActVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct EncounterCursorState {
    pub(super) binding: ActCatalogBinding,
    pub(super) act_id: String,
    pub(super) kind: Option<EncounterKind>,
    pub(super) scope: ActVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
