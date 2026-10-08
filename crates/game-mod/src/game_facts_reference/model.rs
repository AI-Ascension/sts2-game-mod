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

/// Caller-supplied label for how strongly one inventory entry's claim is supported.
///
/// These are the repository's claim labels carried as data, so a consumer never has to read a
/// producer's prose to learn how much a rule is trusted. The model does not authenticate a label.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FactsEvidenceStatus {
    /// Caller claims confirmation against an authorized exact-host comparison.
    Confirmed,
    /// Caller claims derivation from the owner's source and content, without a host comparison.
    /// This label is not authenticated by the inventory model or mapper.
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
    /// Only the caller's `Confirmed` label passes this classification. The model does not verify
    /// that label, and SourceDerived or Proposed labels are not native comparisons.
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
    ///
    /// Disclosed is not the same as exact: a rule-level exactness query carries no combination
    /// context, so a conditional input never supports an exact claim on its own.
    Conditional,
    /// The owner does not know whether this input applies here.
    ///
    /// An unknown input is the weakest case: because the owner cannot say whether it applies, the
    /// result cannot be stated exactly.
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

    /// Returns whether an input with this availability is fully stated for an exact claim.
    ///
    /// Only a `Required` input is. A conditional input leaves the result open for combinations
    /// this inventory does not resolve, and an unknown input leaves it open entirely, so neither
    /// may back an exact-looking result.
    #[must_use]
    pub const fn supports_exact_claim(self) -> bool {
        matches!(self, Self::Required)
    }
}

/// Caller-supplied claimed category for one copied input's source reference.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FactsSourceKind {
    /// The caller identifies the source as GameMod-owned data.
    GameMod,
    /// The caller identifies the source as a content-manifest record.
    ContentManifest,
}

/// Opaque per-input provenance supplied by the caller.
///
/// The inventory validates its shape, but this value does not authenticate its origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsInputSource {
    /// Owner source category for the reference.
    pub kind: FactsSourceKind,
    /// Opaque token supplied by the caller; local validation cannot verify its origin.
    pub reference: String,
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
    /// Optional caller-supplied source reference; presence is not proof of authenticity.
    pub source: Option<FactsInputSource>,
}

/// Caller-supplied exact-build and mode claims, bound to a content-manifest value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactsBuildBinding {
    /// Caller-supplied opaque build identity claim.
    pub build_id: String,
    /// Caller-supplied opaque mode identity claim inside that build.
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

/// One caller-declared supported rule: its opaque id, evidence label, and copied inputs.
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

    /// Returns whether every copied input is fully stated.
    ///
    /// A rule copies only inputs it declared; when any of them is conditional or unknown, the
    /// rule cannot be read as an exact claim, because a consumer would have to guess the missing
    /// combination context.
    #[must_use]
    pub fn inputs_support_exact_claim(&self) -> bool {
        self.inputs
            .iter()
            .all(|input| input.availability.supports_exact_claim())
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
