// SPDX-License-Identifier: MIT

use crate::ContentUnlockState;

use super::model::{
    CharacterCatalogBinding, CharacterContentReference, CharacterDefinitionReference,
    CharacterField, CharacterNumericValue, CharacterOrigin, CharacterText,
};

/// Static starting resource value with a generic resource identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterResource {
    /// Owner-defined resource identity; no resource type is assumed.
    pub resource_id: String,
    /// Optional owner-defined unit for the amount and capacity.
    pub unit: Option<String>,
    /// Starting amount or explicit formula/unavailable state.
    pub amount: CharacterNumericValue,
    /// Optional maximum/capacity value.
    pub capacity: Option<CharacterNumericValue>,
}

/// Starting HP, gold, potion capacity, and generic resources for one loadout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterStartingConfiguration {
    /// Starting HP.
    pub starting_hp: CharacterNumericValue,
    /// Maximum HP at run initialization.
    pub max_hp: CharacterNumericValue,
    /// Starting gold.
    pub gold: CharacterNumericValue,
    /// Maximum potion capacity.
    pub potion_capacity: CharacterNumericValue,
    /// Generic named resources and their starting values.
    pub resources: CharacterField<Vec<CharacterResource>>,
}

/// A generic prerequisite keyed by a progression or mode identifier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterLoadoutRequirement {
    /// Stable progression/configuration identifier.
    pub progression_id: String,
    /// Optional localized/source-defined explanation.
    pub description: CharacterText,
}

/// Stable requirement alias used by character unlock records.
pub type CharacterUnlockRequirement = CharacterLoadoutRequirement;

/// A reference to a character-specific mechanic without assuming its resource or rule type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterMechanicReference {
    /// Stable mechanic/rule identity.
    pub mechanic_id: String,
    /// Optional localized/source-defined label.
    pub label: CharacterText,
    /// Explicit unresolved dependencies for the mechanic, if any.
    pub dependencies: Vec<String>,
}

/// A pool/table reference retained separately from concrete starting content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterPoolReference {
    /// Content family or pool family.
    pub entity_kind: String,
    /// Stable owner-defined pool identity.
    pub pool_id: String,
    /// Optional source-defined pool label.
    pub label: CharacterText,
}

/// One mode/difficulty/loadout-dependent starting variant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterLoadoutInput {
    /// Stable loadout identity scoped by the character.
    pub loadout_id: String,
    /// Source-defined mode identity, if the source distinguishes modes.
    pub mode: Option<String>,
    /// Source-defined difficulty identity, if the source distinguishes difficulties.
    pub difficulty: Option<String>,
    /// Origin/provenance for this loadout variant.
    pub origin: CharacterOrigin,
    /// Whether the owner reports this mode/loadout as available.
    pub availability: CharacterLoadoutAvailability,
    /// Starting values for this loadout.
    pub starting: CharacterStartingConfiguration,
    /// Concrete starting deck references.
    pub starting_deck: CharacterField<Vec<CharacterContentReference>>,
    /// Starting relic/content references.
    pub starting_relics: CharacterField<Vec<CharacterContentReference>>,
    /// Pool/table references available to this loadout.
    pub pools: CharacterField<Vec<CharacterPoolReference>>,
    /// Character-specific mechanic references.
    pub mechanics: CharacterField<Vec<CharacterMechanicReference>>,
    /// Mode/progression prerequisites.
    pub prerequisites: CharacterField<Vec<CharacterLoadoutRequirement>>,
}

/// Validated loadout shape retained by the immutable catalog.
pub type CharacterLoadout = CharacterLoadoutInput;

/// Explicit availability for a mode/difficulty/loadout variant.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterLoadoutAvailability {
    /// The source observed that this variant can be selected.
    Available,
    /// The source knows the variant but cannot offer it for the selected configuration.
    Unavailable(super::model::CharacterUnavailableReason),
}

/// Unlock observation and requirements copied without mutating profile state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterUnlock {
    /// Explicit source observation.
    pub state: ContentUnlockState,
    /// Stable progression identifiers required to unlock the character.
    pub requirements: CharacterField<Vec<CharacterLoadoutRequirement>>,
}

/// Complete source-owned static character definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterDefinitionInput {
    /// Namespaced character identity.
    pub character_id: String,
    /// Localized character name.
    pub name: CharacterText,
    /// Localized character description.
    pub description: CharacterText,
    /// Source origin/provenance.
    pub origin: CharacterOrigin,
    /// Mode/difficulty/loadout-dependent starting variants.
    pub loadouts: Vec<CharacterLoadoutInput>,
    /// Unlock state and progression requirements.
    pub unlock: CharacterField<CharacterUnlock>,
}

/// Static family coverage, including explicit unsupported/unavailable states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterFamilyCoverage {
    /// Family identity.
    pub entity_kind: String,
    /// Source support state.
    pub state: CharacterFamilyState,
    /// Number of character definitions in the manifest.
    pub definition_count: usize,
}

/// Source support state for the character family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CharacterFamilyState {
    /// The source can project all character records.
    Handled,
    /// The family exists but no typed source adapter is available.
    Unsupported,
    /// The family is known but currently unavailable.
    Unavailable,
}

/// Immutable definition bound to a manifest and locale.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterDefinition {
    /// Exact manifest/locale-bound definition reference.
    pub reference: CharacterDefinitionReference,
    /// Localized character name.
    pub name: CharacterText,
    /// Localized character description.
    pub description: CharacterText,
    /// Source origin/provenance.
    pub origin: CharacterOrigin,
    /// Mode/difficulty/loadout variants.
    pub loadouts: Vec<CharacterLoadoutInput>,
    /// Unlock state and progression requirements.
    pub unlock: CharacterField<CharacterUnlock>,
}

impl CharacterDefinition {
    pub(super) fn from_input(
        binding: &CharacterCatalogBinding,
        input: CharacterDefinitionInput,
    ) -> Self {
        Self {
            reference: CharacterDefinitionReference {
                catalog: binding.clone(),
                character_id: input.character_id,
            },
            name: input.name,
            description: input.description,
            origin: input.origin,
            loadouts: input.loadouts,
            unlock: input.unlock,
        }
    }
}

/// Returns the number of bytes in one source-owned definition for the aggregate bound.
pub(super) fn definition_bytes(input: &CharacterDefinitionInput) -> usize {
    let mut total = input.character_id.len()
        + text_bytes(&input.name)
        + text_bytes(&input.description)
        + origin_bytes(&input.origin)
        + 4;
    total += field_bytes(&input.unlock, unlock_bytes);
    for loadout in &input.loadouts {
        total += loadout.loadout_id.len()
            + loadout.mode.as_deref().map_or(0, str::len)
            + loadout.difficulty.as_deref().map_or(0, str::len)
            + origin_bytes(&loadout.origin)
            + 5;
        total += numeric_bytes(&loadout.starting.starting_hp)
            + numeric_bytes(&loadout.starting.max_hp)
            + numeric_bytes(&loadout.starting.gold)
            + numeric_bytes(&loadout.starting.potion_capacity);
        total += field_bytes(&loadout.starting.resources, |values| {
            resources_bytes(values)
        });
        total += field_bytes(&loadout.starting_deck, |values| {
            values.iter().map(content_bytes).sum()
        });
        total += field_bytes(&loadout.starting_relics, |values| {
            values.iter().map(content_bytes).sum()
        });
        total += field_bytes(&loadout.pools, |values| {
            values
                .iter()
                .map(|pool| {
                    pool.entity_kind.len() + pool.pool_id.len() + text_bytes(&pool.label) + 2
                })
                .sum()
        });
        total += field_bytes(&loadout.mechanics, |values| {
            values
                .iter()
                .map(|mechanic| {
                    mechanic.mechanic_id.len()
                        + text_bytes(&mechanic.label)
                        + mechanic.dependencies.iter().map(String::len).sum::<usize>()
                        + 2
                })
                .sum()
        });
        total += field_bytes(&loadout.prerequisites, |values| requirements_bytes(values));
    }
    total
}

fn text_bytes(value: &CharacterText) -> usize {
    match value {
        CharacterText::Available(value) => value.len(),
        CharacterText::Unavailable(_) => 1,
    }
}

fn numeric_bytes(value: &CharacterNumericValue) -> usize {
    match value {
        CharacterNumericValue::Fixed(_) => 8,
        CharacterNumericValue::Formula(formula) => {
            formula.rule_reference.len()
                + formula
                    .unresolved_inputs
                    .iter()
                    .map(String::len)
                    .sum::<usize>()
                + 2
        }
        CharacterNumericValue::Unavailable(_) => 1,
    }
}

fn field_bytes<T>(field: &CharacterField<T>, available: impl FnOnce(&T) -> usize) -> usize {
    match field {
        CharacterField::Available(value) => available(value),
        CharacterField::Unavailable(_) => 1,
    }
}

fn unlock_bytes(value: &CharacterUnlock) -> usize {
    match &value.requirements {
        CharacterField::Available(requirements) => requirements_bytes(requirements),
        CharacterField::Unavailable(_) => 1,
    }
}

fn resources_bytes(values: &[CharacterResource]) -> usize {
    values
        .iter()
        .map(|resource| {
            resource.resource_id.len()
                + resource.unit.as_deref().map_or(0, str::len)
                + numeric_bytes(&resource.amount)
                + resource.capacity.as_ref().map_or(0, numeric_bytes)
                + 2
        })
        .sum()
}

fn origin_bytes(origin: &CharacterOrigin) -> usize {
    origin.kind.len()
        + origin.package_id.as_deref().map_or(0, str::len)
        + origin.package_version.as_deref().map_or(0, str::len)
        + 2
}

fn content_bytes(value: &CharacterContentReference) -> usize {
    value.entity_kind.len() + value.namespaced_id.len() + 8
}

fn requirements_bytes(requirements: &[CharacterLoadoutRequirement]) -> usize {
    requirements
        .iter()
        .map(|requirement| {
            requirement.progression_id.len() + text_bytes(&requirement.description) + 2
        })
        .sum()
}
