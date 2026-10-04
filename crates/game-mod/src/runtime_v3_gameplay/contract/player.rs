// SPDX-License-Identifier: MIT

use super::optional_non_null;

/// A bounded player-visible card description; draw order and unrevealed outcomes are absent.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayCard {
    pub card_id: String,
    pub name: String,
    pub cost: u8,
    pub upgraded: bool,
    /// Host-supplied visible card text; absence means the host did not disclose it.
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
}

/// One relic currently held by the player, described only with host-supplied values.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayRelic {
    pub relic_id: String,
    pub name: String,
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
}

/// Target classification the host supplied for a potion.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayPotionTargetMode {
    #[serde(rename = "self")]
    SelfTarget,
    AnyEnemy,
    AllEnemies,
    None,
    Unknown,
}

/// One potion currently occupying a player belt slot.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayPotion {
    pub potion_id: String,
    pub name: String,
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub slot: Option<u8>,
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub usable: Option<bool>,
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub target_mode: Option<RuntimeV3GameplayPotionTargetMode>,
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
}

/// Player-visible resources, known card contents, and host-observed inventories.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayPlayer {
    pub hp: u16,
    pub max_hp: u16,
    pub energy: u8,
    pub gold: u32,
    pub hand: Vec<RuntimeV3GameplayCard>,
    pub deck: Vec<RuntimeV3GameplayCard>,
    pub discard: Vec<RuntimeV3GameplayCard>,
    pub exhaust: Vec<RuntimeV3GameplayCard>,
    /// `None` means the host did not provide an inventory read; `Some([])` means it read empty.
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub relics: Option<Vec<RuntimeV3GameplayRelic>>,
    /// `None` means the host did not provide an inventory read; `Some([])` means it read empty.
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub potions: Option<Vec<RuntimeV3GameplayPotion>>,
    /// Number of slots in the observed belt, including empty slots; not the occupied-potion count.
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub potion_slots: Option<u8>,
    /// Maximum belt capacity when the host exposes it.
    #[serde(
        default,
        deserialize_with = "optional_non_null",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_potion_slots: Option<u8>,
}
