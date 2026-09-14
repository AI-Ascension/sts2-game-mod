// SPDX-License-Identifier: MIT

use super::super::model::{EnemyField, EnemyNumericValue, EnemyOrigin, EnemyText};

/// One named stat with an explicit unit and fixed/formula/unavailable value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyStat {
    /// Stable owner-defined stat identity.
    pub stat_id: String,
    /// Optional unit for the value, such as `hp` or `count`.
    pub unit: Option<String>,
    /// Observed, dynamic, or unavailable value.
    pub value: EnemyNumericValue,
}

/// One mode/difficulty-scaled stat profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyStatProfile {
    /// Stable profile identity scoped by the enemy.
    pub profile_id: String,
    /// Source-defined mode identity, if present.
    pub mode: Option<String>,
    /// Source-defined difficulty identity, if present.
    pub difficulty: Option<String>,
    /// Profile-specific stats.
    pub stats: Vec<EnemyStat>,
}

/// Base stats and mode/difficulty-scaled profiles.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyStats {
    /// Base/normal definition values.
    pub base: EnemyField<Vec<EnemyStat>>,
    /// Mode/difficulty variants, including explicit empty/unknown states.
    pub scaled: EnemyField<Vec<EnemyStatProfile>>,
}

/// Stable tag reference attached to an enemy definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyTag {
    /// Owner-defined tag identity.
    pub tag_id: String,
    /// Localized/source-defined tag label.
    pub label: EnemyText,
}

/// Typed semantic reference to a status, encounter, effect, condition, or rule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemySemanticReference {
    /// Reference family.
    pub kind: EnemySemanticReferenceKind,
    /// Stable owner-defined identity.
    pub id: String,
    /// Localized/source-defined label.
    pub label: EnemyText,
}

/// Supported semantic reference families.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemySemanticReferenceKind {
    /// A power/status definition in the content manifest.
    Status,
    /// An encounter definition in the content manifest.
    Encounter,
    /// An effect definition or rule link.
    Effect,
    /// A transition or timing rule.
    Rule,
    /// A source-owned condition.
    Condition,
    /// An owner-defined content family not otherwise named here.
    Content { entity_kind: String },
    /// A family not classified by the source.
    Unknown,
}

/// One encounter where an enemy can spawn.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyEncounterReference {
    /// Encounter definition identity.
    pub encounter_id: String,
    /// Optional localized/source-defined encounter label.
    pub label: EnemyText,
    /// Optional owner-defined encounter role.
    pub role: Option<String>,
}

/// An origin/package variant retained separately from the base definition origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyOriginVariant {
    /// Stable variant identity scoped by the enemy.
    pub variant_id: String,
    /// Localized/source-defined variant label.
    pub label: EnemyText,
    /// Variant provenance.
    pub origin: EnemyOrigin,
    /// Variant stats when the source can provide them.
    pub stats: EnemyField<EnemyStats>,
    /// Variant move IDs when the source overrides the base move set.
    pub move_ids: EnemyField<Vec<String>>,
}
