// SPDX-License-Identifier: MIT

use std::sync::Arc;

use super::super::{
    SettingsCatalogBinding, SettingsCategory, SettingsDefinitionReference, SettingsFieldStatus,
    SettingsLevel, SettingsSensitivity, SettingsText, SettingsValueType, SettingsVisibility,
    SettingsVisibilityScope,
};

#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct ContinuationScope;

/// Opaque single-use settings-list continuation.
///
/// The value is cheaply clonable, but the retained token is single-use: the reader removes it on
/// first consumption, so a reused clone is rejected as an invalid continuation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SettingsContinuation {
    token: String,
    pub(super) scope: Arc<ContinuationScope>,
}

impl SettingsContinuation {
    pub(super) fn new(token: String, scope: Arc<ContinuationScope>) -> Self {
        Self { token, scope }
    }

    /// Returns the opaque fixture token.
    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }
}

/// Bounded settings page request.
#[derive(Debug, Eq, PartialEq)]
pub struct SettingsListQuery {
    /// Locale expected by the caller.
    pub locale: String,
    /// Optional category filter.
    pub category: Option<SettingsCategory>,
    /// Optional stored-scope filter.
    pub level: Option<SettingsLevel>,
    /// Visibility scope.
    pub scope: SettingsVisibilityScope,
    /// Maximum entries in one page.
    pub limit: usize,
    /// Single-use continuation from a previous page.
    pub continuation: Option<SettingsContinuation>,
}

/// Typed summary returned by one bounded settings page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsDefinitionSummary {
    /// Exact static definition reference.
    pub reference: SettingsDefinitionReference,
    /// Localized setting label or explicit unavailable state.
    pub label: SettingsText,
    /// Owner-defined category.
    pub category: SettingsCategory,
    /// Scope the preference is stored at.
    pub level: SettingsLevel,
    /// Declared value shape.
    pub value_type: SettingsValueType,
    /// Whether the setting may be discovered publicly.
    pub sensitivity: SettingsSensitivity,
    /// Owner-defined visibility.
    pub visibility: SettingsVisibility,
    /// Availability of the shipped default value.
    pub default_value: SettingsFieldStatus,
    /// Availability of the stored preference value.
    pub stored_value: SettingsFieldStatus,
    /// Availability of the runtime-effective value.
    pub effective_value: SettingsFieldStatus,
    /// Availability of the run-configuration link.
    pub run_link: SettingsFieldStatus,
}

/// Complete or partial settings page.
#[derive(Debug, Eq, PartialEq)]
pub struct SettingsDefinitionPage {
    /// Catalog witness for every entry.
    pub binding: SettingsCatalogBinding,
    /// Deterministically ordered summaries.
    pub entries: Vec<SettingsDefinitionSummary>,
    /// Number of visible settings.
    pub total: usize,
    /// Whether no continuation remains.
    pub complete: bool,
    /// Present only when the page is partial.
    pub continuation: Option<SettingsContinuation>,
}

#[derive(Clone, Debug)]
pub(super) struct SettingsCursorState {
    pub(super) binding: SettingsCatalogBinding,
    pub(super) locale: String,
    pub(super) category: Option<SettingsCategory>,
    pub(super) level: Option<SettingsLevel>,
    pub(super) scope: SettingsVisibilityScope,
    pub(super) limit: usize,
    pub(super) offset: usize,
}
