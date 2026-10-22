// SPDX-License-Identifier: MIT

//! Owned host copies used by the live producer. These are internal source data, not a wire API.

use serde::{Deserialize, Serialize};
use sts2_protocol::game_facts_reference_v1::{AuthenticatedScope, ErrorCode};

/// Explicit availability of a raw owner field; unavailable never means zero or false.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveFactsFieldStatus {
    /// The owner copied the value.
    Available,
    /// The owner did not observe this field.
    NotObserved,
    /// This source cannot represent the field.
    Unsupported,
    /// The owner could not establish the field's meaning.
    Unknown,
    /// A source accessor failed.
    Failed,
    /// The retained value has expired.
    Stale,
}

/// Raw field copied without a narrowing integer conversion or a default value.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LiveFactsField<T> {
    /// Availability from the source owner.
    pub status: LiveFactsFieldStatus,
    /// Present only when available.
    pub value: Option<T>,
}

/// A card occurrence copied on the host thread, with no host reference or pointer.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedLiveFactsCard {
    /// Source-owned occurrence identity from the retained bootstrap.
    pub instance_id: String,
    /// Definition copied from the host card.
    pub definition_id: String,
    /// Content inventory associated with that definition.
    pub definition_manifest: String,
    /// Current raw resolved energy cost, without a byte clamp.
    pub resolved_cost: LiveFactsField<i32>,
    /// True only when the owner observed that the card does not have an X energy cost.
    pub fixed_energy_cost: LiveFactsField<bool>,
}

/// One owner-retained capture, revalidated on the host thread before and after catalog copying.
///
/// Public construction is useful for deterministic tests, not proof of authenticity. Production
/// constructs this only after the admitted lease and native-run association have been checked.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedLiveFactsSnapshot {
    /// Scope from the authenticated lookup-binding association, not the request's claims.
    pub scope: AuthenticatedScope,
    /// Exact native mode copied with the values.
    pub mode_id: String,
    /// Current catalog generation retained by the authenticated lookup binding.
    pub catalog_generation: u64,
    /// Source-owned retained snapshot identity.
    pub snapshot_id: String,
    /// Source-owned retained capture epoch.
    pub epoch: u64,
    /// Gameplay generation copied before and after extraction.
    pub state_generation: u64,
    /// Raw current player energy, without a byte clamp.
    pub player_energy: LiveFactsField<i32>,
    /// At most 512 owned card occurrences from the same read.
    pub cards: Vec<OwnedLiveFactsCard>,
}

/// Sanitized source-side refusal; host messages and exceptions never enter this vocabulary.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiveFactsRefusal {
    /// No usable owner source is installed or available.
    MissingCapability,
    /// The request is outside the admitted lease/run/visibility scope.
    DeniedScope,
    /// The retained capture no longer matches the live owner.
    StaleSnapshot,
    /// Content or locale no longer matches the admitted capture.
    InvalidBinding,
}

impl LiveFactsRefusal {
    pub(super) const fn code(self) -> ErrorCode {
        match self {
            Self::MissingCapability => ErrorCode::MissingCapability,
            Self::DeniedScope => ErrorCode::DeniedScope,
            Self::StaleSnapshot => ErrorCode::StaleSnapshot,
            Self::InvalidBinding => ErrorCode::InvalidBinding,
        }
    }
}
