// SPDX-License-Identifier: MIT

use super::{
    definition::{CharacterResourceDefinition, CharacterResourceValue, SecondaryEntityDefinition},
    model::{CharacterStateField, CharacterStateOwner, CharacterStateVisibility},
};

/// One ordered slot copied from a public resource slot system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterResourceSlot {
    /// Source-defined slot identity.
    pub slot_id: String,
    /// Stable source ordering position.
    pub position: u32,
    /// Explicitly available or unavailable slot content.
    pub content: CharacterStateField<CharacterResourceSlotContent>,
}

/// Typed content retained in one resource slot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterResourceSlotContent {
    /// Slot is explicitly empty.
    Empty,
    /// Slot points to a static definition.
    Definition(String),
    /// Slot points to another live resource/card/entity instance.
    Instance(String),
    /// Source-visible bounded label.
    Text(String),
    /// Source-defined slot value.
    Custom { kind: String, value: String },
}

/// Source-owned live resource before its coherent snapshot is attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterResourceInput {
    /// Stable live resource identity, distinct from its definition ID.
    pub instance_id: String,
    /// Static resource definition identity.
    pub definition_id: String,
    /// Resource owner identity.
    pub owner: CharacterStateOwner,
    /// Current typed value.
    pub current: CharacterStateField<CharacterResourceValue>,
    /// Maximum typed value, when the source exposes one.
    pub maximum: CharacterStateField<CharacterResourceValue>,
    /// Ordered slot contents where the definition exposes a slot system.
    pub slots: CharacterStateField<Vec<CharacterResourceSlot>>,
    /// Whether the mechanic is currently active.
    pub active: CharacterStateField<bool>,
}

/// Live resource detail joined to its static definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterResource {
    /// Exact live resource identity.
    pub reference: super::model::CharacterResourceReference,
    /// Static resource definition.
    pub definition: CharacterResourceDefinition,
    /// Resource owner.
    pub owner: CharacterStateOwner,
    /// Current typed value.
    pub current: CharacterStateField<CharacterResourceValue>,
    /// Maximum typed value.
    pub maximum: CharacterStateField<CharacterResourceValue>,
    /// Ordered slot contents.
    pub slots: CharacterStateField<Vec<CharacterResourceSlot>>,
    /// Active state.
    pub active: CharacterStateField<bool>,
}

/// Visible link from a secondary entity to its controlling owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecondaryEntityControllerReference {
    /// Controller owner family.
    pub owner_kind: super::model::CharacterStateOwnerKind,
    /// Controller live identity.
    pub owner_id: super::model::CharacterStateOwnerId,
    /// Controller character identity.
    pub character_id: String,
    /// Controller label when visible.
    pub label: CharacterStateField<String>,
}

/// One status attached to a secondary entity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecondaryEntityStatus {
    /// Distinct live status identity.
    pub instance_id: String,
    /// Status definition identity.
    pub definition_id: String,
    /// Current integer amount when visible.
    pub amount: CharacterStateField<i64>,
    /// Explicit status visibility.
    pub visibility: CharacterStateVisibility,
    /// Localized label when visible.
    pub label: CharacterStateField<String>,
}

/// Target shape for a secondary-entity intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SecondaryEntityTarget {
    /// One owner identity.
    Owner(String),
    /// One secondary/entity identity.
    Entity(String),
    /// Every visible valid target.
    All,
    /// Source-defined target shape.
    Custom(String),
}

/// Kind of pending secondary-entity intent.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SecondaryEntityIntentKind {
    /// Deal damage or apply a harmful effect.
    Attack,
    /// Gain block or defensive state.
    Defend,
    /// Apply a beneficial effect.
    Buff,
    /// Restore or heal.
    Heal,
    /// Source-defined action family.
    Custom(String),
}

/// One pending visible secondary-entity intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecondaryEntityIntent {
    /// Intent family.
    pub kind: SecondaryEntityIntentKind,
    /// Stable owner rule/effect reference when supplied.
    pub rule_reference: CharacterStateField<String>,
    /// Target shape.
    pub target: CharacterStateField<SecondaryEntityTarget>,
    /// Integer amount when the source supplies one.
    pub amount: CharacterStateField<i64>,
    /// Localized intent label.
    pub label: CharacterStateField<String>,
    /// Explicit intent visibility.
    pub visibility: CharacterStateVisibility,
}

/// Source-owned secondary entity before its coherent snapshot is attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecondaryEntityInput {
    /// Stable live entity identity.
    pub instance_id: String,
    /// Static entity definition identity.
    pub definition_id: String,
    /// Entity owner identity.
    pub owner: CharacterStateOwner,
    /// Visible controller link.
    pub controller: CharacterStateField<SecondaryEntityControllerReference>,
    /// Current hit points.
    pub hp: CharacterStateField<i64>,
    /// Maximum hit points.
    pub maximum_hp: CharacterStateField<i64>,
    /// Current block.
    pub block: CharacterStateField<i64>,
    /// Ordered status values.
    pub statuses: CharacterStateField<Vec<SecondaryEntityStatus>>,
    /// Pending intent.
    pub intent: CharacterStateField<SecondaryEntityIntent>,
    /// Whether the entity remains active.
    pub active: CharacterStateField<bool>,
}

/// Live secondary-entity detail joined to its static definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterSecondaryEntity {
    /// Exact live entity identity.
    pub reference: super::model::CharacterSecondaryEntityReference,
    /// Static entity definition.
    pub definition: SecondaryEntityDefinition,
    /// Entity owner identity.
    pub owner: CharacterStateOwner,
    /// Visible controller link.
    pub controller: CharacterStateField<SecondaryEntityControllerReference>,
    /// Current hit points.
    pub hp: CharacterStateField<i64>,
    /// Maximum hit points.
    pub maximum_hp: CharacterStateField<i64>,
    /// Current block.
    pub block: CharacterStateField<i64>,
    /// Ordered status values.
    pub statuses: CharacterStateField<Vec<SecondaryEntityStatus>>,
    /// Pending intent.
    pub intent: CharacterStateField<SecondaryEntityIntent>,
    /// Whether the entity remains active.
    pub active: CharacterStateField<bool>,
}
