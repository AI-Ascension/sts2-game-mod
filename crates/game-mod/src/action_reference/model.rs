// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

use super::ActionText;

pub use super::kind::*;

/// Manifest family handled by this owner-local producer.
pub const ACTION_REFERENCE_ENTITY_KIND: &str = "action";
/// Manifest family resolved for preview effect and rule definitions.
pub const ACTION_REFERENCE_EFFECT_KIND: &str = "action-effect";
/// Manifest family resolved for card targets and card movements.
pub const ACTION_REFERENCE_CARD_KIND: &str = "card";
/// Manifest family resolved for enemy targets.
pub const ACTION_REFERENCE_ENEMY_KIND: &str = "enemy";
/// Manifest family resolved for co-op player targets.
pub const ACTION_REFERENCE_PLAYER_KIND: &str = "player";
/// Manifest family resolved for status and power transitions.
pub const ACTION_REFERENCE_STATUS_KIND: &str = "power_status";
/// Manifest family resolved for relic targets.
pub const ACTION_REFERENCE_RELIC_KIND: &str = "relic";
/// Manifest family resolved for potion targets.
pub const ACTION_REFERENCE_POTION_KIND: &str = "potion";
/// Manifest family resolved for bounded resources and statuses.
pub const ACTION_REFERENCE_RESOURCE_KIND: &str = "resource";
/// Manifest family resolved for a selection one action still requires.
pub const ACTION_REFERENCE_SELECTION_KIND: &str = "selection";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const ACTION_REFERENCE_PRODUCER_VERSION: &str = "game-action-preview-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const ACTION_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const ACTION_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one action definition.
pub const ACTION_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum action definitions in one source snapshot.
pub const ACTION_MAX_DEFINITIONS: usize = 256;
/// Maximum observed targets on one action definition.
pub const ACTION_MAX_TARGETS: usize = 64;
/// Maximum cost contributors on one action definition.
pub const ACTION_MAX_COST_CONTRIBUTORS: usize = 32;
/// Maximum target restrictions on one action definition.
pub const ACTION_MAX_RESTRICTIONS: usize = 32;
/// Maximum consequence records on one preview.
pub const ACTION_MAX_CHANGES: usize = 32;
/// Maximum previews declared on one action definition.
pub const ACTION_MAX_PREVIEWS: usize = 32;
/// Maximum affected target identities named by one preview.
pub const ACTION_MAX_AFFECTED: usize = 32;
/// Maximum status transitions on one preview.
pub const ACTION_MAX_STATUSES: usize = 32;
/// Maximum card movements on one preview.
pub const ACTION_MAX_MOVEMENTS: usize = 32;
/// Maximum selection requirements on one preview.
pub const ACTION_MAX_SELECTION_REQUIREMENTS: usize = 16;
/// Maximum picks one previewed selection requirement may demand.
pub const ACTION_MAX_REQUIRED_PICKS: u32 = 32;
/// Maximum stated assumptions on one preview.
pub const ACTION_MAX_ASSUMPTIONS: usize = 16;
/// Maximum named omissions on one preview.
pub const ACTION_MAX_OMISSIONS: usize = 32;
/// Maximum declared supported preview classes on one action definition.
pub const ACTION_MAX_PREVIEW_CLASSES: usize = 5;
/// Maximum named coverage records on one action definition.
pub const ACTION_MAX_COVERAGE_RECORDS: usize = 64;
/// Maximum semantic references on one owner record.
pub const ACTION_MAX_REFERENCES: usize = 128;
/// Maximum entries returned by one bounded page.
pub const ACTION_MAX_PAGE_ITEMS: usize = 64;
/// Maximum outstanding continuations a reader retains per list.
pub const ACTION_MAX_CONTINUATIONS: usize = 64;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized action value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static legal-action definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionReference {
    /// Catalog witness that owns this action identity.
    pub catalog: ActionCatalogBinding,
    /// Namespaced legal-action definition identity.
    pub action_id: String,
}

/// Live run/instance/epoch fence for one observed legal-action frame.
///
/// This source-only slice never reads a live frame; the type exists so a caller cannot confuse a
/// run or instance identity with a static definition identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionSnapshotReference {
    /// Observed run identity.
    pub run_id: String,
    /// Observed game instance identity.
    pub instance_id: String,
    /// Observed run epoch.
    pub epoch: u64,
    /// Observed coherent snapshot identity.
    pub snapshot_id: String,
}

/// Transient action-instance identity fenced by the legal-action generation that produced it.
///
/// The host presents one generation for one rendered legal-action frame; when any relevant
/// transition occurs the host moves to the next generation. A reference bound to an earlier or
/// later generation is refused rather than answered with the current frame's consequences.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionInstanceReference {
    /// Observed run/instance/epoch/snapshot fence.
    pub snapshot: ActionSnapshotReference,
    /// Observed transient action-instance identity.
    pub action_instance_id: String,
    /// Legal-action generation the caller observed.
    pub generation: u64,
}

/// Static legal-action frame reference fenced by the legal-action generation that produced it.
///
/// This is the "current action reference" a caller explains or previews: a stable definition
/// identity plus the generation the host presented. It carries no run, instance, or snapshot
/// identity, so a static reference can never be mistaken for a live one.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionFrameReference {
    /// Exact static legal-action definition reference.
    pub action: ActionReference,
    /// Legal-action generation the caller observed.
    pub generation: u64,
}

/// Exact target reference scoped by its legal-action definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionTargetReference {
    /// Catalog witness that owns this target identity.
    pub catalog: ActionCatalogBinding,
    /// Owning legal-action definition identity.
    pub action_id: String,
    /// Stable observed target identity within its action.
    pub target_id: String,
}

/// Exact static preview reference scoped by its legal-action definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionPreviewReference {
    /// Catalog witness that owns this preview identity.
    pub catalog: ActionCatalogBinding,
    /// Owning legal-action definition identity.
    pub action_id: String,
    /// Stable preview identity.
    pub preview_id: String,
}

/// Typed semantic reference with an explicit family.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionSemanticReference {
    /// Referenced family.
    pub kind: ActionReferenceKind,
    /// Referenced identity.
    pub id: String,
    /// Localized label for the referenced identity.
    pub label: ActionText,
}

/// Owner-defined action visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static action query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionVisibilityScope {
    /// Publicly visible actions.
    Public,
    /// Public actions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a static action fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionEvidence {
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

/// Source support state for the action reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionFamilyState {
    /// Source can project all action records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: ActionFamilyState,
    /// Number of action definitions in the manifest.
    pub definition_count: usize,
}

/// Relative permissiveness of a visibility label for follow-up leak checks.
///
/// A record may not target a definition that is more restricted than the record reaching it.
pub(super) const fn action_visibility_rank(visibility: ActionVisibility) -> u8 {
    match visibility {
        ActionVisibility::Hidden | ActionVisibility::Unknown => 0,
        ActionVisibility::OwnerOnly => 1,
        ActionVisibility::Visible => 2,
    }
}
