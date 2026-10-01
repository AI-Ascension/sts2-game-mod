// SPDX-License-Identifier: MIT

//! Mirrored from `sts2-protocol`: the player-visible shop item.
//!
//! The mod re-declares the neutral shapes rather than depending on the owner crate, so this file
//! exists only because splitting it keeps the mirrored `contract.rs` inside the repository's size
//! budget. Its content is a verbatim mirror with no local extension.

/// Player-visible shop item.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayShopItem {
    pub item_id: String,
    pub name: String,
    pub price: u32,
}
