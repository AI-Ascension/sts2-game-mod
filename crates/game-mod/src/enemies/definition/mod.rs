// SPDX-License-Identifier: MIT

mod encoding;
mod r#move;
mod stats;

pub use r#move::*;
pub use stats::*;

pub(super) use encoding::definition_bytes;

use crate::ContentUnlockState;

use super::model::{
    EnemyCatalogBinding, EnemyDefinitionReference, EnemyField, EnemyKind, EnemyMoveReference,
    EnemyOrigin, EnemyProbability, EnemyText, EnemyVisibility,
};

/// One behavior phase and its move membership.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyPhaseDefinition {
    /// Stable phase identity.
    pub phase_id: String,
    /// Localized phase name.
    pub name: EnemyText,
    /// Localized phase description.
    pub description: EnemyText,
    /// Stable ordering within the definition.
    pub order: u16,
    /// Moves available in this phase.
    pub move_ids: Vec<String>,
    /// Optional entry predicate.
    pub entry_condition: EnemyField<EnemyConditionReference>,
}

/// One transition between behavior phases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyBehaviorTransition {
    /// Stable transition identity.
    pub transition_id: String,
    /// Optional source phase; `None` means the initial phase.
    pub from_phase: Option<String>,
    /// Destination phase identity.
    pub to_phase: String,
    /// Predicate for the transition.
    pub condition: EnemyConditionReference,
    /// Optional source-defined probability/weight.
    pub probability: EnemyProbability,
    /// Typed rule/effect/status references.
    pub references: Vec<EnemySemanticReference>,
}

/// Complete source-owned static enemy definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyDefinitionInput {
    /// Namespaced enemy identity.
    pub enemy_id: String,
    /// Localized enemy name.
    pub name: EnemyText,
    /// Localized enemy description.
    pub description: EnemyText,
    /// Enemy role.
    pub kind: EnemyKind,
    /// Source origin/provenance.
    pub origin: EnemyOrigin,
    /// Explicit unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition itself.
    pub visibility: EnemyVisibility,
    /// Stable tags.
    pub tags: Vec<EnemyTag>,
    /// Base and scaled stats.
    pub stats: EnemyStats,
    /// Spawn predicates, preserving unavailable/empty distinctions.
    pub spawn_conditions: EnemyField<Vec<EnemyConditionReference>>,
    /// Encounter references, preserving unavailable/empty distinctions.
    pub encounters: EnemyField<Vec<EnemyEncounterReference>>,
    /// Origin/package variants.
    pub origin_variants: Vec<EnemyOriginVariant>,
    /// Behavior phases.
    pub phases: Vec<EnemyPhaseDefinition>,
    /// Supported move rules.
    pub moves: Vec<EnemyMoveDefinitionInput>,
    /// Phase transitions and conditional behavior.
    pub transitions: Vec<EnemyBehaviorTransition>,
    /// Top-level status/encounter/rule links.
    pub references: Vec<EnemySemanticReference>,
}

/// Immutable enemy definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyDefinition {
    /// Exact static definition reference.
    pub reference: EnemyDefinitionReference,
    /// Localized enemy name.
    pub name: EnemyText,
    /// Localized enemy description.
    pub description: EnemyText,
    /// Enemy role.
    pub kind: EnemyKind,
    /// Source origin/provenance.
    pub origin: EnemyOrigin,
    /// Unlock/availability observation.
    pub unlock_state: ContentUnlockState,
    /// Visibility of the definition.
    pub visibility: EnemyVisibility,
    /// Stable tags.
    pub tags: Vec<EnemyTag>,
    /// Base and scaled stats.
    pub stats: EnemyStats,
    /// Spawn predicates.
    pub spawn_conditions: EnemyField<Vec<EnemyConditionReference>>,
    /// Encounter references.
    pub encounters: EnemyField<Vec<EnemyEncounterReference>>,
    /// Origin/package variants.
    pub origin_variants: Vec<EnemyOriginVariant>,
    /// Behavior phases.
    pub phases: Vec<EnemyPhaseDefinition>,
    /// Supported move rules.
    pub moves: Vec<EnemyMoveDefinition>,
    /// Phase transitions and conditional behavior.
    pub transitions: Vec<EnemyBehaviorTransition>,
    /// Top-level references.
    pub references: Vec<EnemySemanticReference>,
}

impl EnemyDefinition {
    /// Binds an input definition and all of its move references to one catalog.
    pub(super) fn from_input(binding: &EnemyCatalogBinding, input: EnemyDefinitionInput) -> Self {
        let enemy_id = input.enemy_id.clone();
        let moves = input
            .moves
            .into_iter()
            .map(|move_input| EnemyMoveDefinition {
                reference: EnemyMoveReference {
                    catalog: binding.clone(),
                    enemy_id: enemy_id.clone(),
                    move_id: move_input.move_id,
                },
                name: move_input.name,
                description: move_input.description,
                effects: move_input.effects,
                targeting: move_input.targeting,
                phase_ids: move_input.phase_ids,
                conditions: move_input.conditions,
                cooldown: move_input.cooldown,
                repetition: move_input.repetition,
                probability: move_input.probability,
                references: move_input.references,
                visibility: move_input.visibility,
            })
            .collect();
        Self {
            reference: EnemyDefinitionReference {
                catalog: binding.clone(),
                enemy_id,
            },
            name: input.name,
            description: input.description,
            kind: input.kind,
            origin: input.origin,
            unlock_state: input.unlock_state,
            visibility: input.visibility,
            tags: input.tags,
            stats: input.stats,
            spawn_conditions: input.spawn_conditions,
            encounters: input.encounters,
            origin_variants: input.origin_variants,
            phases: input.phases,
            moves,
            transitions: input.transitions,
            references: input.references,
        }
    }
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: EnemyFamilyState,
    /// Number of enemy definitions in the manifest.
    pub definition_count: usize,
}

/// Source support state for the enemy family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EnemyFamilyState {
    /// Source can project all enemy records.
    Handled,
    /// Family exists but no typed source adapter is available.
    Unsupported,
    /// Family is known but currently unavailable.
    Unavailable,
}
