// SPDX-License-Identifier: MIT

use super::super::ActSemanticReference;
use super::super::model::{
    ActEncounterReference, ActNumericValue, ActText, ActVisibility, GenerationWeight,
};
use super::EligibilityCondition;

/// Coarse encounter category copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EncounterKind {
    /// Ordinary encounter.
    Normal,
    /// Elite encounter.
    Elite,
    /// Boss or act-ending encounter.
    Boss,
    /// Event or non-combat encounter.
    Event,
    /// Owner-defined encounter category.
    Custom(String),
    /// A category is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the encounter.
    Unknown,
}

/// One enemy slot inside an encounter group, with an explicit quantity and variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterEnemy {
    /// Namespaced enemy definition identity.
    pub enemy_id: String,
    /// Number of this enemy, or an explicit formula/unavailable value.
    pub quantity: ActNumericValue,
    /// Stable variant identities of this enemy.
    pub variant_ids: Vec<String>,
    /// Typed links to the enemy or associated rules.
    pub references: Vec<ActSemanticReference>,
}

/// One ordered enemy group in an encounter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterEnemyGroup {
    /// Stable group identity scoped by the encounter.
    pub group_id: String,
    /// Localized/source-defined group label.
    pub label: ActText,
    /// Ordered enemy slots in this group.
    pub enemies: Vec<EncounterEnemy>,
    /// Typed links associated with this group.
    pub references: Vec<ActSemanticReference>,
}

/// Source-owned static encounter definition before manifest binding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterDefinitionInput {
    /// Stable encounter identity scoped by the act.
    pub encounter_id: String,
    /// Localized encounter name.
    pub name: ActText,
    /// Encounter category.
    pub kind: EncounterKind,
    /// Optional room/node category this encounter belongs to.
    pub room_category_id: Option<String>,
    /// Ordered enemy composition groups.
    pub groups: Vec<EncounterEnemyGroup>,
    /// Eligibility predicates for this encounter.
    pub eligibility: Vec<EligibilityCondition>,
    /// Generation weight, never evaluated as live RNG.
    pub weight: GenerationWeight,
    /// Typed enemy/rule references.
    pub references: Vec<ActSemanticReference>,
    /// Visibility of the static encounter rule.
    pub visibility: ActVisibility,
}

/// Static encounter definition bound to its owning act and catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterDefinition {
    /// Exact encounter reference.
    pub reference: ActEncounterReference,
    /// Localized encounter name.
    pub name: ActText,
    /// Encounter category.
    pub kind: EncounterKind,
    /// Optional room/node category this encounter belongs to.
    pub room_category_id: Option<String>,
    /// Ordered enemy composition groups.
    pub groups: Vec<EncounterEnemyGroup>,
    /// Eligibility predicates for this encounter.
    pub eligibility: Vec<EligibilityCondition>,
    /// Generation weight with evidence.
    pub weight: GenerationWeight,
    /// Typed references.
    pub references: Vec<ActSemanticReference>,
    /// Visibility of the static encounter rule.
    pub visibility: ActVisibility,
}

impl EncounterDefinition {
    pub(super) fn from_input(
        act_id: &str,
        binding: &super::super::model::ActCatalogBinding,
        input: EncounterDefinitionInput,
    ) -> Self {
        Self {
            reference: ActEncounterReference {
                catalog: binding.clone(),
                act_id: act_id.to_owned(),
                encounter_id: input.encounter_id,
            },
            name: input.name,
            kind: input.kind,
            room_category_id: input.room_category_id,
            groups: input.groups,
            eligibility: input.eligibility,
            weight: input.weight,
            references: input.references,
            visibility: input.visibility,
        }
    }
}

/// Coarse encounter-pool category copied from the owner source.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum EncounterPoolKind {
    /// Pool of ordinary encounters.
    Normal,
    /// Pool of elite encounters.
    Elite,
    /// Pool of boss encounters.
    Boss,
    /// Pool of event encounters.
    Event,
    /// Pool mixing multiple categories.
    Mixed,
    /// Owner-defined pool category.
    Custom(String),
    /// A category is known but unsupported by this producer.
    Unsupported(String),
    /// Source could not classify the pool.
    Unknown,
}

/// One weighted entry in an encounter pool.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterPoolEntry {
    /// Stable entry identity scoped by the pool.
    pub entry_id: String,
    /// Encounter definition identity scoped by the owning act.
    pub encounter_id: String,
    /// Entry generation weight.
    pub weight: GenerationWeight,
}

/// An encounter pool with generation weights.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncounterPool {
    /// Stable pool identity scoped by the act.
    pub pool_id: String,
    /// Localized/source-defined pool name.
    pub name: ActText,
    /// Pool category.
    pub kind: EncounterPoolKind,
    /// Optional room/node category this pool belongs to.
    pub room_category_id: Option<String>,
    /// Weighted encounter entries.
    pub entries: Vec<EncounterPoolEntry>,
    /// Typed links associated with this pool.
    pub references: Vec<ActSemanticReference>,
    /// Visibility of the static pool rule.
    pub visibility: ActVisibility,
}
