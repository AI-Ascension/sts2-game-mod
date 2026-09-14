// SPDX-License-Identifier: MIT

mod encoding;
mod encounter;

pub use encounter::*;

pub(super) use encoding::definition_bytes;

use crate::ContentUnlockState;

use super::ActSemanticReference;
use super::model::{
    ActCatalogBinding, ActDefinitionReference, ActEvidence, ActField, ActNumericValue, ActText,
    ActVisibility,
};

/// Coarse room/node category copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum RoomCategoryKind {
    /// Ordinary monster room.
    Normal,
    /// Elite monster room.
    Elite,
    /// Boss room.
    Boss,
    /// Shop room.
    Shop,
    /// Rest or campfire room.
    Rest,
    /// Treasure or chest room.
    Treasure,
    /// Event room.
    Event,
    /// Owner-defined node category.
    Custom(String),
    /// A category is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the node category.
    Unknown,
}

/// One room/node category in an act.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoomCategoryDefinition {
    /// Stable category identity scoped by the act.
    pub category_id: String,
    /// Localized category name.
    pub name: ActText,
    /// Category taxonomy.
    pub kind: RoomCategoryKind,
    /// Localized category description.
    pub description: ActText,
    /// Typed links associated with this category.
    pub references: Vec<ActSemanticReference>,
    /// Visibility of the static category.
    pub visibility: ActVisibility,
}

/// Coarse eligibility predicate category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EligibilityKind {
    /// Minimum or exact act order.
    ActOrder,
    /// Game mode identity.
    Mode,
    /// Difficulty identity.
    Difficulty,
    /// Progression/unlock requirement.
    Unlock,
    /// Content-configuration flag.
    ContentFlag,
    /// Owner rule reference.
    Rule,
    /// Owner-defined predicate.
    Custom(String),
    /// Source could not classify the predicate.
    Unknown,
}

/// One visible parameter retained without evaluating hidden state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActParameter {
    /// Stable parameter identity.
    pub parameter_id: String,
    /// Localized/source-defined label.
    pub label: ActText,
    /// Optional unit.
    pub unit: Option<String>,
    /// Fixed, formula-backed, or unavailable value.
    pub value: ActNumericValue,
}

/// One eligibility predicate for an encounter or assignment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EligibilityCondition {
    /// Stable condition identity.
    pub condition_id: String,
    /// Predicate category.
    pub kind: EligibilityKind,
    /// Localized/source-defined condition label.
    pub label: ActText,
    /// Typed parameters used by the condition.
    pub parameters: Vec<ActParameter>,
    /// Typed rule/content links.
    pub references: Vec<ActSemanticReference>,
    /// Visibility of the static predicate.
    pub visibility: ActVisibility,
}

/// Coarse map-generation constraint category.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MapConstraintKind {
    /// Constrains where rooms may be placed.
    RoomPlacement,
    /// Constrains path count or path shape.
    PathCount,
    /// Constrains boss placement.
    BossPlacement,
    /// Constrains elite count.
    EliteCount,
    /// Fixes a room at a map position.
    FixedRoom,
    /// Owner rule reference.
    Rule,
    /// Owner-defined constraint.
    Custom(String),
    /// Source could not classify the constraint.
    Unknown,
}

/// One map-generation constraint or rule reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MapGenerationConstraint {
    /// Stable constraint identity scoped by the act.
    pub constraint_id: String,
    /// Constraint category.
    pub kind: MapConstraintKind,
    /// Localized/source-defined constraint label.
    pub label: ActText,
    /// Owner rule reference, preserving unavailable/empty distinctions.
    pub rule_reference: ActField<String>,
    /// Optional mode this constraint depends on.
    pub mode: Option<String>,
    /// Optional difficulty this constraint depends on.
    pub difficulty: Option<String>,
    /// Typed parameters used by the constraint.
    pub parameters: Vec<ActParameter>,
    /// Typed rule/content links.
    pub references: Vec<ActSemanticReference>,
    /// Evidence label for this constraint.
    pub evidence: ActEvidence,
    /// Visibility of the static constraint.
    pub visibility: ActVisibility,
}

/// Complete source-owned static act definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActDefinitionInput {
    /// Namespaced act identity.
    pub act_id: String,
    /// Localized act name.
    pub name: ActText,
    /// Localized act description.
    pub description: ActText,
    /// Owner-defined act order within the run.
    pub order: u16,
    /// Explicit unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition itself.
    pub visibility: ActVisibility,
    /// Room/node categories.
    pub room_categories: Vec<RoomCategoryDefinition>,
    /// Encounter definitions.
    pub encounters: Vec<EncounterDefinitionInput>,
    /// Encounter pools, preserving unavailable/empty distinctions.
    pub pools: ActField<Vec<EncounterPool>>,
    /// Map-generation constraints, preserving unavailable/empty distinctions.
    pub constraints: ActField<Vec<MapGenerationConstraint>>,
    /// Top-level typed references.
    pub references: Vec<ActSemanticReference>,
}

/// Immutable act definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActDefinition {
    /// Exact static definition reference.
    pub reference: ActDefinitionReference,
    /// Localized act name.
    pub name: ActText,
    /// Localized act description.
    pub description: ActText,
    /// Owner-defined act order.
    pub order: u16,
    /// Unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition.
    pub visibility: ActVisibility,
    /// Room/node categories.
    pub room_categories: Vec<RoomCategoryDefinition>,
    /// Encounter definitions.
    pub encounters: Vec<EncounterDefinition>,
    /// Encounter pools.
    pub pools: ActField<Vec<EncounterPool>>,
    /// Map-generation constraints.
    pub constraints: ActField<Vec<MapGenerationConstraint>>,
    /// Top-level references.
    pub references: Vec<ActSemanticReference>,
}

impl ActDefinition {
    /// Binds an input definition and all of its encounter references to one catalog.
    pub(super) fn from_input(binding: &ActCatalogBinding, input: ActDefinitionInput) -> Self {
        let act_id = input.act_id.clone();
        let encounters = input
            .encounters
            .into_iter()
            .map(|encounter| EncounterDefinition::from_input(&act_id, binding, encounter))
            .collect();
        Self {
            reference: ActDefinitionReference {
                catalog: binding.clone(),
                act_id,
            },
            name: input.name,
            description: input.description,
            order: input.order,
            unlock_state: input.unlock_state,
            visibility: input.visibility,
            room_categories: input.room_categories,
            encounters,
            pools: input.pools,
            constraints: input.constraints,
            references: input.references,
        }
    }
}
