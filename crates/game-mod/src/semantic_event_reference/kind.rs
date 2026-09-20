// SPDX-License-Identifier: MIT

//! The closed inventory of semantic event kinds this boundary can state.

/// One kind of authoritative gameplay event.
///
/// The inventory is closed: a kind this boundary cannot name is `Unsupported` coverage rather than a
/// free-form string, so a consumer never has to interpret an owner-defined label to decide whether an
/// event is understood. Each kind is observed at the host boundary; none is derived from comparing
/// two snapshots.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticEventKind {
    /// A card was played, optionally naming its resolved target.
    CardPlayed,
    /// Damage was dealt.
    Damage,
    /// Block was gained.
    Block,
    /// Health was restored.
    Heal,
    /// A character-specific resource changed.
    ResourceChanged,
    /// A status or power was applied.
    StatusApplied,
    /// A status or power was removed.
    StatusRemoved,
    /// A modifier was applied.
    ModifierApplied,
    /// A modifier was removed.
    ModifierRemoved,
    /// A card moved between piles.
    PileMoved,
    /// The party moved to another room or act.
    RoomTransitioned,
    /// A choice was made, including a reward or event decision.
    ChoiceMade,
    /// An offer was presented and is inspectable.
    OfferPresented,
    /// A purchase was made.
    PurchaseMade,
}

impl SemanticEventKind {
    /// Every kind this boundary can state, in a stable order.
    pub const ALL: [Self; 14] = [
        Self::CardPlayed,
        Self::Damage,
        Self::Block,
        Self::Heal,
        Self::ResourceChanged,
        Self::StatusApplied,
        Self::StatusRemoved,
        Self::ModifierApplied,
        Self::ModifierRemoved,
        Self::PileMoved,
        Self::RoomTransitioned,
        Self::ChoiceMade,
        Self::OfferPresented,
        Self::PurchaseMade,
    ];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::CardPlayed => "card_played",
            Self::Damage => "damage",
            Self::Block => "block",
            Self::Heal => "heal",
            Self::ResourceChanged => "resource_changed",
            Self::StatusApplied => "status_applied",
            Self::StatusRemoved => "status_removed",
            Self::ModifierApplied => "modifier_applied",
            Self::ModifierRemoved => "modifier_removed",
            Self::PileMoved => "pile_moved",
            Self::RoomTransitioned => "room_transitioned",
            Self::ChoiceMade => "choice_made",
            Self::OfferPresented => "offer_presented",
            Self::PurchaseMade => "purchase_made",
        }
    }

    /// Returns whether an event of this kind must name a target subject.
    ///
    /// A targeted kind without a target is a refusal rather than an event whose target is unknown, so
    /// a consumer cannot read a missing target as "no target existed".
    #[must_use]
    pub const fn requires_target(self) -> bool {
        matches!(
            self,
            Self::Damage
                | Self::Block
                | Self::Heal
                | Self::StatusApplied
                | Self::StatusRemoved
                | Self::ModifierApplied
                | Self::ModifierRemoved
        )
    }

    /// Returns whether this kind may carry a causal parent.
    ///
    /// Only an effect can have a cause; a room transition or an offer has no causal parent to state,
    /// so one supplied for such a kind is refused rather than recorded as unverified causality.
    #[must_use]
    pub const fn admits_cause(self) -> bool {
        matches!(
            self,
            Self::Damage
                | Self::Block
                | Self::Heal
                | Self::ResourceChanged
                | Self::StatusApplied
                | Self::StatusRemoved
                | Self::ModifierApplied
                | Self::ModifierRemoved
                | Self::PileMoved
                | Self::ChoiceMade
                | Self::PurchaseMade
        )
    }

    /// Returns whether an event of this kind must state a bounded quantity.
    ///
    /// A kind that reports how much happened without saying how much would be closed by a zeroed
    /// quantity, which is exactly the invented value this vocabulary refuses.
    #[must_use]
    pub const fn requires_quantity(self) -> bool {
        matches!(
            self,
            Self::Damage
                | Self::Block
                | Self::Heal
                | Self::ResourceChanged
                | Self::StatusApplied
                | Self::StatusRemoved
                | Self::ModifierApplied
                | Self::ModifierRemoved
        )
    }

    /// Returns whether an event of this kind must name a content identity.
    #[must_use]
    pub const fn requires_reference(self) -> bool {
        matches!(
            self,
            Self::CardPlayed | Self::PileMoved | Self::PurchaseMade | Self::OfferPresented
        )
    }

    /// Returns whether an event of this kind must name the actor that caused it.
    ///
    /// An effect that cannot name who caused it is not an authoritative event, so the actor is
    /// required where the boundary observed one and left optional only where an offer or a room
    /// transition has no acting subject to name.
    #[must_use]
    pub const fn requires_actor(self) -> bool {
        matches!(
            self,
            Self::CardPlayed
                | Self::Damage
                | Self::Block
                | Self::Heal
                | Self::ResourceChanged
                | Self::StatusApplied
                | Self::StatusRemoved
                | Self::ModifierApplied
                | Self::ModifierRemoved
                | Self::PileMoved
                | Self::ChoiceMade
                | Self::PurchaseMade
        )
    }
}
