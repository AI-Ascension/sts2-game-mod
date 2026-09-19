// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::{
    binding::{RunConfigurationBinding, RunConfigurationCacheKey, RunConfigurationLiveBinding},
    definition::{RunConfigurationCompleteness, RunConfigurationDefinitionReference},
    field::RunConfigurationFieldStatus,
    model::{RunMode, RunSeedPolicy, RunVisibilityScope},
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use run-configuration-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RunConfigurationContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl RunConfigurationContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded run-configuration page request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Optional settled-revision filter.
    pub revision: Option<u64>,
    /// Optional admitted-mode filter.
    pub mode: Option<RunMode>,
    /// Visibility scope.
    pub scope: RunVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<RunConfigurationContinuation>,
}

/// Typed summary returned by one bounded run-configuration page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationSummary {
    /// Exact static definition reference.
    pub reference: RunConfigurationDefinitionReference,
    /// Live witness for the run.
    pub live: RunConfigurationLiveBinding,
    /// Seed visibility policy applied to the run.
    pub seed_policy: RunSeedPolicy,
    /// Availability of the settled mode.
    pub mode: RunConfigurationFieldStatus,
    /// Availability of the settled difficulty.
    pub difficulty: RunConfigurationFieldStatus,
    /// Availability of the settled character.
    pub character: RunConfigurationFieldStatus,
    /// Number of modifiers the host reports as active.
    pub active_modifiers: usize,
    /// Whether every required field carries a settled host value.
    pub completeness: RunConfigurationCompleteness,
    /// Cache key that includes seed material when the policy makes the seed visible.
    pub cache: RunConfigurationCacheKey,
    /// Cache key computed without seed material.
    pub seed_blind_cache: RunConfigurationCacheKey,
}

/// Complete or partial run-configuration page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunConfigurationPage {
    /// Catalog witness for every entry.
    pub binding: RunConfigurationBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<RunConfigurationSummary>,
    /// Number of visible runs.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<RunConfigurationContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct RunConfigurationCursorState {
    pub(super) binding: RunConfigurationBinding,
    pub(super) locale: String,
    pub(super) revision: Option<u64>,
    pub(super) mode: Option<RunMode>,
    pub(super) scope: RunVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
