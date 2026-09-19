// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

use super::RestText;

/// Manifest family handled by this owner-local producer.
pub const REST_REFERENCE_ENTITY_KIND: &str = "rest-site";
/// Manifest family resolved for rest-option effect and rule definitions.
pub const REST_REFERENCE_EFFECT_KIND: &str = "rest-effect";
/// Manifest family resolved for card targets, upgrade candidates, and card cost targets.
pub const REST_REFERENCE_CARD_KIND: &str = "card";
/// Manifest family resolved for relic targets and relic limits.
pub const REST_REFERENCE_RELIC_KIND: &str = "relic";
/// Manifest family resolved for potion targets.
pub const REST_REFERENCE_POTION_KIND: &str = "potion";
/// Manifest family resolved for co-op player targets in a shared-rest selection.
pub const REST_REFERENCE_PLAYER_KIND: &str = "player";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const REST_REFERENCE_PRODUCER_VERSION: &str = "game-rest-site-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const REST_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const REST_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one rest-site definition.
pub const REST_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum rest-site definitions in one source snapshot.
pub const REST_MAX_DEFINITIONS: usize = 256;
/// Maximum rest options on one rest-site definition.
pub const REST_MAX_OPTIONS: usize = 64;
/// Maximum effects on one rest option.
pub const REST_MAX_EFFECTS: usize = 32;
/// Maximum healing modifiers on one healing effect.
pub const REST_MAX_HEAL_MODIFIERS: usize = 32;
/// Maximum requirements on one option or selection requirement.
pub const REST_MAX_REQUIREMENTS: usize = 32;
/// Maximum costs on one rest option.
pub const REST_MAX_COSTS: usize = 16;
/// Maximum limits on one rest option.
pub const REST_MAX_LIMITS: usize = 16;
/// Maximum selection candidates retained for one selection requirement.
pub const REST_MAX_CANDIDATES: usize = 64;
/// Maximum prospective changes on one comparison.
pub const REST_MAX_PROSPECTIVE_CHANGES: usize = 32;
/// Maximum named coverage records on one rest-site definition.
pub const REST_MAX_COVERAGE_RECORDS: usize = 64;
/// Maximum semantic references on one owner record.
pub const REST_MAX_REFERENCES: usize = 128;
/// Maximum entries returned by one bounded rest-site or option page.
pub const REST_MAX_PAGE_ITEMS: usize = 64;
/// Maximum outstanding continuations a reader retains per list.
pub const REST_MAX_CONTINUATIONS: usize = 64;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized rest-site value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static rest-site definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestSiteDefinitionReference {
    /// Catalog witness that owns this rest-site identity.
    pub catalog: RestCatalogBinding,
    /// Namespaced rest-site definition identity.
    pub site_id: String,
}

/// Exact static rest-option reference scoped by its rest-site definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestOptionReference {
    /// Catalog witness that owns this option identity.
    pub catalog: RestCatalogBinding,
    /// Owning rest-site definition identity.
    pub site_id: String,
    /// Stable rest-option identity.
    pub option_id: String,
}

/// Availability reference fenced by the option-set generation that produced it.
///
/// The host renders a rest menu for one option set; when the host re-renders that menu the earlier
/// availability no longer describes the current site. A reference bound to the earlier generation
/// is refused rather than answered with the newer options.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestOptionSetReference {
    /// Exact static option reference.
    pub option: RestOptionReference,
    /// Option-set generation the caller observed.
    pub option_set_generation: u64,
}

/// Live run/instance/epoch fence for one observed rest site.
///
/// This source-only slice does not read a live rest site; the type exists so a future live reader
/// cannot confuse a run/instance identity with a static definition identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestSnapshotReference {
    /// Observed run identity.
    pub run_id: String,
    /// Observed game instance identity.
    pub instance_id: String,
    /// Observed run epoch.
    pub epoch: u64,
    /// Observed coherent snapshot identity.
    pub snapshot_id: String,
}

/// Transient rest-action identity, distinct from a static option or effect identity.
///
/// The host assigns an action identity for one rendered rest menu. It is never a stable definition
/// identity, and this slice neither resolves nor performs it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestActionReference {
    /// Observed run/instance/epoch/snapshot fence.
    pub snapshot: RestSnapshotReference,
    /// Observed transient action identity.
    pub action_id: String,
}

/// Typed semantic reference with an explicit family.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RestSemanticReference {
    /// Referenced family.
    pub kind: RestReferenceKind,
    /// Referenced identity.
    pub id: String,
    /// Localized label for the referenced identity.
    pub label: RestText,
}

/// Family of a typed rest-site reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestReferenceKind {
    /// A rest-site definition.
    Site,
    /// A rest-option or effect/rule definition.
    Effect,
    /// A card definition.
    Card,
    /// A relic definition.
    Relic,
    /// A potion definition.
    Potion,
    /// A co-op player identity.
    Player,
    /// A rest option inside a rest-site definition.
    Option,
    /// Any other manifest family named by the source.
    Content {
        /// Manifest entity family.
        entity_kind: String,
    },
    /// Source could not classify the reference.
    Unknown,
}

/// Owner-defined rest-site visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static rest-site reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a static rest-site fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestEvidence {
    /// The owner source exposes the value as an authoritative definition.
    Authoritative,
    /// The value was copied from a source-owned definition without runtime verification.
    SourceDerived,
    /// The value was independently authored and is not a host claim.
    IndependentlyAuthored,
    /// The value was observed on a visible surface.
    Observed,
    /// Evidence is insufficient to classify the value as authoritative.
    Unverified,
}

/// Kinds of rest option a rest site can offer.
///
/// The named variants are the identifiers the audited source exposes; anything else is carried
/// explicitly as a custom, unsupported, or unknown kind rather than folded into one of them.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestOptionKind {
    /// Rest to restore a fraction of maximum health.
    Heal,
    /// Remove a card from the deck.
    Smith,
    /// Mend a card that a prior smith damaged.
    Mend,
    /// Add a card from a produced set.
    Cook,
    /// Add or transform a card from a produced set.
    Clone,
    /// Dig for a relic.
    Dig,
    /// Hatch a held egg-like relic.
    Hatch,
    /// Kindle a held light-like relic.
    Kindle,
    /// Lift a held training-like relic.
    Lift,
    /// Owner-defined option kind.
    Custom(String),
    /// A kind is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the option.
    Unknown,
}

/// Source support state for the rest-site reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RestFamilyState {
    /// Source can project all rest-site records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: RestFamilyState,
    /// Number of rest-site definitions in the manifest.
    pub definition_count: usize,
}

/// Relative permissiveness of a visibility label for follow-up leak checks.
///
/// A record may not target a definition that is more restricted than the record reaching it.
pub(super) const fn visibility_rank(visibility: RestVisibility) -> u8 {
    match visibility {
        RestVisibility::Hidden | RestVisibility::Unknown => 0,
        RestVisibility::OwnerOnly => 1,
        RestVisibility::Visible => 2,
    }
}
