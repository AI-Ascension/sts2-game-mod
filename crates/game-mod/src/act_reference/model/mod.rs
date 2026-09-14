// SPDX-License-Identifier: MIT

mod value;

pub use value::*;

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const ACT_REFERENCE_ENTITY_KIND: &str = "act";
/// Manifest family resolved for encounter enemy references.
pub const ACT_REFERENCE_ENEMY_KIND: &str = "enemy";
/// Manifest family resolved for encounter references.
pub const ACT_REFERENCE_ENCOUNTER_KIND: &str = "encounter";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const ACT_REFERENCE_PRODUCER_VERSION: &str = "game-act-encounter-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const ACT_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const ACT_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one act definition.
pub const ACT_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum act definitions in one source snapshot.
pub const ACT_MAX_DEFINITIONS: usize = 1_024;
/// Maximum entries returned by one bounded definition page.
pub const ACT_MAX_PAGE_ITEMS: usize = 64;
/// Maximum room/node categories on one act.
pub const ACT_MAX_ROOM_CATEGORIES: usize = 64;
/// Maximum encounter definitions on one act.
pub const ACT_MAX_ENCOUNTERS: usize = 256;
/// Maximum encounter pools on one act.
pub const ACT_MAX_POOLS: usize = 32;
/// Maximum entries in one encounter pool.
pub const ACT_MAX_POOL_ENTRIES: usize = 128;
/// Maximum enemy groups in one encounter.
pub const ACT_MAX_ENEMY_GROUPS: usize = 32;
/// Maximum enemies in one encounter group.
pub const ACT_MAX_GROUP_ENEMIES: usize = 32;
/// Maximum variant identities on one encounter enemy.
pub const ACT_MAX_VARIANTS: usize = 32;
/// Maximum eligibility conditions on one encounter.
pub const ACT_MAX_ELIGIBILITY: usize = 32;
/// Maximum map-generation constraints on one act.
pub const ACT_MAX_CONSTRAINTS: usize = 64;
/// Maximum parameters on one condition or constraint.
pub const ACT_MAX_PARAMETERS: usize = 32;
/// Maximum unresolved formula inputs.
pub const ACT_MAX_FORMULA_INPUTS: usize = 32;
/// Maximum semantic references on one owner record.
pub const ACT_MAX_REFERENCES: usize = 128;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized act value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static act definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActDefinitionReference {
    /// Catalog witness that owns this act identity.
    pub catalog: ActCatalogBinding,
    /// Namespaced act definition identity.
    pub act_id: String,
}

/// Exact static encounter reference.  Encounter IDs are scoped by their act definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActEncounterReference {
    /// Catalog witness that owns this encounter identity.
    pub catalog: ActCatalogBinding,
    /// Owning act definition identity.
    pub act_id: String,
    /// Stable encounter identity.
    pub encounter_id: String,
}

/// Exact static room/node category reference scoped by its act definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActRoomCategoryReference {
    /// Catalog witness that owns this category identity.
    pub catalog: ActCatalogBinding,
    /// Owning act definition identity.
    pub act_id: String,
    /// Stable room/node category identity.
    pub category_id: String,
}

/// Exact static encounter pool reference scoped by its act definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActPoolReference {
    /// Catalog witness that owns this pool identity.
    pub catalog: ActCatalogBinding,
    /// Owning act definition identity.
    pub act_id: String,
    /// Stable encounter pool identity.
    pub pool_id: String,
}

/// Live map topology identity, deliberately distinct from any static definition identity.
///
/// A map node belongs to one observed map instance; it is never an act, encounter, pool, or
/// category definition ID.  This source-only slice does not read live maps.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActMapNodeReference {
    /// Observed map snapshot identity.
    pub map_id: String,
    /// Observed live node instance identity.
    pub node_id: String,
}

/// Source-only encounter possibility for a room/node category.
///
/// This states which reference definitions *could* be assigned.  It is never a seed-specific
/// assignment and never reveals a hidden future room.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EncounterPossibility {
    /// The category maps to an enumerable encounter pool.
    Pool(ActPoolReference),
    /// A seed/run-specific assignment exists but is withheld from this source-only slice.
    Withheld(ActUnavailableReason),
    /// No pool is known for the requested category.
    Unavailable(ActUnavailableReason),
}

/// Owner-defined act visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActVisibility {
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
pub enum ActVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a generation or eligibility fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActEvidence {
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

/// Source support state for the act reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActFamilyState {
    /// Source can project all act records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: ActFamilyState,
    /// Number of act definitions in the manifest.
    pub definition_count: usize,
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::ActReferenceError> {
    if value.is_empty()
        || value.len() > ACT_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::ActReferenceError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::ActReferenceError> {
    if value.is_empty() || value.len() > ACT_MAX_TEXT_BYTES || value.chars().any(char::is_control) {
        return Err(super::ActReferenceError::InvalidInput(field));
    }
    Ok(())
}
