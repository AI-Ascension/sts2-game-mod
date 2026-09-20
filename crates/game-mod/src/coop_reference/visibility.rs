// SPDX-License-Identifier: MIT

//! Per-field ownership and visibility, so another member's private state is never assumed.

/// Who may observe a co-op gameplay field.
///
/// The host stores every member's state, so possession by the host is not evidence of permission
/// to publish it. The four classes answer four different questions and stay distinguishable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopFieldVisibility {
    /// Any member of the party may observe it.
    PublicToParty,
    /// The owning member explicitly shares it with the party.
    ExplicitlyShared,
    /// Only the owning member's own view may observe it.
    LocalOnly,
    /// The supported build publishes nothing for this field.
    Unavailable,
}

/// Scope a caller reads a party from.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopReadScope {
    /// The local member's own view, which may observe its own local-only fields.
    Peer,
    /// The shared party view, which never observes any member's local-only fields.
    Party,
}

/// Whether a peer is the observing member or another member of the same party.
///
/// The role is separate from the identity because only the local member may carry local-only
/// values, so a record that grants an ally local-only access is refused rather than filtered.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopPeerRole {
    /// The member this read observes from.
    Local,
    /// Another member of the same party.
    Ally,
}

impl CoopReadScope {
    /// Returns whether this scope may observe a peer field with the given role and visibility.
    ///
    /// A local-only field is observable only from the local member's own view, so the party scope
    /// and every ally are refused it, and an unavailable field is observable from nowhere.
    #[must_use]
    pub const fn observes_peer_field(
        self,
        role: CoopPeerRole,
        visibility: CoopFieldVisibility,
    ) -> bool {
        match visibility {
            CoopFieldVisibility::PublicToParty | CoopFieldVisibility::ExplicitlyShared => true,
            CoopFieldVisibility::LocalOnly => {
                matches!(self, Self::Peer) && matches!(role, CoopPeerRole::Local)
            }
            CoopFieldVisibility::Unavailable => false,
        }
    }

    /// Returns whether a caller at this scope may be observing the given member at all.
    #[must_use]
    pub const fn observes_peer(self, role: CoopPeerRole) -> bool {
        match self {
            Self::Peer => true,
            Self::Party => matches!(role, CoopPeerRole::Ally),
        }
    }
}
