// SPDX-License-Identifier: MIT

//! The identity namespaces kept distinct inside one history.

/// Which namespace an identity belongs to.
///
/// The accepted query contract keeps definition identities, live instance identities and action
/// identities distinct. This vocabulary therefore records the namespace each identity was minted in
/// rather than comparing identities as bare strings: two equal tokens from different namespaces are a
/// collision to refuse, never an aliasing to accept.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticIdentityNamespace {
    /// A static definition identity resolved through the content manifest.
    Definition,
    /// A live identity scoped to one run, branch, episode or epoch.
    LiveInstance,
    /// An action identity from the action vocabulary.
    Action,
    /// An event identity minted by this history.
    Event,
}

impl SemanticIdentityNamespace {
    /// Every namespace, in a stable order.
    pub const ALL: [Self; 4] = [
        Self::Definition,
        Self::LiveInstance,
        Self::Action,
        Self::Event,
    ];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::LiveInstance => "live_instance",
            Self::Action => "action",
            Self::Event => "event",
        }
    }

    /// Returns whether a subject of an event may be minted in this namespace.
    ///
    /// An actor or a target of an event is something that exists in the live run, so it belongs to
    /// the live-instance namespace. A definition identity, an action identity or another event
    /// identity is not a subject, and accepting one would let an event name a definition as though it
    /// were the thing that acted.
    #[must_use]
    pub const fn admits_subject_role(self) -> bool {
        matches!(self, Self::LiveInstance)
    }
}
