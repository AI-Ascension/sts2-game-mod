// SPDX-License-Identifier: MIT

use super::field::EnemyIntentField;
use super::targets::{
    EnemyIntentAmount, EnemyIntentDamage, EnemyIntentTargetInfo, EnemyIntentUnit,
};
use crate::ContentCursorBinding;

/// Owner-local producer identity for structured enemy intents.
pub const ENEMY_INTENT_PRODUCER_VERSION: &str = "game-enemy-intents-producer-v1";
/// Maximum bytes accepted for one identity token.
pub const ENEMY_INTENT_MAX_IDENTITY_BYTES: usize = 256;
/// Maximum bytes accepted for one visible description or parameter text.
pub const ENEMY_INTENT_MAX_TEXT_BYTES: usize = 16 * 1024;
/// Maximum aggregate bytes retained for one live enemy detail.
pub const ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES: usize = 64 * 1024;
/// Maximum aggregate bytes retained by one coherent enemy snapshot.
pub const ENEMY_INTENT_MAX_SNAPSHOT_BYTES: usize = 2 * 1024 * 1024;
/// Maximum live enemies retained in one coherent snapshot.
pub const ENEMY_INTENT_MAX_ENEMIES: usize = 256;
/// Maximum ordered components on one intent.
pub const ENEMY_INTENT_MAX_COMPONENTS: usize = 32;
/// Maximum visible target references on one target set.
pub const ENEMY_INTENT_MAX_TARGETS: usize = 128;
/// Maximum status/entity references attached to one component.
pub const ENEMY_INTENT_MAX_EFFECT_REFERENCES: usize = 64;
/// Maximum typed parameters attached to one component.
pub const ENEMY_INTENT_MAX_PARAMETERS: usize = 64;
/// Maximum status entries retained for one enemy.
pub const ENEMY_INTENT_MAX_STATUSES: usize = 128;
/// Maximum bounded source code used to retain an unsupported category.
pub const ENEMY_INTENT_MAX_KIND_BYTES: usize = 256;

/// Static content witness and locale used by a live intent projection.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyIntentCatalogBinding {
    /// Immutable content-manifest cursor witness.
    pub content_manifest: ContentCursorBinding,
    /// Locale used for localized enemy/move text.
    pub locale: String,
    /// Exact owner-local producer identity.
    pub producer_version: String,
}

/// Full identity fence for one coherent enemy-intent snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyIntentLiveBinding {
    /// Static content and locale witness.
    pub catalog: EnemyIntentCatalogBinding,
    /// Selected game instance identity.
    pub game_instance_id: String,
    /// Selected run identity.
    pub run_id: String,
    /// Stable combat identity.
    pub combat_id: String,
    /// Coherent source snapshot identity.
    pub snapshot_id: String,
    /// Monotonic owner-local observation epoch.
    pub epoch: u64,
}

/// Typed parameter value retained without evaluating hidden formulas.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EnemyIntentParameterValue {
    /// Signed integer parameter.
    Integer(i64),
    /// Boolean parameter.
    Boolean(bool),
    /// Bounded textual parameter.
    Text(String),
    /// Fixed-point parameter with explicit decimal scale.
    Decimal { value: i64, scale: u8 },
    /// Source-defined value whose type is not supported by this projection.
    Unknown,
}

/// One visible effect parameter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentParameter {
    /// Stable source parameter identity.
    pub id: String,
    /// Optional localized parameter label.
    pub label: EnemyIntentField<String>,
    /// Typed value or explicit unknown.
    pub value: EnemyIntentParameterValue,
    /// Optional semantic unit.
    pub unit: EnemyIntentField<EnemyIntentUnit>,
}

/// A status/entity reference affected by one visible component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentEffectReference {
    /// Stable status, entity, or effect identity.
    pub id: String,
    /// Source family of the reference.
    pub kind: EnemyIntentEffectReferenceKind,
    /// Optional signed amount when the source exposes one.
    pub amount: EnemyIntentField<i32>,
}

/// Family of a component's referenced effect.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyIntentEffectReferenceKind {
    /// A power or status definition.
    Status,
    /// A live entity affected by the component.
    Entity,
    /// An owner-defined effect/rule identity.
    Effect,
    /// The source could not classify the reference.
    Unknown,
}

/// Intent category retained as an explicit owner-local vocabulary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyIntentComponentKind {
    /// A damage-dealing component.
    Attack,
    /// A block/defensive component.
    Defend,
    /// A healing component.
    Heal,
    /// A beneficial power/status component.
    Buff,
    /// A harmful power/status component.
    Debuff,
    /// A component that creates a new entity.
    Summon,
    /// A component that removes or flees the enemy.
    Escape,
    /// A phase or form transition.
    PhaseChange,
    /// A source-defined category retained without guessed semantics.
    Custom(String),
    /// The source cannot classify this component.
    Unknown,
    /// A new category is known but unsupported by this producer.
    Unsupported(String),
}

/// Linkage between a live intent and its source move/definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentLinkage {
    /// Stable identity for this visible intent instance.
    pub intent_id: String,
    /// Static move identity when visible.
    pub move_id: EnemyIntentField<String>,
    /// Static enemy-move definition identity when visible.
    pub definition_id: EnemyIntentField<String>,
    /// Localized move label when visible.
    pub label: EnemyIntentField<String>,
}

/// One ordered structured component of a visible enemy intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentComponent {
    /// Stable component identity, distinct from intent and move IDs.
    pub component_id: String,
    /// Component kind.
    pub kind: EnemyIntentComponentKind,
    /// Optional localized component description.
    pub description: EnemyIntentField<String>,
    /// Attack damage semantics.
    pub damage: EnemyIntentField<EnemyIntentDamage>,
    /// Visible block/heal/summon amount.
    pub amount: EnemyIntentField<EnemyIntentAmount>,
    /// Status, entity, or effect references affected by this component.
    pub effects: EnemyIntentField<Vec<EnemyIntentEffectReference>>,
    /// Typed source parameters.
    pub parameters: EnemyIntentField<Vec<EnemyIntentParameter>>,
    /// Component target domain and public target identities.
    pub targets: EnemyIntentField<EnemyIntentTargetInfo>,
}

/// One source-owned structured enemy intent before snapshot binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentInput {
    /// Intent linkage and stable intent identity.
    pub linkage: EnemyIntentLinkage,
    /// Ordered visible effects; order is source-significant.
    pub components: Vec<EnemyIntentComponent>,
    /// Aggregate target information for the complete intent.
    pub targets: EnemyIntentField<EnemyIntentTargetInfo>,
}

/// A visible status carried with an enemy read.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentStatus {
    /// Stable live status instance identity when supplied.
    pub instance_id: EnemyIntentField<String>,
    /// Static status definition identity when supplied.
    pub definition_id: String,
    /// Optional visible status label.
    pub label: EnemyIntentField<String>,
    /// Optional visible stack/amount.
    pub amount: EnemyIntentField<i32>,
}

/// Source-owned enemy detail before snapshot validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentEnemyInput {
    /// Stable live enemy instance identity.
    pub enemy_instance_id: String,
    /// Static enemy definition identity.
    pub enemy_definition_id: String,
    /// Localized enemy name.
    pub name: EnemyIntentField<String>,
    /// Current hit points.
    pub hp: EnemyIntentField<u32>,
    /// Maximum hit points.
    pub max_hp: EnemyIntentField<u32>,
    /// Current block.
    pub block: EnemyIntentField<u32>,
    /// Current visible powers/statuses.
    pub statuses: EnemyIntentField<Vec<EnemyIntentStatus>>,
    /// Current structured intent and its target visibility.
    pub intent: EnemyIntentField<EnemyIntentInput>,
}

/// Coherent source snapshot before validation and adoption.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentSnapshotInput {
    /// Full identity fence for this snapshot.
    pub binding: EnemyIntentLiveBinding,
    /// Enemy values copied from one coherent source observation.
    pub enemies: Vec<EnemyIntentEnemyInput>,
}

/// Stable live enemy reference tied to one exact coherent snapshot.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyIntentEnemyReference {
    /// Snapshot identity fence.
    pub binding: EnemyIntentLiveBinding,
    /// Live enemy instance identity.
    pub enemy_instance_id: String,
    /// Static enemy definition identity.
    pub enemy_definition_id: String,
}

/// Stable reference to one structured intent component.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EnemyIntentComponentReference {
    /// Snapshot identity fence.
    pub binding: EnemyIntentLiveBinding,
    /// Live enemy instance identity.
    pub enemy_instance_id: String,
    /// Distinct visible intent identity.
    pub intent_id: String,
    /// Ordered component identity.
    pub component_id: String,
}

pub(crate) fn validate_identity(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > ENEMY_INTENT_MAX_IDENTITY_BYTES
        || value.bytes().any(|byte| {
            !byte.is_ascii_alphanumeric() && !matches!(byte, b'.' | b':' | b'/' | b'_' | b'-')
        })
    {
        return Err(field);
    }
    Ok(())
}

pub(crate) fn validate_kind(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > ENEMY_INTENT_MAX_KIND_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(field);
    }
    Ok(())
}

pub(crate) fn validate_text(value: &str, field: &'static str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > ENEMY_INTENT_MAX_TEXT_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(field);
    }
    Ok(())
}
