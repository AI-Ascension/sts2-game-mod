// SPDX-License-Identifier: MIT

//! Public party context: the phase the party is in, the votes it is taking, and the relationships
//! between its members.
//!
//! This is the context a caller needs to interpret an action, and it is deliberately separate from
//! authority: legal action authority, synchronization receipts, host authority and the epoch stay
//! with the modules that already own them and are not republished here. A vote a party is taking is
//! public by construction, so it carries no member's private choice.

use super::CoopFieldValue;

/// Phase of the public turn a party is in.
///
/// The phase is a property of the party rather than of one member, because two members of one party
/// cannot be in two phases at once and a per-member phase would let a caller read a vote against
/// the wrong step.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopTurnPhase {
    /// The party is choosing between offered options.
    Selection,
    /// The party is resolving a combat turn.
    Combat,
    /// The party is between rooms.
    Interlude,
    /// The run has reached a terminal state.
    Terminal,
}

impl CoopTurnPhase {
    /// Returns whether a vote may be taken in this phase.
    ///
    /// A terminal party is not choosing anything, so a vote published against it is refused rather
    /// than shown as a step that can never resolve.
    #[must_use]
    pub const fn admits_vote(self) -> bool {
        matches!(self, Self::Selection | Self::Combat)
    }

    /// Returns the stable name used in failures.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Selection => "selection",
            Self::Combat => "combat",
            Self::Interlude => "interlude",
            Self::Terminal => "terminal",
        }
    }
}

/// One public option a party vote offers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopVoteChoice {
    /// Opaque option identity.
    pub choice_id: String,
    /// Localized label, or the reason no label is observed.
    pub label: CoopFieldValue<String>,
}

/// One public vote the party is taking.
///
/// The vote states what is being decided and which members have signalled a choice, but not which
/// option any member picked: a member's own selection is its private choice, so only the fact that
/// the vote was decided is published.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopVote {
    /// Opaque vote identity.
    pub vote_id: String,
    /// Phase the vote is being taken in.
    pub phase: CoopTurnPhase,
    /// The options offered, or the reason none is observed.
    pub choices: CoopFieldValue<Vec<CoopVoteChoice>>,
    /// Members that have signalled a choice, in party order.
    pub decided_peer_ids: Vec<String>,
    /// Whether the vote has resolved.
    pub resolved: bool,
}

/// One public targeting relationship between two members.
///
/// A target is published as a relationship rather than as a value on either member, so a caller
/// reads who is aiming at whom without reading either member's private state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopTargeting {
    /// Opaque member the relationship originates from.
    pub source_peer_id: String,
    /// Opaque member the relationship points at.
    pub target_peer_id: String,
}

/// The public context of one party.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPartyContext {
    /// Phase the party is in.
    pub phase: CoopTurnPhase,
    /// Monotonic public step of this party's turn stream.
    pub step: u64,
    /// Votes the party is taking, or the reason none is observed.
    pub votes: CoopFieldValue<Vec<CoopVote>>,
    /// Targeting relationships between members, or the reason none is observed.
    pub targeting: CoopFieldValue<Vec<CoopTargeting>>,
}
