// SPDX-License-Identifier: MIT

//! Typed placeholders for the ADR 0037 coverage families.
//!
//! Every type here is a bounded, owned value model. Field names are coverage
//! categories from the source-derived inventory; they are not claims about
//! proprietary member names, and holding a value proves nothing about a host.

/// Maximum UTF-8 bytes of one identifier-shaped payload text.
pub const PAYLOAD_MAX_IDENTIFIER_BYTES: usize = 128;
/// Maximum gameplay-affecting RNG streams in one payload.
pub const PAYLOAD_MAX_RNG_STREAMS: usize = 32;
/// Maximum 64-bit state words per RNG stream.
pub const PAYLOAD_MAX_RNG_STATE_WORDS: usize = 8;
/// Maximum acts in a run's act sequence.
pub const PAYLOAD_MAX_ACTS: usize = 8;
/// Maximum run modifiers.
pub const PAYLOAD_MAX_MODIFIERS: usize = 64;
/// Maximum visited map nodes.
pub const PAYLOAD_MAX_VISITED_NODES: usize = 256;
/// Maximum held keys.
pub const PAYLOAD_MAX_KEYS: usize = 8;
/// Maximum card instances, deck references, or references in one pile.
pub const PAYLOAD_MAX_CARDS: usize = 1024;
/// Maximum ordered piles.
pub const PAYLOAD_MAX_PILES: usize = 8;
/// Maximum temporary values on one card instance.
pub const PAYLOAD_MAX_TEMPORARY_VALUES: usize = 32;
/// Maximum relics.
pub const PAYLOAD_MAX_RELICS: usize = 128;
/// Maximum occupied potion slots.
pub const PAYLOAD_MAX_POTION_SLOTS: usize = 16;
/// Maximum power entries across all owners.
pub const PAYLOAD_MAX_POWERS: usize = 256;
/// Maximum enemies in one encounter.
pub const PAYLOAD_MAX_ENEMIES: usize = 16;
/// Maximum intents per enemy.
pub const PAYLOAD_MAX_INTENTS: usize = 8;
/// Maximum declared external inputs.
pub const PAYLOAD_MAX_EXTERNAL_INPUTS: usize = 32;

/// One gameplay-affecting RNG stream cursor and state (ADR 0040 rows).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadRngStream {
    /// Stable stream identity; distinct within one payload.
    pub stream_id: String,
    /// Algorithm or version token observed for the stream.
    pub algorithm: String,
    /// Exact draw cursor.
    pub cursor: u64,
    /// Exact state words in stream order (at least one).
    pub state_words: Vec<u64>,
}

/// Seed and RNG closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadSeedAndRng {
    /// Canonical master seed text.
    pub master_seed: String,
    /// Seed derivation version token.
    pub derivation_version: String,
    /// Every gameplay-affecting stream in producer order.
    pub streams: Vec<PayloadRngStream>,
}

/// Character, ascension, mode, acts, and modifiers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadRunConfiguration {
    /// Character identity.
    pub character: String,
    /// Ascension level.
    pub ascension: u64,
    /// Run mode token.
    pub mode: String,
    /// Ordered act identities (at least one).
    pub act_sequence: Vec<String>,
    /// Ordered run modifiers.
    pub modifiers: Vec<String>,
}

/// Map and room progress.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadCampaignProgress {
    /// Zero-based act index.
    pub act_index: u64,
    /// Floor within the run.
    pub floor: u64,
    /// Current (settled) map node identity.
    pub current_node: String,
    /// Room kind token for the current node.
    pub room_kind: String,
    /// Ordered visited node identities.
    pub visited_nodes: Vec<String>,
}

/// Player resources outside combat.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadPlayerResources {
    /// Current health.
    pub current_hp: u64,
    /// Maximum health.
    pub max_hp: u64,
    /// Gold.
    pub gold: u64,
    /// Ordered held key identities.
    pub keys: Vec<String>,
}

/// One ordered pile of card instance references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadCardPile {
    /// Pile identity.
    pub pile_id: String,
    /// Ordered card instance references.
    pub cards: Vec<String>,
}

/// Ordered deck and piles by card instance reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadDeckAndPiles {
    /// Ordered deck references.
    pub deck: Vec<String>,
    /// Ordered piles (empty outside combat).
    pub piles: Vec<PayloadCardPile>,
}

/// One temporary key/value on a card instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadTemporaryValue {
    /// Value key.
    pub key: String,
    /// Exact value.
    pub value: i64,
}

/// One card instance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadCardInstance {
    /// Instance identity referenced by deck and piles.
    pub instance_id: String,
    /// Definition identity.
    pub definition_id: String,
    /// Upgrade level.
    pub upgrade_level: u64,
    /// Ordered temporary values.
    pub temporary_values: Vec<PayloadTemporaryValue>,
}

/// Card instances, upgrades, and temporary values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadCardInstances {
    /// Instances in producer order; identities are distinct.
    pub cards: Vec<PayloadCardInstance>,
}

/// One relic with its counter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadRelic {
    /// Relic identity.
    pub relic_id: String,
    /// Exact counter value.
    pub counter: i64,
}

/// PayloadRelics in acquisition order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadRelics {
    /// Ordered relics.
    pub relics: Vec<PayloadRelic>,
}

/// One occupied potion slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadPotionSlot {
    /// Slot index.
    pub index: u64,
    /// Potion identity.
    pub potion_id: String,
}

/// Potion capacity and occupied slots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadPotions {
    /// Slot capacity.
    pub capacity: u64,
    /// Occupied slots in index order.
    pub slots: Vec<PayloadPotionSlot>,
}

/// Combat turn witnesses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadCombatTurn {
    /// One-based player turn.
    pub turn: u64,
    /// Remaining energy.
    pub energy: u64,
    /// Player block.
    pub player_block: u64,
}

/// One power on one owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadPower {
    /// Owner identity (player or enemy reference).
    pub owner: String,
    /// Power identity.
    pub power_id: String,
    /// Exact amount.
    pub amount: i64,
}

/// PayloadPowers in application order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadPowers {
    /// Ordered powers.
    pub powers: Vec<PayloadPower>,
}

/// One enemy intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadEnemyIntent {
    /// Intent identity.
    pub intent_id: String,
    /// Exact intent value.
    pub value: i64,
}

/// One enemy with its intents.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadEnemy {
    /// Enemy identity.
    pub enemy_id: String,
    /// Current health.
    pub current_hp: u64,
    /// Maximum health.
    pub max_hp: u64,
    /// Block.
    pub block: u64,
    /// Ordered intents.
    pub intents: Vec<PayloadEnemyIntent>,
}

/// Enemies and intents in encounter order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadEnemies {
    /// Ordered enemies.
    pub enemies: Vec<PayloadEnemy>,
}

/// Declared mode of one external input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadExternalInputMode {
    /// The input is controlled by the capture owner.
    Controlled,
    /// The input is absent at this boundary.
    Absent,
}

impl PayloadExternalInputMode {
    /// Returns the stable payload token.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Controlled => "controlled",
            Self::Absent => "absent",
        }
    }
}

/// One declared external input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadExternalInput {
    /// Input identity.
    pub input_id: String,
    /// Declared mode.
    pub mode: PayloadExternalInputMode,
}

/// Pending effects and external inputs at the boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PayloadPendingEffects {
    /// Outstanding effects; must be empty at a supported boundary.
    pub pending_effects: Vec<String>,
    /// Declared external inputs.
    pub external_inputs: Vec<PayloadExternalInput>,
}
