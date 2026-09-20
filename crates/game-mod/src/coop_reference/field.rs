// SPDX-License-Identifier: MIT

//! The closed inventory of a peer's gameplay fields and the availability of each row.

use super::CoopFieldVisibility;

/// Field of one peer's co-op gameplay state.
///
/// The inventory is closed so a field the source does not project is a stated coverage failure
/// rather than a silently absent key.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopField {
    /// The character this peer is playing.
    Character,
    /// Current and maximum health.
    Health,
    /// Character-specific resources such as energy, focus or a secondary meter.
    Resources,
    /// Permitted piles, each carrying only the cards this scope may observe.
    Piles,
    /// Relics this peer holds.
    Relics,
    /// Potions this peer holds.
    Potions,
    /// Powers and statuses affecting this peer.
    Powers,
    /// Special mechanics the supported build reports for this peer.
    SpecialMechanics,
    /// Whether this peer has signalled readiness for the current public step.
    Readiness,
}

impl CoopField {
    /// Every field a peer row must state, in deterministic order.
    pub const ALL: [Self; 9] = [
        Self::Character,
        Self::Health,
        Self::Resources,
        Self::Piles,
        Self::Relics,
        Self::Potions,
        Self::Powers,
        Self::SpecialMechanics,
        Self::Readiness,
    ];

    /// Returns the visibility the supported build gives this field.
    ///
    /// The default is a property of the field, not of the reader: possession by a host that stores
    /// every member's state is not permission to publish it.
    #[must_use]
    pub const fn default_visibility(self) -> CoopFieldVisibility {
        match self {
            Self::Character | Self::Health | Self::Resources | Self::Powers | Self::Readiness => {
                CoopFieldVisibility::PublicToParty
            }
            Self::Relics | Self::SpecialMechanics => CoopFieldVisibility::ExplicitlyShared,
            Self::Potions => CoopFieldVisibility::LocalOnly,
            Self::Piles => CoopFieldVisibility::PublicToParty,
        }
    }

    /// Returns the stable name used in failures.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Character => "character",
            Self::Health => "health",
            Self::Resources => "resources",
            Self::Piles => "piles",
            Self::Relics => "relics",
            Self::Potions => "potions",
            Self::Powers => "powers",
            Self::SpecialMechanics => "special_mechanics",
            Self::Readiness => "readiness",
        }
    }
}

/// Availability of one peer gameplay field.
///
/// `Present` is the only status that carries a value, so an unknown value never becomes a zero, an
/// empty collection or an invented description, and permission and freshness stay distinguishable
/// from absence and from an unsupported build.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopFieldStatus {
    /// A value was reported.
    Present,
    /// The host reports the field as absent for this peer.
    Absent,
    /// The supported build does not carry the field.
    Unsupported,
    /// The field exists but the owner withholds it.
    Withheld,
    /// The field exists but this scope is not permitted to observe it.
    NotPermitted,
    /// The retained value is older than the snapshot the fence names.
    Stale,
}

impl CoopFieldStatus {
    /// Returns whether a value may be published for this status.
    #[must_use]
    pub const fn is_present(self) -> bool {
        matches!(self, Self::Present)
    }

    /// Returns whether the status states a reason the field carries no value.
    #[must_use]
    pub const fn is_stated(self) -> bool {
        !self.is_present()
    }
}
