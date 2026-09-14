// SPDX-License-Identifier: MIT

mod value;

pub use value::*;

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const EVENT_REFERENCE_ENTITY_KIND: &str = "event";
/// Manifest family resolved for encounter references.
pub const EVENT_REFERENCE_ENCOUNTER_KIND: &str = "encounter";
/// Manifest family resolved for enemy references.
pub const EVENT_REFERENCE_ENEMY_KIND: &str = "enemy";
/// Manifest family resolved for relic references.
pub const EVENT_REFERENCE_RELIC_KIND: &str = "relic";
/// Manifest family resolved for card references.
pub const EVENT_REFERENCE_CARD_KIND: &str = "card";
/// Manifest family resolved for potion references.
pub const EVENT_REFERENCE_POTION_KIND: &str = "potion";
/// Manifest family resolved for act references.
pub const EVENT_REFERENCE_ACT_KIND: &str = "act";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const EVENT_REFERENCE_PRODUCER_VERSION: &str = "game-event-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const EVENT_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const EVENT_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one event definition.
pub const EVENT_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum event definitions in one source snapshot.
pub const EVENT_MAX_DEFINITIONS: usize = 1_024;
/// Maximum entries returned by one bounded definition page.
pub const EVENT_MAX_PAGE_ITEMS: usize = 64;
/// Maximum narrative pages on one event.
pub const EVENT_MAX_PAGES: usize = 16;
/// Maximum choices on one event.
pub const EVENT_MAX_OPTIONS: usize = 64;
/// Maximum typed requirements on one event or option.
pub const EVENT_MAX_REQUIREMENTS: usize = 32;
/// Maximum typed costs on one option.
pub const EVENT_MAX_COSTS: usize = 32;
/// Maximum possible outcomes on one option.
pub const EVENT_MAX_OUTCOMES: usize = 32;
/// Maximum effects on one outcome.
pub const EVENT_MAX_EFFECTS: usize = 32;
/// Maximum parameters on one requirement.
pub const EVENT_MAX_PARAMETERS: usize = 32;
/// Maximum unresolved formula inputs.
pub const EVENT_MAX_FORMULA_INPUTS: usize = 32;
/// Maximum semantic references on one owner record.
pub const EVENT_MAX_REFERENCES: usize = 128;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized event value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static event definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventDefinitionReference {
    /// Catalog witness that owns this event identity.
    pub catalog: EventCatalogBinding,
    /// Namespaced event definition identity.
    pub event_id: String,
}

/// Exact static narrative-page reference scoped by its event definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventPageReference {
    /// Catalog witness that owns this page identity.
    pub catalog: EventCatalogBinding,
    /// Owning event definition identity.
    pub event_id: String,
    /// Stable narrative-page identity.
    pub page_id: String,
}

/// Exact static option reference scoped by its event definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventOptionReference {
    /// Catalog witness that owns this option identity.
    pub catalog: EventCatalogBinding,
    /// Owning event definition identity.
    pub event_id: String,
    /// Stable option identity.
    pub option_id: String,
}

/// Exact static outcome reference scoped by its option and event definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventOutcomeReference {
    /// Catalog witness that owns this outcome identity.
    pub catalog: EventCatalogBinding,
    /// Owning event definition identity.
    pub event_id: String,
    /// Owning option identity.
    pub option_id: String,
    /// Stable outcome identity.
    pub outcome_id: String,
}

/// Live event/run instance identity, deliberately distinct from any static definition ID.
///
/// An event instance belongs to one run observation; it is never an event, page, option, or
/// outcome definition ID. This source-only slice does not read live runs.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventInstanceReference {
    /// Observed run identity.
    pub run_id: String,
    /// Observed live event instance identity.
    pub event_instance_id: String,
}

/// Transient button/action identity, distinct from a static option definition.
///
/// The host assigns action identities for one rendered selection; they are never stable option
/// identities and are not resolved by this source-only slice.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventActionReference {
    /// Observed event instance that produced the action.
    pub instance: EventInstanceReference,
    /// Observed transient action identity.
    pub action_id: String,
}

/// Owner-defined event visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static event reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a static event fact.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventEvidence {
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

/// Source support state for the event reference family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EventFamilyState {
    /// Source can project all event records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: EventFamilyState,
    /// Number of event definitions in the manifest.
    pub definition_count: usize,
}

/// Relative permissiveness of a visibility label for follow-up leak checks.
///
/// A branch may not target a page that is more restricted than the outcome that reaches it.
pub(super) const fn visibility_rank(visibility: EventVisibility) -> u8 {
    match visibility {
        EventVisibility::Hidden | EventVisibility::Unknown => 0,
        EventVisibility::OwnerOnly => 1,
        EventVisibility::Visible => 2,
    }
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::EventCatalogError> {
    if value.is_empty()
        || value.len() > EVENT_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::EventCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::EventCatalogError> {
    if value.is_empty() || value.len() > EVENT_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(super::EventCatalogError::InvalidInput(field));
    }
    Ok(())
}
