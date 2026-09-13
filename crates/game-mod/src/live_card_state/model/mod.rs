// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::identity::validate_identity;

mod cost;
mod projection;

pub use cost::*;
pub use projection::*;

/// Which owner currently holds a live card instance.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardOwnerKind {
    /// The player owns the card.
    Player,
    /// A creature or enemy owns the card.
    Enemy,
    /// A reward, shop, or selector owns the card outside a creature.
    Selector,
    /// The source observed a shared owner.
    Shared,
    /// The source could not classify the owner.
    Unknown,
}

impl CardOwnerKind {
    /// Returns the stable owner-local spelling.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Enemy => "enemy",
            Self::Selector => "selector",
            Self::Shared => "shared",
            Self::Unknown => "unknown",
        }
    }
}

/// Owner identity attached to a card instance.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardOwner {
    /// Owner classification.
    pub kind: CardOwnerKind,
    /// Stable owner identity when the source exposes one.
    pub owner_id: Option<String>,
}

impl CardOwner {
    /// Creates an owner with an optional bounded identity.
    pub fn new(
        kind: CardOwnerKind,
        owner_id: Option<impl Into<String>>,
    ) -> Result<Self, super::LiveCardIdentityError> {
        let owner_id = owner_id.map(Into::into);
        if let Some(value) = &owner_id {
            validate_identity("owner_id", value)?;
        }
        Ok(Self { kind, owner_id })
    }
}

/// A pile with stable owner-local semantics.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardPile {
    /// Cards currently in the hand.
    Hand,
    /// Draw pile.
    Draw,
    /// Discard pile.
    Discard,
    /// Exhaust pile.
    Exhaust,
    /// Persistent deck outside combat.
    Deck,
    /// Reward offer.
    Reward,
    /// Shop offer.
    Shop,
    /// An owner-defined bounded pile kind.
    Other(String),
}

impl CardPile {
    /// Returns a stable owner-local spelling.
    #[must_use]
    pub fn code(&self) -> String {
        match self {
            Self::Hand => "hand".to_owned(),
            Self::Draw => "draw".to_owned(),
            Self::Discard => "discard".to_owned(),
            Self::Exhaust => "exhaust".to_owned(),
            Self::Deck => "deck".to_owned(),
            Self::Reward => "reward".to_owned(),
            Self::Shop => "shop".to_owned(),
            Self::Other(value) => value.clone(),
        }
    }
}

/// Position evidence that does not turn an unobservable position into zero.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardPosition {
    /// Position in the owning pile or selector.
    Known(usize),
    /// The source supports position but did not expose it in this snapshot.
    NotObserved,
    /// This location has no meaningful position.
    NotApplicable,
}

/// Current pile or selector location.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardLocation {
    /// A card in a pile.
    Pile {
        /// Owning pile.
        pile: CardPile,
        /// Position evidence.
        position: CardPosition,
    },
    /// A card selected outside a hand/pile, such as a reward or card selector.
    Selector {
        /// Stable selector identity.
        selector_id: String,
        /// Position in the selector page when observable.
        position: CardPosition,
    },
    /// Location was not available on the owner-local surface.
    Unavailable {
        /// Sanitized reason, not a host path or exception.
        reason: super::LiveCardFieldStatus,
    },
}

/// Upgrade and variant state for one card instance.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CardUpgrade {
    /// Number of upgrades applied to this instance.
    pub count: u16,
    /// Stable owner-defined upgrade path or variant.
    pub variant: Option<String>,
    /// Distinct path identity when the source exposes one.
    pub path: Option<String>,
}

impl CardUpgrade {
    /// Creates bounded upgrade metadata.
    pub fn new(
        count: u16,
        variant: Option<impl Into<String>>,
        path: Option<impl Into<String>>,
    ) -> Result<Self, super::LiveCardIdentityError> {
        let variant = variant.map(Into::into);
        let path = path.map(Into::into);
        if let Some(value) = &variant {
            validate_identity("upgrade_variant", value)?;
        }
        if let Some(value) = &path {
            validate_identity("upgrade_path", value)?;
        }
        Ok(Self {
            count,
            variant,
            path,
        })
    }
}

/// Retain/exhaust/ethereal and other explicitly supported flags.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardFlags {
    /// Card remains in the hand after play.
    pub retained: bool,
    /// Card is exhausted after play.
    pub exhaust: bool,
    /// Card is removed by the end of combat if not played.
    pub ethereal: bool,
    /// Additional owner-defined flags, in deterministic key order.
    pub other: BTreeSet<String>,
}

impl CardFlags {
    /// Creates the ordinary no-flag state.
    #[must_use]
    pub fn none() -> Self {
        Self {
            retained: false,
            exhaust: false,
            ethereal: false,
            other: BTreeSet::new(),
        }
    }
}

/// Typed effect-parameter override value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardEffectValue {
    /// Signed numeric parameter.
    Integer(i64),
    /// Boolean parameter.
    Boolean(bool),
    /// Bounded textual parameter or rule reference.
    Text(String),
    /// Ordered bounded integer parameters.
    IntegerList(Vec<i64>),
    /// Source observed a value but could not classify it.
    Unknown,
}

/// A known expiry condition for a temporary modifier.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardExpiration {
    /// Modifier never expires for this instance.
    Permanent,
    /// Modifier expires after the current turn.
    EndOfTurn,
    /// Modifier expires after the current combat.
    EndOfCombat,
    /// Modifier expires after a bounded number of card plays.
    CardPlays(u16),
    /// Owner-defined bounded condition.
    Condition(String),
    /// Source could not establish an expiry condition.
    Unknown,
}

/// Scope attached to a permanent or temporary modifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardModifierScope {
    /// Applies to the card for its entire owned lifetime.
    Instance,
    /// Applies to the active run.
    Run,
    /// Applies to the active combat.
    Combat,
    /// Applies to the current turn.
    Turn,
}

/// Typed modifier value. Unknown stays explicit rather than becoming zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardModifierValue {
    /// Signed amount.
    Integer(i64),
    /// Boolean toggle.
    Boolean(bool),
    /// Bounded text/rule reference.
    Text(String),
    /// Cost replacement or cost-shape modifier.
    Cost(CardCost),
    /// Marker modifier with no scalar amount.
    Marker,
    /// Source observed a modifier but could not classify it.
    Unknown,
}

/// One ordered permanent or temporary instance modifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardModifier {
    /// Source effect/enchantment/rule identity.
    pub source_ref: String,
    /// Host-semantic ordering; the vector order remains authoritative.
    pub order: u16,
    /// Scope in which the modifier applies.
    pub scope: CardModifierScope,
    /// Optional scalar amount retained separately from the typed value.
    pub amount: Option<i64>,
    /// Typed value or explicit unknown.
    pub value: CardModifierValue,
    /// Expiration condition when known.
    pub expiration: CardExpiration,
}
