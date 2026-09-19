// SPDX-License-Identifier: MIT

/// Exact-build selection family a selector belongs to.
///
/// The named variants are the families the audited source exposes; anything else is carried
/// explicitly as a custom, unsupported, or unknown family rather than folded into one of them.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionKind {
    /// A card selection, as used by combat card play and discard prompts.
    Card,
    /// A player selection, as used by co-op target prompts.
    Player,
    /// A reward selection.
    Reward,
    /// A card removal selection owned by a shop.
    ShopRemoval,
    /// A card upgrade selection.
    Upgrade,
    /// A selection a rest option requires.
    RestOption,
    /// A shared co-op selection.
    Coop,
    /// Owner-defined selection family.
    Custom(String),
    /// A family is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the selection family.
    Unknown,
}

/// Operation whose prompt owns the selector.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionParentOperation {
    /// Playing a card in combat.
    CombatCardPlay,
    /// Claiming a reward.
    RewardClaim,
    /// Removing a card in a shop.
    ShopRemoval,
    /// Upgrading a card.
    CardUpgrade,
    /// Performing a rest option.
    RestOption,
    /// A shared co-op rest action.
    CoopRest,
    /// Answering an event choice.
    EventChoice,
    /// Owner-defined parent operation.
    Custom(String),
    /// Source could not classify the parent operation.
    Unknown,
}

/// Family of entity a candidate resolves to.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SelectionCandidateKind {
    /// A card candidate.
    Card,
    /// A relic candidate.
    Relic,
    /// A potion candidate.
    Potion,
    /// A player candidate.
    Player,
    /// A reward offer candidate.
    RewardItem,
    /// A shop item candidate.
    ShopItem,
    /// A card named as an upgrade target.
    UpgradeTarget,
    /// A rest option named as a candidate.
    RestOption,
    /// Owner-defined candidate family.
    Custom(String),
    /// Source could not classify the candidate family.
    Unknown,
}
