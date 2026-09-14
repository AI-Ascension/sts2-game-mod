// SPDX-License-Identifier: MIT

use crate::ContentCursorBinding;

/// Manifest family handled by this owner-local producer.
pub const ENEMY_ENTITY_KIND: &str = "enemy";
/// Source-only producer identity; this is not a wire or native ABI version.
pub const ENEMY_PRODUCER_VERSION: &str = "game-enemy-reference-producer-v1";
/// Maximum bytes accepted for one owner-defined identity.
pub const ENEMY_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one localized or owner-defined text value.
pub const ENEMY_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one enemy definition.
pub const ENEMY_MAX_DEFINITION_BYTES: usize = 128 * 1024;
/// Maximum enemy definitions in one source snapshot.
pub const ENEMY_MAX_DEFINITIONS: usize = 4_096;
/// Maximum entries returned by one bounded definition or move page.
pub const ENEMY_MAX_PAGE_ITEMS: usize = 64;
/// Maximum tags on one enemy.
pub const ENEMY_MAX_TAGS: usize = 64;
/// Maximum stats in one base or scaled profile.
pub const ENEMY_MAX_STATS: usize = 64;
/// Maximum difficulty/mode profiles on one enemy.
pub const ENEMY_MAX_STAT_PROFILES: usize = 64;
/// Maximum origin variants on one enemy.
pub const ENEMY_MAX_ORIGIN_VARIANTS: usize = 32;
/// Maximum moves on one enemy.
pub const ENEMY_MAX_MOVES: usize = 128;
/// Maximum effects in one move.
pub const ENEMY_MAX_EFFECTS: usize = 32;
/// Maximum phases on one enemy.
pub const ENEMY_MAX_PHASES: usize = 32;
/// Maximum behavior transitions on one enemy.
pub const ENEMY_MAX_TRANSITIONS: usize = 64;
/// Maximum semantic references on one enemy or move.
pub const ENEMY_MAX_REFERENCES: usize = 128;
/// Maximum parameters on one effect, condition, or move.
pub const ENEMY_MAX_PARAMETERS: usize = 32;
/// Maximum unresolved formula inputs.
pub const ENEMY_MAX_FORMULA_INPUTS: usize = 32;

/// Static catalog identity: content manifest, locale, and producer compatibility.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyCatalogBinding {
    /// Existing content-manifest invalidation witness.
    pub manifest: ContentCursorBinding,
    /// Locale used for every localized enemy value.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Exact static enemy definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyDefinitionReference {
    /// Catalog witness that owns this enemy identity.
    pub catalog: EnemyCatalogBinding,
    /// Namespaced content definition identity.
    pub enemy_id: String,
}

/// Exact static move reference.  Move IDs are scoped by their enemy definition.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyMoveReference {
    /// Catalog witness that owns this move identity.
    pub catalog: EnemyCatalogBinding,
    /// Owning enemy definition identity.
    pub enemy_id: String,
    /// Stable move identity.
    pub move_id: String,
}

/// Explicit reason that a source value was not observed.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyUnavailableReason {
    /// The source supports the field but did not observe it.
    NotObserved,
    /// The supported host/build has no extractor for the field.
    Unsupported,
    /// The caller's source scope denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
    /// The field has no meaning for the selected definition.
    NotApplicable,
}

/// A source field that distinguishes an observed empty value from an unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyField<T> {
    /// The source observed the field; `T` may be empty without becoming unavailable.
    Available(T),
    /// The source could not safely provide the field for the stated reason.
    Unavailable(EnemyUnavailableReason),
}

impl<T> EnemyField<T> {
    /// Returns the explicit field availability status.
    #[must_use]
    pub const fn status(&self) -> EnemyFieldStatus {
        match self {
            Self::Available(_) => EnemyFieldStatus::Available,
            Self::Unavailable(reason) => reason.status(),
        }
    }

    /// Returns the observed value, including an observed empty collection.
    #[must_use]
    pub const fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) => Some(value),
            Self::Unavailable(_) => None,
        }
    }

    /// Creates an explicit unavailable field value.
    #[must_use]
    pub const fn unavailable(reason: EnemyUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// Coarse availability status for one source-owned enemy field.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyFieldStatus {
    /// The source observed the field, including a known empty value.
    Available,
    /// The field has no meaning for the selected definition.
    NotApplicable,
    /// The field is supported but was not observed.
    NotObserved,
    /// No extractor is supported for the field.
    Unsupported,
    /// The source denied the field.
    Denied,
    /// Extraction failed without a safe value.
    Failed,
    /// The source could not classify the field.
    Unknown,
}

impl EnemyUnavailableReason {
    /// Returns the corresponding coarse field status.
    #[must_use]
    pub const fn status(self) -> EnemyFieldStatus {
        match self {
            Self::NotObserved => EnemyFieldStatus::NotObserved,
            Self::Unsupported => EnemyFieldStatus::Unsupported,
            Self::Denied => EnemyFieldStatus::Denied,
            Self::Failed => EnemyFieldStatus::Failed,
            Self::Unknown => EnemyFieldStatus::Unknown,
            Self::NotApplicable => EnemyFieldStatus::NotApplicable,
        }
    }
}

/// Localized text or an explicit non-value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyText {
    /// Text copied for the snapshot locale.
    Available(String),
    /// Text was unavailable; no placeholder is inferred.
    Unavailable(EnemyUnavailableReason),
}

impl EnemyText {
    /// Creates a bounded available localized text value.
    pub fn available(value: impl Into<String>) -> Result<Self, super::EnemyCatalogError> {
        let value = value.into();
        if value.is_empty() {
            return Err(super::EnemyCatalogError::InvalidInput("text"));
        }
        validate_text(&value, "text")?;
        Ok(Self::Available(value))
    }

    /// Creates an explicit unavailable text value.
    #[must_use]
    pub const fn unavailable(reason: EnemyUnavailableReason) -> Self {
        Self::Unavailable(reason)
    }
}

/// A source-owned dynamic formula without an executable expression.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyFormula {
    /// Stable owner rule identity.
    pub rule_reference: String,
    /// Inputs unavailable to this source-only projection.
    pub unresolved_inputs: Vec<String>,
}

impl EnemyFormula {
    /// Creates a bounded formula reference with explicit unresolved inputs.
    pub fn new(
        rule_reference: impl Into<String>,
        unresolved_inputs: impl IntoIterator<Item = String>,
    ) -> Result<Self, super::EnemyCatalogError> {
        let formula = Self {
            rule_reference: rule_reference.into(),
            unresolved_inputs: unresolved_inputs.into_iter().collect(),
        };
        validate_identity(&formula.rule_reference, "formula_rule")?;
        if formula.unresolved_inputs.len() > ENEMY_MAX_FORMULA_INPUTS {
            return Err(super::EnemyCatalogError::InvalidInput("formula_inputs"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for input in &formula.unresolved_inputs {
            validate_identity(input, "formula_input")?;
            if !seen.insert(input.as_str()) {
                return Err(super::EnemyCatalogError::InvalidInput(
                    "duplicate_formula_input",
                ));
            }
        }
        Ok(formula)
    }
}

/// A fixed, formula-backed, or unavailable numeric value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyNumericValue {
    /// Value observed directly in the owner definition.
    Fixed(i64),
    /// Dynamic value linked to a non-executable owner rule.
    Formula(EnemyFormula),
    /// Value was not safely available.
    Unavailable(EnemyUnavailableReason),
}

/// Owner-defined source origin/provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyOrigin {
    /// Source-defined origin kind, such as a package or registry.
    pub kind: String,
    /// Optional active package identity.
    pub package_id: Option<String>,
    /// Optional active package version.
    pub package_version: Option<String>,
}

/// Coarse enemy role copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyKind {
    /// Ordinary encounter enemy.
    Normal,
    /// Elite encounter enemy.
    Elite,
    /// Boss or act-ending enemy.
    Boss,
    /// Temporary or spawned subordinate enemy.
    Minion,
    /// Owner-defined role.
    Custom(String),
}

/// Owner-defined enemy visibility.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyVisibility {
    /// Visible on the supported public reference surface.
    Visible,
    /// Visible only in an explicitly owner-authorized scope.
    OwnerOnly,
    /// Source knows a value exists but must not reveal it.
    Hidden,
    /// Source could not classify visibility.
    Unknown,
}

/// Scope requested by a static reference query.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyVisibilityScope {
    /// Publicly visible and unlocked definitions.
    Public,
    /// Public definitions plus locked references.
    Reference,
    /// Explicit owner-authorized scope.
    Owner,
}

/// Evidence label for a behavior fact.  This keeps unverified rules distinct from observations.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyEvidence {
    /// The owner source exposes the rule as an authoritative definition.
    Authoritative,
    /// The rule was copied from a source-owned definition without runtime verification.
    SourceDerived,
    /// The rule was independently authored and is not a host claim.
    IndependentlyAuthored,
    /// The value was observed on a visible surface.
    Observed,
    /// Evidence is insufficient to classify the value as authoritative.
    Unverified,
}

/// A static targeting domain used by a move or effect.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyTargetDomain {
    /// The acting enemy itself.
    SelfOnly,
    /// One player target.
    AnyPlayer,
    /// All player targets.
    AllPlayers,
    /// One enemy target.
    AnyEnemy,
    /// All enemy targets.
    AllEnemies,
    /// One source-defined entity.
    SpecificEntity,
    /// The effect has no target.
    None,
    /// The source could not classify the target domain.
    Unknown,
}

/// Targeting domain plus a typed target count where one is available.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyTargeting {
    /// Public targeting domain.
    pub domain: EnemyTargetDomain,
    /// Number of affected targets, or an explicit unavailable/formula value.
    pub count: EnemyNumericValue,
}

/// A probability or weight retained without evaluating hidden RNG.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyProbability {
    /// Exact rational probability/weight and its evidence label.
    Exact {
        /// Numerator.
        numerator: u32,
        /// Positive denominator.
        denominator: u32,
        /// Provenance/evidence for the value.
        evidence: EnemyEvidence,
    },
    /// Non-executable owner formula with explicit evidence.
    Formula {
        /// Formula and unresolved inputs.
        formula: EnemyFormula,
        /// Provenance/evidence for the formula.
        evidence: EnemyEvidence,
    },
    /// No safe probability was available.
    Unavailable(EnemyUnavailableReason),
}

/// A typed reset boundary for repetition/cooldown restrictions.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyResetBoundary {
    /// Never resets during the retained definition lifetime.
    Never,
    /// Reset after an enemy turn.
    Turn,
    /// Reset after a combat round.
    Round,
    /// Reset when the room/combat ends.
    Combat,
    /// Reset at a phase boundary.
    Phase,
    /// Source-defined reset boundary is unknown.
    Unknown,
}

/// Validates an owner-defined identity token.
pub(super) fn validate_identity(
    value: &str,
    field: &'static str,
) -> Result<(), super::EnemyCatalogError> {
    if value.is_empty()
        || value.len() > ENEMY_MAX_IDENTITY_BYTES
        || value.chars().any(char::is_control)
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric()
                && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-' | b'#')
        })
    {
        return Err(super::EnemyCatalogError::InvalidInput(field));
    }
    Ok(())
}

/// Validates a localized or owner-defined text value.
pub(super) fn validate_text(
    value: &str,
    field: &'static str,
) -> Result<(), super::EnemyCatalogError> {
    if value.is_empty() || value.len() > ENEMY_MAX_TEXT_BYTES || value.chars().any(char::is_control)
    {
        return Err(super::EnemyCatalogError::InvalidInput(field));
    }
    Ok(())
}
