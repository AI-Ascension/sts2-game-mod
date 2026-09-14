// SPDX-License-Identifier: MIT

mod value;

pub use value::*;

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const ENEMY_ENTITY_KIND: &str = "enemy";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const ENEMY_PRODUCER_VERSION: &str = "game-enemy-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const ENEMY_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const ENEMY_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one enemy definition.
pub const ENEMY_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum enemy definitions in one source snapshot.
pub const ENEMY_MAX_DEFINITIONS: usize = 4_096;
/// Maximum entries returned by one bounded definition or move page.
pub const ENEMY_MAX_PAGE_ITEMS: usize = 64;
/// Maximum tags on one enemy.
pub const ENEMY_MAX_TAGS: usize = 64;
/// Maximum stats in one base or scaled profile.
pub const ENEMY_MAX_STATS: usize = 64;
/// Maximum difficulty/mode profiles on one enemy.
pub const ENEMY_MAX_STAT_PROFILES: usize = 64;
/// Maximum origin variants on one enemy.
pub const ENEMY_MAX_ORIGIN_VARIANTS: usize = 32;
/// Maximum moves on one enemy.
pub const ENEMY_MAX_MOVES: usize = 128;
/// Maximum effects in one move.
pub const ENEMY_MAX_EFFECTS: usize = 32;
/// Maximum phases on one enemy.
pub const ENEMY_MAX_PHASES: usize = 32;
/// Maximum behavior transitions on one enemy.
pub const ENEMY_MAX_TRANSITIONS: usize = 64;
/// Maximum semantic references on one enemy or move.
pub const ENEMY_MAX_REFERENCES: usize = 128;
/// Maximum parameters on one effect, condition, or move.
pub const ENEMY_MAX_PARAMETERS: usize = 32;
/// Maximum unresolved formula inputs.
pub const ENEMY_MAX_FORMULA_INPUTS: usize = 32;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized enemy value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static enemy definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyDefinitionReference {
    /// Catalog witness that owns this enemy identity.
    pub catalog: EnemyCatalogBinding,
    /// Namespaced content definition identity.
    pub enemy_id: String,
}

/// Exact static move reference.  Move IDs are scoped by their enemy definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyMoveReference {
    /// Catalog witness that owns this move identity.
    pub catalog: EnemyCatalogBinding,
    /// Owning enemy definition identity.
    pub enemy_id: String,
    /// Stable move identity.
    pub move_id: String,
}

/// Owner-defined source origin/provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyOrigin {
    /// Source-defined origin kind, such as a package or registry.
    pub kind: String,
    /// Optional active package identity.
    pub package_id: Option<String>,
    /// Optional active package version.
    pub package_version: Option<String>,
}

/// Coarse enemy role copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyKind {
    /// Ordinary encounter enemy.
    Normal,
    /// Elite encounter enemy.
    Elite,
    /// Boss or act-ending enemy.
    Boss,
    /// Temporary or spawned subordinate enemy.
    Minion,
    /// Owner-defined role.
    Custom(String),
}

/// Owner-defined enemy visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a behavior fact.  This keeps unverified rules distinct from observations.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyEvidence {
    /// The owner source exposes the rule as an authoritative definition.
    Authoritative,
    /// The rule was copied from a source-owned definition without runtime verification.
    SourceDerived,
    /// The rule was independently authored and is not a host claim.
    IndependentlyAuthored,
    /// The value was observed on a visible surface.
    Observed,
    /// Evidence is insufficient to classify the value as authoritative.
    Unverified,
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::EnemyCatalogError> {
    if value.is_empty()
        || value.len() > ENEMY_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::EnemyCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::EnemyCatalogError> {
    if value.is_empty() || value.len() > ENEMY_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(super::EnemyCatalogError::InvalidInput(field));
    }
    Ok(())
}
