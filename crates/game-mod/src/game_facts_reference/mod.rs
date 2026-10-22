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
//! The generic inventory and optional [`CoreFactsInventory`] admit caller-supplied metadata;
//! local validation proves consistency and Core signature agreement, not source authenticity.
//! The separate [`OwnedLiveGameFactsProducer`] maps the production owner's retained live copies
//! to the accepted facts profile. It reads no host objects itself and evaluates no rule. Its
//! bridge performs source authorization and freshness checks; constructing equivalent public
//! source DTOs is not proof of provenance. All emitted observations remain source-derived, with
//! native parity and exact-host acceptance separately unverified.

mod core_catalog;
mod error;
mod identity;
mod inventory;
mod live;
mod live_mapping;
mod live_source;
mod model;
mod protocol_v1;
mod validation;

pub use core_catalog::CoreFactsInventory;
pub use error::GameFactsError;
pub use identity::is_opaque_facts_identity;
pub use inventory::FactsInventory;
pub use live::OwnedLiveGameFactsProducer;
pub use live_source::{
    LiveFactsField, LiveFactsFieldStatus, LiveFactsRefusal, OwnedLiveFactsCard,
    OwnedLiveFactsSnapshot,
};
pub use model::{
    FactsBuildBinding, FactsEvidenceStatus, FactsInputAvailability, FactsInputSource,
    FactsRepresentation, FactsRuleEntry, FactsRuleInput, FactsSourceKind,
    FactsUnsupportedCombination, GAME_FACTS_MAX_COMBINATION_MEMBERS, GAME_FACTS_MAX_IDENTITY_BYTES,
    GAME_FACTS_MAX_IDENTITY_SEGMENTS, GAME_FACTS_MAX_INPUTS_PER_RULE, GAME_FACTS_MAX_LABEL_BYTES,
    GAME_FACTS_MAX_RULES, GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION,
};
pub use protocol_v1::GameFactsReferenceV1Adapter;
