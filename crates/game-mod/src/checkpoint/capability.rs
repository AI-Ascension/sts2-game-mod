// SPDX-License-Identifier: MIT

use super::error::CheckpointCaptureRejection;

/// Every boundary that the owner must classify before capture is admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointBoundary {
    /// A settled map choice with no queued effect or transition.
    SettledMapChoice,
    /// A stable player-turn combat decision.
    StablePlayerTurnCombat,
    /// A later-turn combat decision with changed turn/RNG witnesses.
    LaterTurnCombat,
    /// A reward or card-selection offer.
    RewardCardSelection,
    /// An event choice.
    Event,
    /// A shop choice.
    Shop,
    /// A rest-site choice.
    Rest,
    /// Enemy execution is not a capture boundary.
    EnemyTurn,
    /// Animation settlement is not a capture boundary.
    Animation,
    /// Room or phase transition is not a capture boundary.
    Transition,
    /// An unrecognized host phase is not a capture boundary.
    Unknown,
}

impl CheckpointBoundary {
    /// Returns the stable boundary token used in diagnostics and fixtures.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::SettledMapChoice => "settled_map_choice",
            Self::StablePlayerTurnCombat => "stable_player_turn_combat",
            Self::LaterTurnCombat => "later_turn_combat",
            Self::RewardCardSelection => "reward_card_selection",
            Self::Event => "event",
            Self::Shop => "shop",
            Self::Rest => "rest",
            Self::EnemyTurn => "enemy_turn",
            Self::Animation => "animation",
            Self::Transition => "transition",
            Self::Unknown => "unknown",
        }
    }

    /// Returns the complete bounded matrix, including explicitly unsafe phases.
    #[must_use]
    pub const fn all() -> &'static [Self; 11] {
        &[
            Self::SettledMapChoice,
            Self::StablePlayerTurnCombat,
            Self::LaterTurnCombat,
            Self::RewardCardSelection,
            Self::Event,
            Self::Shop,
            Self::Rest,
            Self::EnemyTurn,
            Self::Animation,
            Self::Transition,
            Self::Unknown,
        ]
    }
}

/// Why a boundary is not currently advertised as capture-capable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointUnavailableReason {
    /// The exact host has not yet supplied independent capture evidence.
    ExactHostEvidenceRequired,
    /// The coverage inventory for this phase is not complete.
    CoverageInventoryPending,
    /// The host phase cannot provide a quiescent closure.
    UnsafeBoundary,
}

impl CheckpointUnavailableReason {
    /// Returns the stable machine-readable rejection reason.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExactHostEvidenceRequired => "exact_host_evidence_required",
            Self::CoverageInventoryPending => "coverage_inventory_pending",
            Self::UnsafeBoundary => "unsafe_boundary",
        }
    }
}

/// Capability descriptor for one owner boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointCapability {
    /// Capture remains unavailable until the stated gate is met.
    Unavailable {
        /// The gate that prevents capture.
        reason: CheckpointUnavailableReason,
    },
}

impl CheckpointCapability {
    /// Returns whether a native capture implementation is advertised.
    #[must_use]
    pub const fn is_available(self) -> bool {
        false
    }

    /// Returns the explicit gate for this capability.
    #[must_use]
    pub const fn reason(self) -> CheckpointUnavailableReason {
        match self {
            Self::Unavailable { reason } => reason,
        }
    }

    /// Maps this fail-closed capability to the typed rejection for a boundary.
    #[must_use]
    pub const fn rejection(self, boundary: CheckpointBoundary) -> CheckpointCaptureRejection {
        match self {
            Self::Unavailable { reason } => match reason {
                CheckpointUnavailableReason::UnsafeBoundary => {
                    CheckpointCaptureRejection::UnsafeBoundary
                }
                _ => CheckpointCaptureRejection::UnsupportedBoundary { boundary, reason },
            },
        }
    }
}

/// Bounded capability descriptor owned by the game-mod capture port.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CheckpointCapabilities;

impl CheckpointCapabilities {
    /// Returns the current fail-closed disposition for a boundary.
    #[must_use]
    pub const fn for_boundary(self, boundary: CheckpointBoundary) -> CheckpointCapability {
        let reason = match boundary {
            CheckpointBoundary::SettledMapChoice
            | CheckpointBoundary::StablePlayerTurnCombat
            | CheckpointBoundary::LaterTurnCombat => {
                CheckpointUnavailableReason::ExactHostEvidenceRequired
            }
            CheckpointBoundary::RewardCardSelection
            | CheckpointBoundary::Event
            | CheckpointBoundary::Shop
            | CheckpointBoundary::Rest => CheckpointUnavailableReason::CoverageInventoryPending,
            CheckpointBoundary::EnemyTurn
            | CheckpointBoundary::Animation
            | CheckpointBoundary::Transition
            | CheckpointBoundary::Unknown => CheckpointUnavailableReason::UnsafeBoundary,
        };
        CheckpointCapability::Unavailable { reason }
    }
}
