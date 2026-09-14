// SPDX-License-Identifier: MIT

mod value;

pub use value::*;

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const REWARD_REFERENCE_ENTITY_KIND: &str = "reward";
/// Manifest family resolved for card item references.
pub const REWARD_REFERENCE_CARD_KIND: &str = "card";
/// Manifest family resolved for relic item references.
pub const REWARD_REFERENCE_RELIC_KIND: &str = "relic";
/// Manifest family resolved for potion item references.
pub const REWARD_REFERENCE_POTION_KIND: &str = "potion";
/// Manifest family resolved for currency-unit references.
pub const REWARD_REFERENCE_CURRENCY_KIND: &str = "currency";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const REWARD_REFERENCE_PRODUCER_VERSION: &str = "game-reward-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const REWARD_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const REWARD_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one reward definition.
pub const REWARD_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum reward definitions in one source snapshot.
pub const REWARD_MAX_DEFINITIONS: usize = 1_024;
/// Maximum entries returned by one bounded definition or item page.
pub const REWARD_MAX_PAGE_ITEMS: usize = 64;
/// Maximum offered items on one reward definition.
pub const REWARD_MAX_ITEMS: usize = 64;
/// Maximum generation rules on one reward definition.
pub const REWARD_MAX_RULES: usize = 64;
/// Maximum typed requirements on one rule or selection.
pub const REWARD_MAX_REQUIREMENTS: usize = 32;
/// Maximum modifier rules on one generation rule.
pub const REWARD_MAX_MODIFIERS: usize = 32;
/// Maximum rarity weights on one generation rule.
pub const REWARD_MAX_RARITY_WEIGHTS: usize = 32;
/// Maximum parameters on one requirement.
pub const REWARD_MAX_PARAMETERS: usize = 32;
/// Maximum unresolved formula inputs.
pub const REWARD_MAX_FORMULA_INPUTS: usize = 32;
/// Maximum semantic references on one owner record.
pub const REWARD_MAX_REFERENCES: usize = 128;
/// Maximum legal actions on one selection group.
pub const REWARD_MAX_LEGAL_ACTIONS: usize = 16;
/// Maximum declared stages on one offer-state policy.
pub const REWARD_MAX_STAGES: usize = 16;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized reward value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static reward offer definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardDefinitionReference {
    /// Catalog witness that owns this reward identity.
    pub catalog: RewardCatalogBinding,
    /// Namespaced reward offer definition identity.
    pub reward_id: String,
}

/// Exact static offered-item reference scoped by its reward definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardItemReference {
    /// Catalog witness that owns this item identity.
    pub catalog: RewardCatalogBinding,
    /// Owning reward offer definition identity.
    pub reward_id: String,
    /// Stable offered-item identity.
    pub item_id: String,
}

/// Live run/room/snapshot fence for one observed reward offer.
///
/// This source-only slice does not read live runs; the type exists so a future live reader cannot
/// confuse a run/room/snapshot identity with a static definition identity.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardSnapshotReference {
    /// Observed run identity.
    pub run_id: String,
    /// Observed live room/node identity.
    pub room_id: String,
    /// Observed coherent snapshot identity.
    pub snapshot_id: String,
}

/// Live reward offer instance identity, deliberately distinct from any static definition ID.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardOfferReference {
    /// Observed run/room/snapshot fence.
    pub snapshot: RewardSnapshotReference,
    /// Observed live reward offer instance identity.
    pub offer_id: String,
}

/// Live offered-item instance identity, distinct from a static item definition ID.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardItemInstanceReference {
    /// Live offer instance that produced the item.
    pub offer: RewardOfferReference,
    /// Observed live item instance identity.
    pub item_instance_id: String,
}

/// Transient reward-action identity, distinct from a static selection or item definition.
///
/// The host assigns action identities for one rendered selection; they are never stable definition
/// identities and are not resolved by this source-only slice.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RewardActionReference {
    /// Observed offer instance that produced the action.
    pub offer: RewardOfferReference,
    /// Observed transient action identity.
    pub action_id: String,
}

/// Owner-defined reward definition visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static reward reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a static reward fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardEvidence {
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

/// Source support state for the reward reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RewardFamilyState {
    /// Source can project all reward records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: RewardFamilyState,
    /// Number of reward definitions in the manifest.
    pub definition_count: usize,
}

/// Live reward-offer state representation.
///
/// This slice defines the state vocabulary but does not observe live state. Claimed,
/// blocked-capacity, replacement-required, optional-skip, and multi-stage outcomes stay distinct
/// rather than collapsing into a single "handled" boolean.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RewardOfferState {
    /// The offer is visible and selectable.
    Offered,
    /// The reward was already claimed.
    Claimed,
    /// The reward is blocked because the destination has no free capacity.
    BlockedCapacity,
    /// Acceptance requires replacing an existing item.
    ReplacementRequired,
    /// An optional reward was explicitly skipped.
    Skipped,
    /// The reward is one stage of a multi-stage grant.
    MultiStage {
        /// One-based current stage.
        stage: u32,
        /// Total declared stages.
        total: u32,
    },
    /// The source could not classify the offer state.
    Unknown,
}

/// Relative permissiveness of a visibility label for follow-up leak checks.
///
/// A record may not target an item that is more restricted than the record that reaches it.
pub(super) const fn visibility_rank(visibility: RewardVisibility) -> u8 {
    match visibility {
        RewardVisibility::Hidden | RewardVisibility::Unknown => 0,
        RewardVisibility::OwnerOnly => 1,
        RewardVisibility::Visible => 2,
    }
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::RewardCatalogError> {
    if value.is_empty()
        || value.len() > REWARD_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::RewardCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::RewardCatalogError> {
    if value.is_empty()
        || value.len() > REWARD_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(super::RewardCatalogError::InvalidInput(field));
    }
    Ok(())
}
