// SPDX-License-Identifier: MIT

//! The five piles, each carrying its own visibility rather than one party-wide answer.

use super::{CoopFieldValue, CoopFieldVisibility};

/// Kind of a pile one member's state is divided into.
///
/// A pile is not a single collection with one visibility: the discard pile is shared while the hand
/// is not, so each kind answers the visibility question for itself.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopPileKind {
    /// Cards not yet drawn.
    Draw,
    /// Cards held by the member.
    Hand,
    /// Cards discarded this combat.
    Discard,
    /// Cards removed from the combat.
    Exhaust,
    /// Cards currently being played.
    Play,
}

impl CoopPileKind {
    /// Every pile a member row must state, in deterministic order.
    pub const ALL: [Self; 5] = [
        Self::Draw,
        Self::Hand,
        Self::Discard,
        Self::Exhaust,
        Self::Play,
    ];

    /// Returns the visibility the supported build gives this pile.
    #[must_use]
    pub const fn default_visibility(self) -> CoopFieldVisibility {
        match self {
            Self::Draw | Self::Hand => CoopFieldVisibility::LocalOnly,
            Self::Discard | Self::Exhaust | Self::Play => CoopFieldVisibility::PublicToParty,
        }
    }

    /// Returns whether this pile may be observed only by its own member.
    #[must_use]
    pub const fn is_local(self) -> bool {
        matches!(self.default_visibility(), CoopFieldVisibility::LocalOnly)
    }

    /// Returns the stable name used in failures.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Draw => "draw",
            Self::Hand => "hand",
            Self::Discard => "discard",
            Self::Exhaust => "exhaust",
            Self::Play => "play",
        }
    }
}

/// The cards one pile holds, with the count the host reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPileContents {
    /// Number of cards the host reports in this pile.
    pub count: u32,
    /// Cards this scope may observe, in host order.
    pub cards: Vec<String>,
}

/// One pile of one member with its visibility and stated availability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopPileView {
    /// Which pile this row states.
    pub kind: CoopPileKind,
    /// Visibility the row declares, which must match the kind's own.
    pub visibility: CoopFieldVisibility,
    /// Observed contents, or the reason the row carries none.
    pub contents: CoopFieldValue<CoopPileContents>,
}

impl CoopPileView {
    /// States the contents of a pile the source reports.
    #[must_use]
    pub fn reported(kind: CoopPileKind, count: u32, cards: Vec<String>) -> Self {
        Self {
            kind,
            visibility: kind.default_visibility(),
            contents: CoopFieldValue::present(CoopPileContents { count, cards }),
        }
    }

    /// States that this scope is not permitted to observe the pile.
    ///
    /// The row keeps its kind and visibility, so an ally reads "the hand exists and is not yours to
    /// see" rather than an empty hand.
    #[must_use]
    pub const fn not_permitted(kind: CoopPileKind) -> Self {
        Self {
            kind,
            visibility: kind.default_visibility(),
            contents: CoopFieldValue::not_permitted(),
        }
    }
}
