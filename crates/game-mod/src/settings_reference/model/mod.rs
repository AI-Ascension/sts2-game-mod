// SPDX-License-Identifier: MIT

mod reference;
mod value;

pub use reference::*;
pub use value::*;

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const SETTINGS_REFERENCE_ENTITY_KIND: &str = "setting";
/// Manifest family resolved for run-affecting setting links.
pub const SETTINGS_REFERENCE_RUN_KIND: &str = "run_configuration";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const SETTINGS_REFERENCE_PRODUCER_VERSION: &str = "game-settings-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const SETTINGS_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const SETTINGS_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one setting definition.
pub const SETTINGS_MAX_DEFINITION_BYTES: usize = 64 * 1024;
/// Maximum setting definitions in one source snapshot.
pub const SETTINGS_MAX_DEFINITIONS: usize = 2_048;
/// Maximum entries returned by one bounded definition page.
pub const SETTINGS_MAX_PAGE_ITEMS: usize = 64;
/// Maximum constraints attached to one setting definition.
pub const SETTINGS_MAX_CONSTRAINTS: usize = 32;
/// Maximum options accepted in one enumeration constraint.
pub const SETTINGS_MAX_OPTIONS: usize = 256;
/// Maximum keys retained by one key-binding value.
pub const SETTINGS_MAX_KEYS: usize = 16;
/// Maximum semantic references on one setting definition.
pub const SETTINGS_MAX_REFERENCES: usize = 32;

/// Owner-defined setting category.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsCategory {
    /// Language and text-rendering preferences.
    Language,
    /// Accessibility preferences.
    Accessibility,
    /// Input and key-binding preferences.
    Input,
    /// Display and window preferences.
    Display,
    /// Audio preferences.
    Audio,
    /// Gameplay-interaction preferences.
    GameplayInteraction,
    /// The source could not classify the category.
    Unknown,
}

/// Scope a preference is stored at.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsLevel {
    /// One global value shared by every profile.
    Global,
    /// One value stored per player profile.
    Profile,
    /// One value stored by an addon package.
    Addon,
    /// The source could not classify the stored scope.
    Unknown,
}

/// Typed value shape of one setting.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsValueType {
    /// Boolean toggle.
    Boolean,
    /// Bounded integer.
    Integer,
    /// One option from a closed enumeration.
    Enumeration,
    /// One or more input keys.
    KeyBinding,
    /// Bounded free text.
    Text,
    /// The source could not type the value.
    Unknown,
}

/// Whether a stored change needs a restart before it takes effect.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsRestartState {
    /// The effective value follows the stored value without a restart.
    NotRequired,
    /// The stored value takes effect only after a restart.
    Required,
    /// The source could not classify the restart requirement.
    Unknown,
}

/// Owner-defined visibility of one setting.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// The source knows the setting exists but must not reveal it.
    Hidden,
    /// The source could not classify visibility.
    Unknown,
}

/// Scope requested by a settings query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsVisibilityScope {
    /// Publicly visible settings only; private settings are excluded.
    Public,
    /// Public settings plus owner-only settings, still without private values.
    Reference,
    /// Explicit owner-authorized scope, including private settings.
    Owner,
}

/// Whether a setting is safe for public discovery.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsSensitivity {
    /// The setting may be discovered and read in a public scope.
    Public,
    /// The setting covers credentials, private endpoints, or operator secrets.
    Private,
}

/// Supported owner read seam a value was copied through.
///
/// This is the only admitted provenance: a value read from an arbitrary config-file path or
/// through reflection is not representable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsReadSeam {
    /// Owner-provided settings API.
    OwnerSettingsApi,
    /// Owner-provided per-profile preference API.
    ProfilePreferenceApi,
    /// Owner-provided addon settings API.
    AddonSettingsApi,
    /// The source could not classify the read seam.
    Unknown,
}

/// Owner identity of the profile the snapshot was read under.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SettingsProfile {
    /// Opaque profile identity supplied by the owner.
    pub profile_id: String,
    /// Whether the profile is the default, a named profile, or an addon scope.
    pub kind: SettingsProfileKind,
}

/// Kind of profile a settings snapshot was read under.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsProfileKind {
    /// The owner default profile.
    Default,
    /// A named player profile.
    Named,
    /// An addon-scoped profile.
    Addon,
    /// The source could not classify the profile kind.
    Unknown,
}

/// Static catalog identity: content manifest, locale, profile, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SettingsCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized setting value.
    pub locale: String,
    /// Profile selected when the values were read.
    pub profile: SettingsProfile,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static setting definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SettingsDefinitionReference {
    /// Catalog witness that owns this setting identity.
    pub catalog: SettingsCatalogBinding,
    /// Namespaced setting definition identity.
    pub setting_id: String,
}

/// Source support state for the settings family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettingsFamilyState {
    /// Source can project every setting definition.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SettingsFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: SettingsFamilyState,
    /// Number of setting definitions in the manifest.
    pub definition_count: usize,
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::SettingsReferenceError> {
    if value.is_empty()
        || value.len() > SETTINGS_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::SettingsReferenceError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::SettingsReferenceError> {
    if value.is_empty()
        || value.len() > SETTINGS_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(super::SettingsReferenceError::InvalidInput(field));
    }
    Ok(())
}
