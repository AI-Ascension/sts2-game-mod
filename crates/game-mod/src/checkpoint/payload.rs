// SPDX-License-Identifier: MIT

//! Game-owned checkpoint payload contract v1 (`ascension.checkpoint_payload.v1`).
//!
//! The typed model covers only the three first-release boundaries. It lowers to
//! the restricted [`CanonicalValue`] model and parses back strictly, so the
//! closed schema in `schemas/checkpoint-payload-v1.schema.json` and this owner
//! code describe one contract. Holding a payload value proves nothing about a
//! host: no native phase is advertised as available, and every family is a
//! typed placeholder over the ADR 0037 inventory.

mod error;
mod families;
mod lower;
mod parse;
mod rules;

pub use error::CheckpointPayloadError;
pub use families::{
    PAYLOAD_MAX_ACTS, PAYLOAD_MAX_CARDS, PAYLOAD_MAX_ENEMIES, PAYLOAD_MAX_EXTERNAL_INPUTS,
    PAYLOAD_MAX_IDENTIFIER_BYTES, PAYLOAD_MAX_INTENTS, PAYLOAD_MAX_KEYS, PAYLOAD_MAX_MODIFIERS,
    PAYLOAD_MAX_PILES, PAYLOAD_MAX_POTION_SLOTS, PAYLOAD_MAX_POWERS, PAYLOAD_MAX_RELICS,
    PAYLOAD_MAX_RNG_STATE_WORDS, PAYLOAD_MAX_RNG_STREAMS, PAYLOAD_MAX_TEMPORARY_VALUES,
    PAYLOAD_MAX_VISITED_NODES, PayloadCampaignProgress, PayloadCardInstance, PayloadCardInstances,
    PayloadCardPile, PayloadCombatTurn, PayloadDeckAndPiles, PayloadEnemies, PayloadEnemy,
    PayloadEnemyIntent, PayloadExternalInput, PayloadExternalInputMode, PayloadPendingEffects,
    PayloadPlayerResources, PayloadPotionSlot, PayloadPotions, PayloadPower, PayloadPowers,
    PayloadRelic, PayloadRelics, PayloadRngStream, PayloadRunConfiguration, PayloadSeedAndRng,
    PayloadTemporaryValue,
};

use super::canonical::{CanonicalError, CanonicalValue, parse_canonical_text};
use super::capability::CheckpointBoundary;

/// Schema identifier carried by every v1 payload.
pub const CHECKPOINT_PAYLOAD_SCHEMA: &str = "ascension.checkpoint_payload.v1";
/// Repository-relative path of the closed JSON schema.
pub const CHECKPOINT_PAYLOAD_SCHEMA_SOURCE: &str = "schemas/checkpoint-payload-v1.schema.json";
/// Pinned SHA-256 (lowercase hex) of the closed JSON schema bytes.
pub const CHECKPOINT_PAYLOAD_SCHEMA_DIGEST: &str =
    "e332d8ca1a44d903147d3d3a3336f65686ff502522a45293228d6fc9c63f73dc";
/// Boundaries that have a v1 payload schema, in matrix order.
pub const CHECKPOINT_PAYLOAD_BOUNDARIES: [CheckpointBoundary; 3] = [
    CheckpointBoundary::SettledMapChoice,
    CheckpointBoundary::StablePlayerTurnCombat,
    CheckpointBoundary::LaterTurnCombat,
];

/// Explicit coverage disposition of one family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayloadFamilyCoverage<T> {
    /// The family closure was read completely at the boundary.
    Captured(T),
    /// The producer could not prove the closure; never accepted for a required family.
    Unknown,
    /// The family has no state at this boundary.
    NotApplicable,
}

impl<T> PayloadFamilyCoverage<T> {
    /// Returns the stable coverage token.
    #[must_use]
    pub const fn token(&self) -> &'static str {
        match self {
            Self::Captured(_) => "captured",
            Self::Unknown => "unknown",
            Self::NotApplicable => "not_applicable",
        }
    }

    /// Returns the captured value, if any.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Captured(value) => Some(value),
            Self::Unknown | Self::NotApplicable => None,
        }
    }
}

/// Every ADR 0037 coverage family with its explicit disposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointPayloadFamilies {
    /// Seed and RNG stream cursors.
    pub seed_and_rng: PayloadFamilyCoverage<PayloadSeedAndRng>,
    /// Character, ascension, mode, acts, modifiers.
    pub run_configuration: PayloadFamilyCoverage<PayloadRunConfiguration>,
    /// Map and room progress.
    pub campaign_progress: PayloadFamilyCoverage<PayloadCampaignProgress>,
    /// Player resources.
    pub player_resources: PayloadFamilyCoverage<PayloadPlayerResources>,
    /// Ordered deck and piles.
    pub deck_and_piles: PayloadFamilyCoverage<PayloadDeckAndPiles>,
    /// Card instances, upgrades, temporary values.
    pub card_instances: PayloadFamilyCoverage<PayloadCardInstances>,
    /// PayloadRelics.
    pub relics: PayloadFamilyCoverage<PayloadRelics>,
    /// PayloadPotions.
    pub potions: PayloadFamilyCoverage<PayloadPotions>,
    /// Combat turn witnesses.
    pub combat_turn: PayloadFamilyCoverage<PayloadCombatTurn>,
    /// PayloadPowers.
    pub powers: PayloadFamilyCoverage<PayloadPowers>,
    /// Enemies and intents.
    pub enemies_and_intents: PayloadFamilyCoverage<PayloadEnemies>,
    /// Pending effects and external inputs.
    pub pending_effects: PayloadFamilyCoverage<PayloadPendingEffects>,
}

impl CheckpointPayloadFamilies {
    /// Returns `(family, coverage token)` pairs in schema order.
    #[must_use]
    pub fn coverage_tokens(&self) -> [(&'static str, &'static str); 12] {
        [
            ("seed_and_rng", self.seed_and_rng.token()),
            ("run_configuration", self.run_configuration.token()),
            ("campaign_progress", self.campaign_progress.token()),
            ("player_resources", self.player_resources.token()),
            ("deck_and_piles", self.deck_and_piles.token()),
            ("card_instances", self.card_instances.token()),
            ("relics", self.relics.token()),
            ("potions", self.potions.token()),
            ("combat_turn", self.combat_turn.token()),
            ("powers", self.powers.token()),
            ("enemies_and_intents", self.enemies_and_intents.token()),
            ("pending_effects", self.pending_effects.token()),
        ]
    }
}

/// A validated v1 payload for one supported boundary.
#[derive(Clone, Eq, PartialEq)]
pub struct CheckpointPayload {
    boundary: CheckpointBoundary,
    families: CheckpointPayloadFamilies,
}

impl CheckpointPayload {
    /// Validates the boundary rules, bounds, and cross-family consistency.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointPayloadError`] when the boundary has no payload
    /// schema, a required family is `unknown` or missing, a family that does not
    /// apply is captured, a bound is exceeded, or references do not resolve.
    pub fn new(
        boundary: CheckpointBoundary,
        families: CheckpointPayloadFamilies,
    ) -> Result<Self, CheckpointPayloadError> {
        rules::validate(boundary, &families)?;
        Ok(Self { boundary, families })
    }

    /// Returns the payload schema identifier for a boundary, if one exists.
    #[must_use]
    pub const fn schema_for(boundary: CheckpointBoundary) -> Option<&'static str> {
        match boundary {
            CheckpointBoundary::SettledMapChoice
            | CheckpointBoundary::StablePlayerTurnCombat
            | CheckpointBoundary::LaterTurnCombat => Some(CHECKPOINT_PAYLOAD_SCHEMA),
            _ => None,
        }
    }

    /// Returns the admitted boundary.
    #[must_use]
    pub const fn boundary(&self) -> CheckpointBoundary {
        self.boundary
    }

    /// Returns the typed families.
    #[must_use]
    pub const fn families(&self) -> &CheckpointPayloadFamilies {
        &self.families
    }

    /// Lowers the payload to the restricted canonical value model.
    #[must_use]
    pub fn lower(&self) -> CanonicalValue {
        lower::lower(self)
    }

    /// Encodes the payload with the `asc-jcs-state-v1` profile.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalError`] when the canonical encoder rejects the output.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CanonicalError> {
        self.lower().to_canonical_bytes()
    }

    /// Strictly parses a canonical value into a validated payload.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointPayloadError`] for any structural, bound, coverage,
    /// or consistency violation.
    pub fn parse_canonical(value: &CanonicalValue) -> Result<Self, CheckpointPayloadError> {
        let (boundary, families) = parse::parse(value)?;
        Self::new(boundary, families)
    }

    /// Strictly parses restricted canonical JSON text into a validated payload.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointPayloadError::Canonical`] when the text is not
    /// restricted canonical JSON, or any [`CheckpointPayloadError`] from parsing.
    pub fn parse_canonical_text(text: &str) -> Result<Self, CheckpointPayloadError> {
        Self::parse_canonical(&parse_canonical_text(text)?)
    }
}

impl From<&CheckpointPayload> for CanonicalValue {
    fn from(payload: &CheckpointPayload) -> Self {
        payload.lower()
    }
}

/// Redacted: prints the boundary and coverage tokens, never seed, RNG, or values.
impl std::fmt::Debug for CheckpointPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = formatter.debug_struct("CheckpointPayload");
        output.field("boundary", &self.boundary.code());
        for (family, token) in self.families.coverage_tokens() {
            output.field(family, &token);
        }
        output.finish()
    }
}
