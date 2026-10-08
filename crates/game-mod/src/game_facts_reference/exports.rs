// SPDX-License-Identifier: MIT

pub use game_facts_reference::{
    FactsBuildBinding, FactsEvidenceStatus, FactsInputAvailability, FactsInputSource,
    FactsInventory, FactsRepresentation, FactsRuleEntry, FactsRuleInput, FactsSourceKind,
    FactsUnsupportedCombination, GAME_FACTS_MAX_COMBINATION_MEMBERS, GAME_FACTS_MAX_IDENTITY_BYTES,
    GAME_FACTS_MAX_IDENTITY_SEGMENTS, GAME_FACTS_MAX_INPUTS_PER_RULE, GAME_FACTS_MAX_LABEL_BYTES,
    GAME_FACTS_MAX_RULES, GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS,
    GAME_FACTS_REFERENCE_PRODUCER_VERSION, GameFactsError, GameFactsReferenceV1Adapter,
    is_opaque_facts_identity,
};
