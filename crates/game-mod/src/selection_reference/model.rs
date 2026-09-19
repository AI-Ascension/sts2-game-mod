// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

use super::SelectionText;

pub use super::kind::*;

/// Manifest family handled by this owner-local producer.
pub const SELECTION_REFERENCE_ENTITY_KIND: &str = "selection";
/// Manifest family resolved for prospective effect and rule definitions.
pub const SELECTION_REFERENCE_EFFECT_KIND: &str = "selection-effect";
/// Manifest family resolved for card candidates and card targets.
pub const SELECTION_REFERENCE_CARD_KIND: &str = "card";
/// Manifest family resolved for relic candidates and relic limits.
pub const SELECTION_REFERENCE_RELIC_KIND: &str = "relic";
/// Manifest family resolved for potion candidates.
pub const SELECTION_REFERENCE_POTION_KIND: &str = "potion";
/// Manifest family resolved for player candidates in a co-op selector.
pub const SELECTION_REFERENCE_PLAYER_KIND: &str = "player";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const SELECTION_REFERENCE_PRODUCER_VERSION: &str = "game-selection-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const SEL_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const SEL_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one selection definition.
pub const SEL_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum selection definitions in one source snapshot.
pub const SEL_MAX_DEFINITIONS: usize = 256;
/// Maximum candidates on one selection definition.
pub const SEL_MAX_CANDIDATES: usize = 64;
/// Maximum picks one selection may require.
pub const SEL_MAX_PICKS: u32 = 32;
/// Maximum prospective effects on one candidate.
pub const SEL_MAX_EFFECTS: usize = 32;
/// Maximum named coverage records on one selection definition.
pub const SEL_MAX_COVERAGE_RECORDS: usize = 64;
/// Maximum semantic references on one owner record.
pub const SEL_MAX_REFERENCES: usize = 128;
/// Maximum declared steps in one multi-step selection sequence.
pub const SEL_MAX_STEPS: u32 = 32;
/// Maximum entries returned by one bounded page.
pub const SEL_MAX_PAGE_ITEMS: usize = 64;
/// Maximum outstanding continuations a reader retains per list.
pub const SEL_MAX_CONTINUATIONS: usize = 64;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized selection value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static selection definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionReference {
    /// Catalog witness that owns this selection identity.
    pub catalog: SelectionCatalogBinding,
    /// Namespaced selection definition identity.
    pub selection_id: String,
}

/// Exact static candidate reference scoped by its selection definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionCandidateReference {
    /// Catalog witness that owns this candidate identity.
    pub catalog: SelectionCatalogBinding,
    /// Owning selection definition identity.
    pub selection_id: String,
    /// Stable candidate identity.
    pub candidate_id: String,
}

/// Selector identity fenced by the selector generation that produced it.
///
/// The host presents one selector generation for one prompt; when a pick changes the candidate
/// domain the host moves to the next generation. A reference bound to an earlier or later
/// generation is refused rather than answered with the newer domain.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectorGenerationReference {
    /// Exact static selection reference.
    pub selection: SelectionReference,
    /// Selector generation the caller observed.
    pub selector_generation: u64,
}

/// Live run/instance/epoch fence for one observed selector.
///
/// This source-only slice never reads a live selector; the type exists so a future live reader
/// cannot confuse a run/instance identity with a static definition identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionSnapshotReference {
    /// Observed run identity.
    pub run_id: String,
    /// Observed game instance identity.
    pub instance_id: String,
    /// Observed run epoch.
    pub epoch: u64,
    /// Observed coherent snapshot identity.
    pub snapshot_id: String,
}

/// Transient selector-instance identity, distinct from a static selection identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectorInstanceReference {
    /// Observed run/instance/epoch/snapshot fence.
    pub snapshot: SelectionSnapshotReference,
    /// Observed transient selector-instance identity.
    pub selector_instance_id: String,
}

/// Transient legal action identity for one selector instance.
///
/// The host assigns an action identity for one rendered prompt. It is never a stable definition
/// identity, and this slice neither resolves nor performs it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionActionReference {
    /// Observed transient selector instance.
    pub selector: SelectorInstanceReference,
    /// Stable host action vocabulary token.
    pub action_kind: String,
    /// Observed transient action identity.
    pub action_id: String,
}

/// Typed semantic reference with an explicit family.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionSemanticReference {
    /// Referenced family.
    pub kind: SelectionReferenceKind,
    /// Referenced identity.
    pub id: String,
    /// Localized label for the referenced identity.
    pub label: SelectionText,
}

/// Family of a typed selection reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionReferenceKind {
    /// A selection definition.
    Selection,
    /// A prospective effect or rule definition.
    Effect,
    /// A card definition.
    Card,
    /// A relic definition.
    Relic,
    /// A potion definition.
    Potion,
    /// A co-op player identity.
    Player,
    /// A candidate inside a selection definition.
    Candidate,
    /// Any other manifest family named by the source.
    Content {
        /// Manifest entity family.
        entity_kind: String,
    },
    /// Source could not classify the reference.
    Unknown,
}

/// Owner-defined selection visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static selection reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionVisibilityScope {
    /// Publicly visible definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a static selection fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionEvidence {
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

/// Ordering rule the presented candidate list follows.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionOrderingRule {
    /// Candidates keep their definition order.
    Stable,
    /// Candidates are ordered by a named key.
    Ordered(String),
    /// The host reorders candidates per presentation.
    PresentationOrder,
    /// Source could not classify the ordering rule.
    Unknown,
}

/// Whether one candidate identity may be picked more than once.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionDuplicateRule {
    /// Each candidate identity may be picked at most once.
    Distinct,
    /// The same candidate identity may be picked more than once.
    RepeatsAllowed,
    /// Source could not classify the duplicate rule.
    Unknown,
}

/// How the prompt completes once its picks are satisfied.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionConfirmation {
    /// The host closes the prompt as soon as the picks are complete.
    AutomaticClose,
    /// The caller confirms explicitly after the picks are complete.
    ExplicitConfirm,
    /// Source could not classify the confirmation semantics.
    Unknown,
}

/// How a prompt may be abandoned.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionCancellation {
    /// The prompt offers an explicit cancel.
    Cancel,
    /// The prompt offers a back step.
    Back,
    /// Owner-defined cancellation semantics.
    Custom(String),
    /// Source could not classify the cancellation semantics.
    Unknown,
}

/// Source support state for the selection reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionFamilyState {
    /// Source can project all selection records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: SelectionFamilyState,
    /// Number of selection definitions in the manifest.
    pub definition_count: usize,
}

/// Domain the selector presents once the current picks are complete.
///
/// A choice can change the next candidate domain, so a multi-step selection names the next
/// selector by identity and generation instead of leaving the caller to guess it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SelectionNextDomain {
    /// Selection definition presented next.
    pub selection_id: String,
    /// Candidate family the next selector presents.
    pub candidate_kind: SelectionCandidateKind,
    /// Selector generation presented next.
    pub selector_generation: u64,
}

/// Relative permissiveness of a visibility label for follow-up leak checks.
///
/// A record may not target a definition that is more restricted than the record reaching it.
pub(super) const fn selection_visibility_rank(visibility: SelectionVisibility) -> u8 {
    match visibility {
        SelectionVisibility::Hidden | SelectionVisibility::Unknown => 0,
        SelectionVisibility::OwnerOnly => 1,
        SelectionVisibility::Visible => 2,
    }
}
