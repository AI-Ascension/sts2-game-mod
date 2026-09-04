// SPDX-License-Identifier: MIT

// Contract mirror: AI-Ascension/sts2-protocol at
// 82507361890c1bdce6cffeaf7e616d93e53a7d99 (MIT).
// Local extensions: public envelope base, bounded wait constructor, and identity value.
mod error;
mod local;
mod message;
mod shape;
mod validation;
mod wire;

pub use error::RuntimeV3GameplayValidationError;
pub use local::RuntimeV3GameplayIdentity;
pub use message::{
    RuntimeV3GameplayContext, RuntimeV3GameplayMessage, RuntimeV3GameplayMessageKind,
    RuntimeV3GameplayProvenance,
};

/// Versioned fair-play semantic gameplay profile.
pub const RUNTIME_V3_GAMEPLAY_PROTOCOL_VERSION: &str = "runtime-v3-gameplay";
/// Release-like artifact identity for the gameplay profile.
pub const RUNTIME_V3_GAMEPLAY_ARTIFACT: &str = "sts2-protocol/runtime-v3-gameplay";
/// Normative source schema path.
pub const RUNTIME_V3_GAMEPLAY_SCHEMA_SOURCE: &str = "schemas/runtime-v3-gameplay.schema.json";
/// Generator recorded in the hand-authored artifact.
pub const RUNTIME_V3_GAMEPLAY_GENERATOR: &str = "hand-authored";
/// Filled after the normative schema is written and hashed.
pub const RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST: &str =
    "b37c80f583aeaf4f81ede2083bcfb4129196baf5eb092470e8738173c4b7226c";
/// Maximum exact JSON-safe generation and lease epoch.
pub const RUNTIME_V3_GAMEPLAY_MAX_GENERATION: u64 = 9_007_199_254_740_991;
/// Maximum number of actions in one complete host-generated catalog.
pub const RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS: usize = 256;
/// Maximum number of player-visible cards or enemies in one observation.
pub const RUNTIME_V3_GAMEPLAY_MAX_ENTITIES: usize = 256;
/// Maximum text/identity field length in bytes.
pub const RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES: usize = 512;

/// Player-visible lifecycle state. Unknown host states must not be coerced into one of these.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayStateKind {
    Setup,
    Map,
    Combat,
    Reward,
    Shop,
    Event,
    Rest,
    Selection,
    Victory,
    Defeat,
    Recovery,
}

/// Combat intent visible to an ordinary player before choosing an action.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", from = "wire::EnemyIntent")]
pub enum RuntimeV3GameplayEnemyIntent {
    Attack { damage: u16, hits: u8 },
    Defend,
    Buff,
    Debuff,
    Unknown,
}

/// A bounded player-visible card description; draw order and unrevealed outcomes are absent.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayCard {
    pub card_id: String,
    pub name: String,
    pub cost: u8,
    pub upgraded: bool,
}

/// A bounded player-visible enemy description.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayEnemy {
    pub enemy_id: String,
    pub name: String,
    pub hp: u16,
    pub max_hp: u16,
    pub intent: RuntimeV3GameplayEnemyIntent,
}

/// Player-visible resources and known card contents.
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
}

/// State-specific player-visible details. No host object, save, RNG, or unrevealed result is
/// representable in this type.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "state", rename_all = "snake_case", from = "wire::State")]
pub enum RuntimeV3GameplayState {
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
    Victory,
    Defeat {
        #[serde(deserialize_with = "required_nullable")]
        reason: Option<String>,
    },
    Recovery {
        code: String,
    },
}

impl RuntimeV3GameplayState {
    /// Returns the state discriminator without interpreting game rules.
    #[must_use]
    pub const fn kind(&self) -> RuntimeV3GameplayStateKind {
        match self {
            Self::Setup { .. } => RuntimeV3GameplayStateKind::Setup,
            Self::Map { .. } => RuntimeV3GameplayStateKind::Map,
            Self::Combat { .. } => RuntimeV3GameplayStateKind::Combat,
            Self::Reward { .. } => RuntimeV3GameplayStateKind::Reward,
            Self::Shop { .. } => RuntimeV3GameplayStateKind::Shop,
            Self::Event { .. } => RuntimeV3GameplayStateKind::Event,
            Self::Rest { .. } => RuntimeV3GameplayStateKind::Rest,
            Self::Selection { .. } => RuntimeV3GameplayStateKind::Selection,
            Self::Victory => RuntimeV3GameplayStateKind::Victory,
            Self::Defeat { .. } => RuntimeV3GameplayStateKind::Defeat,
            Self::Recovery { .. } => RuntimeV3GameplayStateKind::Recovery,
        }
    }
}

/// Player-visible shop item.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayShopItem {
    pub item_id: String,
    pub name: String,
    pub price: u32,
}

/// Complete ordinary player-visible observation. Legal actions are supplied separately so a
/// caller can prove that the catalog and observation use the same generation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayObservation {
    pub state_id: String,
    pub generation: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub visible_seed: Option<String>,
    pub player: RuntimeV3GameplayPlayer,
    pub state: RuntimeV3GameplayState,
}

/// Semantic action payload. Coordinates, arbitrary input events, reflection paths, and process
/// commands are intentionally not variants of this enum.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", from = "wire::Action")]
pub enum RuntimeV3GameplayAction {
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
    EndTurn,
    ChooseReward {
        reward_id: String,
    },
    SkipReward,
    ShopPurchase {
        item_id: String,
    },
    ShopRemove {
        card_id: String,
    },
    Rest,
    Smith {
        card_id: String,
    },
    EventChoice {
        choice_id: String,
    },
    SelectCard {
        card_id: String,
    },
    ConfirmVictory,
    SaveQuit,
}

/// Host-generated action identity plus its typed semantic payload.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayLegalAction {
    pub action_id: String,
    pub action: RuntimeV3GameplayAction,
}

/// An independently checkable postcondition witness. It does not grant authority by itself.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayTransitionWitness {
    pub from_generation: u64,
    pub to_generation: u64,
    pub state_id: String,
    pub effect_kind: String,
}

/// Safe recovery operation. Recovery never selects a strategic gameplay action.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayRecoveryKind {
    Reobserve,
    Reconcile,
    ReleaseLease,
    StopEpisode,
}

/// Bounded recovery request metadata.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeV3GameplayRecovery {
    pub kind: RuntimeV3GameplayRecoveryKind,
    #[serde(deserialize_with = "required_nullable")]
    pub operation_id: Option<String>,
}

/// Wait outcome used to distinguish a stable observation from a timeout.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayWaitOutcome {
    Successor,
    SameStateMutation,
    Timeout,
    RecoveryRequired,
}

/// Lifecycle result used by mutating and recovery operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeV3GameplayStatus {
    Accepted,
    Settled,
    Rejected,
    Unknown,
    Cancelled,
}

fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:/-".contains(&byte))
}

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES
        && !value.chars().any(char::is_control)
}
