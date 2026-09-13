// SPDX-License-Identifier: MIT

use super::model::{CharacterStateCatalogBinding, CharacterStateUnit, CharacterStateVisibility};

/// Resource families retained without assuming a specific character.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterResourceKind {
    /// A bounded count such as charges or counters.
    Counter,
    /// A scalar meter with a current and maximum.
    Meter,
    /// A spendable charge pool.
    Charges,
    /// A stance or mode marker.
    Stance,
    /// An ordered slot system.
    SlotSystem,
    /// A source-defined family.
    Custom(String),
}

/// Typed shape declared by a static resource definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterResourceValueDefinition {
    /// One signed integer with a unit.
    Integer { unit: CharacterStateUnit },
    /// A fixed-point value with decimal places.
    Decimal {
        /// Unit for the scaled value.
        unit: CharacterStateUnit,
        /// Number of decimal places.
        scale: u8,
    },
    /// Boolean state with a source-defined unit or label.
    Boolean { unit: CharacterStateUnit },
    /// Bounded textual state.
    Text,
    /// Source-defined typed value retained without executable semantics.
    Custom {
        kind: String,
        unit: Option<CharacterStateUnit>,
    },
}

/// Static resource definition before catalog binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterResourceDefinitionInput {
    /// Namespaced resource definition identity.
    pub definition_id: String,
    /// Character identity that owns this definition.
    pub character_id: String,
    /// Mode/difficulty identity.
    pub mode_id: String,
    /// Resource family.
    pub kind: CharacterResourceKind,
    /// Localized resource label.
    pub label: String,
    /// Stable owner rule/mechanic reference.
    pub rule_reference: String,
    /// Typed current/max value shape.
    pub value: CharacterResourceValueDefinition,
    /// Optional inclusive maximum copied from the source rule.
    pub maximum: Option<i64>,
    /// Visibility of the definition.
    pub visibility: CharacterStateVisibility,
    /// Whether ordered slot contents are publicly observable.
    pub slots_visibility: CharacterStateVisibility,
}

/// Static resource definition bound to a catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterResourceDefinition {
    /// Exact static reference.
    pub reference: CharacterResourceDefinitionReference,
    /// Character identity.
    pub character_id: String,
    /// Mode/difficulty identity.
    pub mode_id: String,
    /// Resource family.
    pub kind: CharacterResourceKind,
    /// Localized label.
    pub label: String,
    /// Stable owner rule/mechanic reference.
    pub rule_reference: String,
    /// Typed current/max value shape.
    pub value: CharacterResourceValueDefinition,
    /// Optional inclusive maximum.
    pub maximum: Option<i64>,
    /// Definition visibility.
    pub visibility: CharacterStateVisibility,
    /// Slot-content visibility.
    pub slots_visibility: CharacterStateVisibility,
}

/// Exact static resource definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CharacterResourceDefinitionReference {
    /// Catalog witness owning the definition.
    pub catalog: CharacterStateCatalogBinding,
    /// Namespaced definition identity.
    pub definition_id: String,
}

/// Typed current or maximum resource value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CharacterResourceValue {
    /// Signed integer with a unit.
    Integer {
        value: i64,
        unit: CharacterStateUnit,
    },
    /// Fixed-point value with decimal places.
    Decimal {
        /// Scaled integer value.
        value: i64,
        /// Decimal places.
        scale: u8,
        /// Unit.
        unit: CharacterStateUnit,
    },
    /// Boolean state.
    Boolean {
        value: bool,
        unit: CharacterStateUnit,
    },
    /// Bounded textual state.
    Text(String),
    /// Source-defined typed value.
    Custom {
        /// Source-defined kind.
        kind: String,
        /// Bounded source value.
        value: String,
        /// Optional unit.
        unit: Option<CharacterStateUnit>,
    },
}

/// Secondary combat entity families.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SecondaryEntityKind {
    /// A player-controlled summon.
    Summon,
    /// A friendly companion.
    Companion,
    /// An orb or persistent combat object.
    Orb,
    /// A minion controlled by an owner.
    Minion,
    /// An encounter-owned object.
    EncounterObject,
    /// A source-defined family.
    Custom(String),
}

/// Static secondary-entity definition before catalog binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecondaryEntityDefinitionInput {
    /// Namespaced entity definition identity.
    pub definition_id: String,
    /// Character identity that controls this entity family.
    pub character_id: String,
    /// Mode/difficulty identity.
    pub mode_id: String,
    /// Entity family.
    pub kind: SecondaryEntityKind,
    /// Localized entity label.
    pub label: String,
    /// Stable owner rule/mechanic reference.
    pub rule_reference: String,
    /// Visibility of the definition.
    pub visibility: CharacterStateVisibility,
}

/// Static secondary-entity definition bound to a catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecondaryEntityDefinition {
    /// Exact static reference.
    pub reference: SecondaryEntityDefinitionReference,
    /// Character identity.
    pub character_id: String,
    /// Mode/difficulty identity.
    pub mode_id: String,
    /// Entity family.
    pub kind: SecondaryEntityKind,
    /// Localized entity label.
    pub label: String,
    /// Stable owner rule/mechanic reference.
    pub rule_reference: String,
    /// Definition visibility.
    pub visibility: CharacterStateVisibility,
}

/// Exact static secondary-entity definition reference.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SecondaryEntityDefinitionReference {
    /// Catalog witness owning the definition.
    pub catalog: CharacterStateCatalogBinding,
    /// Namespaced definition identity.
    pub definition_id: String,
}
