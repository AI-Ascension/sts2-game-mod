// SPDX-License-Identifier: MIT

//! Owner-local availability and bounded detail recovery.
//!
//! These values describe the game-mod extraction boundary before transport mapping. They are
//! intentionally not a protocol schema: the protocol owner may select different wire names while
//! preserving the distinctions and fences represented here.

mod engine;
mod helpers;
mod model;
mod query;
mod schema;

pub use engine::LocalAvailabilityStore;
pub use model::*;
pub use query::{LocalBasicQuery, LocalKindFixture};
pub use schema::*;

/// Synthetic bounds used by deterministic source-only fixtures.
pub const LOCAL_AVAILABILITY_MAX_PAGE_ITEMS: usize = 64;
/// Synthetic detail response bound used before transport negotiation.
pub const LOCAL_AVAILABILITY_MAX_DETAIL_BYTES: usize = 4 * 1024;
