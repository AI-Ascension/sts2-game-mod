// SPDX-License-Identifier: MIT

// Struct variants, including empty variants, enforce closed JSON objects. Serde unit
// variants otherwise ignore extra members even with deny_unknown_fields on the enum.
use super::*;

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum EnemyIntent {
    Attack { damage: u16, hits: u8 },
    Defend {},
    Buff {},
    Debuff {},
    Unknown {},
}

impl From<EnemyIntent> for RuntimeV3GameplayEnemyIntent {
    fn from(value: EnemyIntent) -> Self {
        match value {
            EnemyIntent::Attack { damage, hits } => Self::Attack { damage, hits },
            EnemyIntent::Defend {} => Self::Defend,
            EnemyIntent::Buff {} => Self::Buff,
            EnemyIntent::Debuff {} => Self::Debuff,
            EnemyIntent::Unknown {} => Self::Unknown,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum State {
    Setup {
        characters: Vec<String>,
    },
    Map {
        #[serde(deserialize_with = "required_nullable")]
        node_id: Option<String>,
        options: Vec<String>,
    },
    Combat {
        turn_index: u16,
        enemies: Vec<RuntimeV3GameplayEnemy>,
    },
    Reward {
        options: Vec<String>,
    },
    Shop {
        items: Vec<RuntimeV3GameplayShopItem>,
    },
    Event {
        choices: Vec<String>,
    },
    Rest {
        options: Vec<String>,
    },
    Selection {
        choices: Vec<String>,
    },
    Victory {},
    Defeat {
        #[serde(deserialize_with = "required_nullable")]
        reason: Option<String>,
    },
    Recovery {
        code: String,
    },
}

impl From<State> for RuntimeV3GameplayState {
    fn from(value: State) -> Self {
        match value {
            State::Setup { characters } => Self::Setup { characters },
            State::Map { node_id, options } => Self::Map { node_id, options },
            State::Combat {
                turn_index,
                enemies,
            } => Self::Combat {
                turn_index,
                enemies,
            },
            State::Reward { options } => Self::Reward { options },
            State::Shop { items } => Self::Shop { items },
            State::Event { choices } => Self::Event { choices },
            State::Rest { options } => Self::Rest { options },
            State::Selection { choices } => Self::Selection { choices },
            State::Victory {} => Self::Victory,
            State::Defeat { reason } => Self::Defeat { reason },
            State::Recovery { code } => Self::Recovery { code },
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Action {
    StartRun {
        character_id: String,
    },
    SelectMapNode {
        node_id: String,
    },
    PlayCard {
        card_id: String,
        #[serde(deserialize_with = "required_nullable")]
        target_id: Option<String>,
    },
    EndTurn {},
    ChooseReward {
        reward_id: String,
    },
    SkipReward {},
    ShopPurchase {
        item_id: String,
    },
    ShopRemove {
        card_id: String,
    },
    Rest {},
    Smith {
        card_id: String,
    },
    EventChoice {
        choice_id: String,
    },
    SelectCard {
        card_id: String,
    },
    ConfirmVictory {},
    SaveQuit {},
}

impl From<Action> for RuntimeV3GameplayAction {
    fn from(value: Action) -> Self {
        match value {
            Action::StartRun { character_id } => Self::StartRun { character_id },
            Action::SelectMapNode { node_id } => Self::SelectMapNode { node_id },
            Action::PlayCard { card_id, target_id } => Self::PlayCard { card_id, target_id },
            Action::EndTurn {} => Self::EndTurn,
            Action::ChooseReward { reward_id } => Self::ChooseReward { reward_id },
            Action::SkipReward {} => Self::SkipReward,
            Action::ShopPurchase { item_id } => Self::ShopPurchase { item_id },
            Action::ShopRemove { card_id } => Self::ShopRemove { card_id },
            Action::Rest {} => Self::Rest,
            Action::Smith { card_id } => Self::Smith { card_id },
            Action::EventChoice { choice_id } => Self::EventChoice { choice_id },
            Action::SelectCard { card_id } => Self::SelectCard { card_id },
            Action::ConfirmVictory {} => Self::ConfirmVictory,
            Action::SaveQuit {} => Self::SaveQuit,
        }
    }
}
