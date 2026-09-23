// SPDX-License-Identifier: MIT

//! Bounds and the owner-declared vocabulary: evidence status, input availability, and the rules,
//! inputs, build binding and structured representation one inventory carries.

use crate::ContentCursorBinding;

/// Source-only producer identity for the facts-handoff inventory; this is not a wire or native ABI
/// version.
pub const GAME_FACTS_REFERENCE_PRODUCER_VERSION: &str = "game-facts-reference-producer-v1";
/// Maximum bytes accepted for one opaque identity.
pub const GAME_FACTS_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum dot-separated segments accepted in one opaque identity.
pub const GAME_FACTS_MAX_IDENTITY_SEGMENTS: usize = 8;
/// Maximum bytes accepted for one owner-defined label or reason.
pub const GAME_FACTS_MAX_LABEL_BYTES: usize = 1024;
/// Maximum rules one owner inventory may declare.
pub const GAME_FACTS_MAX_RULES: usize = 1024;
/// Maximum inputs one rule may copy.
pub const GAME_FACTS_MAX_INPUTS_PER_RULE: usize = 64;
/// Maximum unsupported combinations one inventory may record.
pub const GAME_FACTS_MAX_UNSUPPORTED_COMBINATIONS: usize = 256;
/// Maximum rules one unsupported combination may name.
pub const GAME_FACTS_MAX_COMBINATION_MEMBERS: usize = 16;

/// How strongly one inventory entry's claim is supported.
///
/// These are the repository's claim labels carried as data, so a consumer never has to read a
/// producer's prose to learn how much a rule is trusted.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FactsEvidenceStatus {
    /// Confirmed against an authorized exact-host comparison.
    Confirmed,
    /// Derived from the owner's own source and content, without a host comparison.
    SourceDerived,
    /// Proposed to the owner; not yet agreed.
    Proposed,
    /// Inferred from adjacent evidence; not directly observed.
    Inferred,
    /// Recorded without support.
    Unverified,
}

impl FactsEvidenceStatus {
    /// Every status, in a stable order.
    pub const ALL: [Self; 5] = [
        Self::Confirmed,
        Self::SourceDerived,
        Self::Proposed,
        Self::Inferred,
        Self::Unverified,
    ];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Confirmed => "confirmed",
            Self::SourceDerived => "source_derived",
            Self::Proposed => "proposed",
            Self::Inferred => "inferred",
            Self::Unverified => "unverified",
        }
    }

    /// Returns whether this status alone supports an exact rule claim.
    ///
    /// Only a host-confirmed entry does. A source-derived or proposed entry is real evidence, but it
    /// is not a native comparison, so a consumer must not present it as exact.
    #[must_use]
    pub const fn supports_exact_claim(self) -> bool {
        matches!(self, Self::Confirmed)
    }
}

/// Whether one copied input is required, conditional or explicitly unknown for its rule.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FactsInputAvailability {
    /// The rule needs this input to be stated at all.
    Required,
    /// The input changes the result only for some combinations, which stay disclosed.
    Conditional,
    /// The owner does not know whether this input applies here.
    Unknown,
}

impl FactsInputAvailability {
    /// Every availability, in a stable order.
    pub const ALL: [Self; 3] = [Self::Required, Self::Conditional, Self::Unknown];

    /// The stable lowercase name used in owner-defined text and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Required => "required",
            Self::Conditional => "conditional",
            Self::Unknown => "unknown",
        }
    }
}

/// One input a rule copies: an opaque name, the unit it is stated in, and its availability.
///
/// The unit is carried with the name because a quantity without a unit is a prose tag, not a fact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsRuleInput {
    /// Opaque input name, unique inside its rule.
    pub name: String,
    /// Opaque unit the input is stated in, for example `health`, `flat` or `percent`.
    pub unit: String,
    /// Whether this input is required, conditional or explicitly unknown.
    pub availability: FactsInputAvailability,
}

/// The exact build and mode one inventory was taken from, bound to the content manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsBuildBinding {
    /// Opaque build identity the inventory was taken from.
    pub build_id: String,
    /// Opaque mode identity inside that build.
    pub mode_id: String,
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
}

/// The negotiated structured representation rule facts are carried in.
///
/// The representation is named and versioned rather than left in prose, so a consumer binds to it
/// instead of parsing a producer's label.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsRepresentation {
    /// Opaque representation version.
    pub version: String,
    /// Opaque encoding name, for example `structured-facts`.
    pub encoding: String,
}

/// One supported rule: its opaque id, how it is known, and the inputs it copies.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsRuleEntry {
    /// Opaque rule id, unique inside its inventory.
    pub rule_id: String,
    /// How strongly this entry's claim is supported.
    pub evidence: FactsEvidenceStatus,
    /// The inputs this rule copies; never empty, so facts are never hidden in prose.
    pub inputs: Vec<FactsRuleInput>,
}

impl FactsRuleEntry {
    /// Returns the copied input with this name, when the rule copies it.
    #[must_use]
    pub fn input(&self, name: &str) -> Option<&FactsRuleInput> {
        self.inputs.iter().find(|input| input.name == name)
    }
}

/// One combination the inventory declares it cannot represent exactly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsUnsupportedCombination {
    /// Two or more opaque rule ids whose interaction is not represented exactly.
    pub rule_ids: Vec<String>,
    /// Bounded owner-defined reason, never a substitute for the structured record.
    pub reason: String,
}
