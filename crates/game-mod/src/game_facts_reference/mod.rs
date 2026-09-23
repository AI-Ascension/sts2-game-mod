// SPDX-License-Identifier: MIT

//! Proposed owner inventory for the game-facts handoff: which rules this boundary will state, the
//! inputs and units each one copies, the exact build, mode and content-manifest binding they were
//! taken from, and how strongly each claim is supported.
//!
//! This is the game-mod half of the authoritative-rules handoff that `sts2-game-core#13` asks for.
//! Core keeps the pure rule models; this module states only what the owner declares, and refuses
//! rather than reconstructs, because a handoff that quietly fills a gap is worse than one that
//! admits it:
//!
//! - The inventory is closed. A rule is either listed with its inputs, units and evidence status or
//!   it is absent; an absent rule is never read as an exact result.
//! - Every copied input carries a unit, so a fact is never hidden behind a producer's prose tag or
//!   a downstream parser's guess.
//! - Every entry records how it is known. Only a `Confirmed` entry supports an exact claim; a
//!   `SourceDerived`, `Proposed`, `Inferred` or `Unverified` entry does not, and a rule that takes
//!   part in a recorded unsupported combination never reads as exact even when its own entry is
//!   confirmed.
//! - Identities are opaque: a filesystem path, a control byte or a traversal segment is refused.
//!
//! Source-only by construction. The inventory is a *proposal* to the owner; it claims no native
//! parity, no host comparison and no transport. Extracting live facts through game-owned access and
//! mapping them into core inputs is the follow-up adapter, deliberately not built here.

mod error;
mod identity;
mod inventory;
mod model;
mod validation;

pub use error::GameFactsError;
pub use identity::is_opaque_facts_identity;
pub use inventory::FactsInventory;
pub use model::{
    FactsBuildBinding, FactsEvidenceStatus, FactsInputAvailability, FactsRepresentation,
    FactsRuleEntry, FactsRuleInput, FactsUnsupportedCombination,
    GAME_FACTS_MAX_COMBINATION_MEMBERS, GAME_FACTS_MAX_IDENTITY_BYTES,
    GAME_FACTS_MAX_IDENTITY_SEGMENTS, GAME_FACTS_MAX_INPUTS_PER_RULE, GAME_FACTS_MAX_LABEL_BYTES,
    GAME_FACTS_MAX_RULES, GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION,
};
