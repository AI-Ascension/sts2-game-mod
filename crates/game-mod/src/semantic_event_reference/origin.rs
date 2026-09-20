// SPDX-License-Identifier: MIT

//! Where one event came from, kept distinct from what it says.

/// The origin of one recorded event.
///
/// Origin is stated rather than assumed, because native, derived and imported history carry different
/// warrant. A derived event is one the host itself computed; an imported event is one restored from
/// saved history through an owned port. Neither is promoted to native here.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticEventOrigin {
    /// Observed directly at the host boundary during this capture.
    Native,
    /// Computed by the owner from its own observed state, not observed as an event.
    Derived,
    /// Restored from saved history through an owned mod port.
    Imported,
}

impl SemanticEventOrigin {
    /// Every origin, in a stable order.
    pub const ALL: [Self; 3] = [Self::Native, Self::Derived, Self::Imported];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Derived => "derived",
            Self::Imported => "imported",
        }
    }

    /// Returns whether this origin may state an explicit causal parent.
    ///
    /// An imported event's causality was settled when it was captured; re-deriving a parent for it at
    /// import time would be exactly the snapshot-difference inference this vocabulary refuses. An
    /// imported event therefore carries no stated parent and is marked not stated.
    #[must_use]
    pub const fn admits_stated_parent(self) -> bool {
        matches!(self, Self::Native | Self::Derived)
    }
}
