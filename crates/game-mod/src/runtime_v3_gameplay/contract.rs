// SPDX-License-Identifier: MIT

// Contract mirror: AI-Ascension/sts2-protocol at
// d358d0eacfb82af6902b8fce40cdb26c44764799 (MIT).
// Local extensions: public envelope base, bounded wait constructor, identity value, and the
// host-offered `continue_run` producer admission (sts2-game-mod#172), an additive producer
// extension beside `start_run`; the neutral `action_payload` oneOf is already non-exhaustive.
mod error;
mod local;
mod message;
mod offered_attribute;
mod offered_entry;
mod player;
mod shape;
mod shop;
mod text_bounds;
mod validation;
mod wire;

pub use error::RuntimeV3GameplayValidationError;
pub use local::RuntimeV3GameplayIdentity;
pub use message::{
    RuntimeV3GameplayContext, RuntimeV3GameplayMessage, RuntimeV3GameplayMessageKind,
    RuntimeV3GameplayProvenance,
};
pub use offered_attribute::RUNTIME_V3_GAMEPLAY_MAX_OFFERED_ATTRIBUTE_CHARACTERS;
pub(crate) use offered_entry::validate_choices;
pub use offered_entry::{RuntimeV3GameplayChoice, RuntimeV3GameplayOfferedEntry};
pub use player::{
    RuntimeV3GameplayCard, RuntimeV3GameplayPlayer, RuntimeV3GameplayPotion,
    RuntimeV3GameplayPotionTargetMode, RuntimeV3GameplayRelic,
};
pub use shop::RuntimeV3GameplayShopItem;
#[allow(deprecated)]
pub use text_bounds::RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES;
pub use text_bounds::{
    RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES, RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS,
};
use text_bounds::{valid_identity, valid_text};

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
    "0ae1d4d1525162da3059c028dcdb70df1d4d2dcf9620c5edd9b543e5f04aacc2";
/// Maximum exact JSON-safe generation and lease epoch.
pub const RUNTIME_V3_GAMEPLAY_MAX_GENERATION: u64 = 9_007_199_254_740_991;
/// Maximum number of actions in one complete host-generated catalog.
pub const RUNTIME_V3_GAMEPLAY_MAX_LEGAL_ACTIONS: usize = 256;
/// Maximum number of player-visible cards, inventories, or enemies in one observation.
pub const RUNTIME_V3_GAMEPLAY_MAX_ENTITIES: usize = 256;

fn optional_non_null<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    <T as serde::Deserialize<'de>>::deserialize(deserializer).map(Some)
}

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
        options: Vec<RuntimeV3GameplayChoice>,
    },
    Shop {
        items: Vec<RuntimeV3GameplayShopItem>,
    },
    Event {
        choices: Vec<RuntimeV3GameplayChoice>,
    },
    Rest {
        // Deliberately identity-only, matching the schema.
        options: Vec<String>,
    },
    Selection {
        choices: Vec<RuntimeV3GameplayChoice>,
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
    UsePotion {
        potion_id: String,
        #[serde(deserialize_with = "required_nullable")]
        target_id: Option<String>,
    },
    DiscardPotion {
        potion_id: String,
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
    Proceed,
    ConfirmSelection,
    CancelSelection,
    /// Continues the current compatible resumable run instead of starting a new one.
    /// The host offers this beside `start_run`; `run_id` is host-owned and optional (absent when
    /// the screen already determines the run). The producer never accepts a caller-supplied save
    /// path or run name.
    ContinueRun {
        /// Optional host-owned discriminator for the run being resumed.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        run_id: Option<String>,
    },
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

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    serde::Deserialize::deserialize(deserializer)
}
