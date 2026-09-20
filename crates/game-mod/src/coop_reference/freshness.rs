// SPDX-License-Identifier: MIT

//! Peer membership and data freshness, kept distinct from the gameplay values they qualify.

/// Membership state of one peer in the party.
///
/// A join, a disconnect and a rejoin are three different states rather than one absent peer,
/// because only one of them may carry gameplay values and a rejoin starts a new peer generation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopPeerMembership {
    /// The peer is joining and has no coherent gameplay snapshot yet.
    Joining,
    /// The peer is present and its gameplay data is coherent.
    Active,
    /// The peer's connection was lost; its retained data is not current.
    Disconnected,
    /// The peer left the party deliberately.
    Left,
}

impl CoopPeerMembership {
    /// Returns whether this membership may carry a coherent gameplay value.
    #[must_use]
    pub const fn carries_gameplay(self) -> bool {
        matches!(self, Self::Active)
    }
}

/// Freshness of the data reported for one peer.
///
/// A peer whose data is lagging or unavailable is not published as current, because an older
/// value presented without its freshness is indistinguishable from the present state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopPeerFreshness {
    /// The value was observed in the snapshot the fence names.
    Current,
    /// The value was observed in an earlier snapshot and is retained as stale.
    Lagging,
    /// No value is retained for this peer.
    Unavailable,
}

impl CoopPeerFreshness {
    /// Returns whether a value with this freshness may be published as current.
    #[must_use]
    pub const fn is_current(self) -> bool {
        matches!(self, Self::Current)
    }
}
