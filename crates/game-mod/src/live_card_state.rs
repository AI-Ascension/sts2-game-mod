// SPDX-License-Identifier: MIT

//! Source-only owner-local projection of live card instances.
//!
//! This boundary keeps a card definition reference separate from a live instance reference.
//! Every page, detail read, modifier, and cost is bound to one immutable
//! [`LiveCardReadReference`].  The in-memory snapshot and fixture source are deterministic
//! contract witnesses only; they do not inspect native host objects or advertise a transport
//! route.

mod error;
mod identity;
mod model;
mod port;
mod store;

pub use error::{LiveCardError, LiveCardField, LiveCardFieldStatus, LiveCardUnavailableReason};
pub use identity::{
    CardDefinitionReference, CardInstanceReference, LiveCardIdentityError, LiveCardReadReference,
};
pub use model::LiveCardDetail;
pub use model::{
    CardCost, CardCostAmount, CardCostContributor, CardCostSemantics, CardCostUnknownReason,
    CardDefinitionLink, CardEffectValue, CardExpiration, CardFlags, CardLocation, CardModifier,
    CardModifierScope, CardModifierValue, CardOwner, CardOwnerKind, CardPile, CardPosition,
    CardUpgrade, LiveCardCollection, LiveCardContinuation, LiveCardFixture, LiveCardPage,
    LiveCardPageCompleteness, LiveCardProjection, LiveCardQuery, LiveCardSnapshot, LiveCardValue,
};
pub use port::{
    FixtureLiveCardSource, LiveCardCapability, LiveCardSource, UnavailableLiveCardSource,
    default_live_card_bounds,
};
pub use store::LiveCardStore;

/// Owner-local producer version. This is not a protocol version.
pub const LIVE_CARD_STATE_PRODUCER_VERSION: &str = "game-live-card-state-v1";
/// Maximum bytes for one identity or owner-defined reference.
pub const LIVE_CARD_MAX_ID_BYTES: usize = 256;
/// Maximum bytes for one bounded text value.
pub const LIVE_CARD_MAX_TEXT_BYTES: usize = 4 * 1024;
/// Maximum entries in one owner-local page.
pub const LIVE_CARD_MAX_PAGE_ITEMS: usize = 64;
/// Maximum modifiers attached to one card instance.
pub const LIVE_CARD_MAX_MODIFIERS: usize = 64;
/// Maximum effect-parameter overrides attached to one card instance.
pub const LIVE_CARD_MAX_EFFECT_OVERRIDES: usize = 64;
/// Maximum encoded detail bytes accepted for one card instance.
pub const LIVE_CARD_MAX_DETAIL_BYTES: usize = 16 * 1024;
