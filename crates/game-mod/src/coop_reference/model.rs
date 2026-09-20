// SPDX-License-Identifier: MIT

//! Catalog binding, declared support state, and the live fence a current-party read holds.

use crate::ContentCursorBinding;

/// Source-only producer identity; this is not a wire or native ABI version.
pub const COOP_REFERENCE_PRODUCER_VERSION: &str = "game-coop-reference-producer-v1";
/// Maximum bytes accepted for one opaque identity.
pub const COOP_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const COOP_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one party record.
pub const COOP_MAX_PARTY_BYTES: usize = 256 * 1024;
/// Maximum members in one party.
pub const COOP_MAX_PEERS: usize = 16;
/// Maximum piles stated for one member.
pub const COOP_MAX_PILES: usize = 5;
/// Maximum character-specific resources stated for one member.
pub const COOP_MAX_RESOURCES: usize = 16;
/// Maximum relics, potions, powers or special mechanics stated for one member.
pub const COOP_MAX_ENTRIES: usize = 256;
/// Maximum cards stated inside one pile.
pub const COOP_MAX_PILE_CARDS: usize = 256;
/// Maximum shared effects in one party.
pub const COOP_MAX_EFFECTS: usize = 256;
/// Maximum scaling rules in one party.
pub const COOP_MAX_SCALING_RULES: usize = 64;
/// Maximum entries returned by one bounded page.
pub const COOP_MAX_PAGE_ITEMS: usize = 64;
/// Maximum votes stated for one party.
pub const COOP_MAX_VOTES: usize = 16;
/// Maximum options stated for one vote.
pub const COOP_MAX_VOTE_CHOICES: usize = 16;
/// Maximum targeting relationships stated for one party.
pub const COOP_MAX_TARGETS: usize = 64;
/// Manifest family resolved for a member's character reference.
pub const COOP_CHARACTER_KIND: &str = "character";
/// Manifest family resolved for relic references.
pub const COOP_RELIC_KIND: &str = "relic";
/// Manifest family resolved for potion references.
pub const COOP_POTION_KIND: &str = "potion";
/// Manifest family resolved for power references.
pub const COOP_POWER_KIND: &str = "power";
/// Manifest family resolved for shared-effect and scaling-target references.
pub const COOP_EFFECT_KIND: &str = "effect";
/// Manifest family resolved for an owner-defined special mechanic.
pub const COOP_SPECIAL_MECHANIC_KIND: &str = "special_mechanic";
/// Manifest family resolved for a card identity inside one pile.
pub const COOP_CARD_KIND: &str = "card";

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoopCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized party value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Declared support state for the party family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CoopFamilyState {
    /// The source reports a party record.
    Handled,
    /// The supported build reports no party, so the catalog is explicitly empty.
    Unavailable,
}

/// Declared support state with the counts the source reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoopFamilyCoverage {
    /// Whether the source reports a party at all.
    pub state: CoopFamilyState,
    /// Number of peers the source declares.
    pub peer_count: usize,
    /// Number of shared effects the source declares.
    pub effect_count: usize,
    /// Number of scaling rules the source declares.
    pub scaling_count: usize,
}

/// The live fence a current-party read holds.
///
/// A live party is the instance's own membership, so reading it requires the instance, party and
/// epoch the observation was taken at. A retained party read does not require it and does not move
/// it.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoopLiveFence {
    /// Opaque instance identity the observation was taken from.
    pub instance_id: String,
    /// Opaque party identity the observation was taken from.
    pub party_id: String,
    /// Monotonic epoch of that instance's observation stream.
    pub epoch: u64,
}

/// Exact reference to one retained party record.
///
/// The reference carries its catalog witness, so a reference produced for another manifest, locale
/// or producer is refused instead of being resolved against the current snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoopPartyReference {
    /// Catalog binding this reference was produced for.
    pub catalog: CoopCatalogBinding,
    /// Opaque party identity.
    pub party_id: String,
}

/// Exact reference to one member of one retained party.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CoopPeerReference {
    /// Catalog binding this reference was produced for.
    pub catalog: CoopCatalogBinding,
    /// Opaque party identity this member belongs to.
    pub party_id: String,
    /// Opaque member identity.
    pub peer_id: String,
    /// Membership generation this reference was produced for.
    ///
    /// A rejoin starts a new generation, so a reference minted before it names a member that no
    /// longer exists and is refused rather than resolved against the rejoined member.
    pub generation: u64,
}
