// SPDX-License-Identifier: MIT

//! Who an event is about, on both ends of it.

use super::namespace::SemanticIdentityNamespace;

/// Which end of an event one subject names.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticSubjectRole {
    /// The subject that caused the event.
    Actor,
    /// The subject the event acted on.
    Target,
}

impl SemanticSubjectRole {
    /// Every role, in a stable order.
    pub const ALL: [Self; 2] = [Self::Actor, Self::Target];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Actor => "actor",
            Self::Target => "target",
        }
    }
}

/// One named end of an event: the subject, its namespace, and its role.
///
/// The namespace travels with the identity so a consumer can never read an actor that happens to
/// share a token with a definition as that definition. A missing target is expressed by the kind
/// refusing the event, not by an empty subject here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEventSubject {
    /// Which end of the event this subject names.
    pub role: SemanticSubjectRole,
    /// Namespace the identity was minted in.
    pub namespace: SemanticIdentityNamespace,
    /// Opaque subject identity.
    pub subject_id: String,
}
