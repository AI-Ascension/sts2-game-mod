// SPDX-License-Identifier: MIT

//! Source-only co-op party state: membership, per-field visibility, piles, shared effects and
//! scaling, copied into an immutable catalog fenced by the existing content manifest and locale.
//!
//! A co-op party is other members' state, so the module's contract is refusal rather than
//! disclosure: every field carries its own availability, a local-only value is never carried by an
//! ally, a local-only pile is `NotPermitted` for an ally instead of an empty pile, a member that is
//! not active carries no gameplay value, a lagging member's data is not published as current, and a
//! targeted effect must name a member the party actually declares. The party identity is scoped to
//! one live instance and one run and never to the single-player save profile, so a party read
//! selects no profile and touches no save. Synchronization receipts, authority and epoch remain
//! owned by the modules that already hold them and are not republished here.
//!
//! No native co-op read, transport route, or host compatibility is claimed here.

mod catalog;
mod catalog_reader;
mod context;
mod definition;
mod effects;
mod error;
mod field;
mod freshness;
mod identity;
mod model;
mod piles;
mod profile;
mod validation;
mod value;
mod view;
mod visibility;

pub use catalog::{CoopCatalogProducer, CoopCatalogSnapshot, CoopCatalogSource};
pub use catalog_reader::*;
pub use context::{CoopPartyContext, CoopTargeting, CoopTurnPhase, CoopVote, CoopVoteChoice};
pub use definition::{
    CoopEntry, CoopEntryKind, CoopPartyInput, CoopPartyRecord, CoopPeerInput, CoopPeerRecord,
};
pub use effects::{CoopEffectScope, CoopScalingKind, CoopScalingRule, CoopSharedEffect};
pub use error::{CoopError, CoopReadAuthority, CoopSourceError};
pub use field::{CoopField, CoopFieldStatus};
pub use freshness::{CoopPeerFreshness, CoopPeerMembership};
pub use identity::is_opaque_coop_identity;
pub use model::*;
pub use piles::{CoopPileContents, CoopPileKind, CoopPileView};
pub use profile::CoopLocalView;
pub use value::{CoopFieldValue, CoopHealth, CoopQuantity, CoopResource, CoopUnit};
pub use view::{CoopPartySummary, CoopPartyView, CoopPeerView};
pub use visibility::{CoopFieldVisibility, CoopPeerRole, CoopReadScope};
