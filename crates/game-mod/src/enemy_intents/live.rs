// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::{
    ENEMY_INTENT_MAX_ENEMIES, ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES, ENEMY_INTENT_PRODUCER_VERSION,
    EnemyIntentComponent, EnemyIntentComponentReference, EnemyIntentEnemyInput,
    EnemyIntentEnemyReference, EnemyIntentError, EnemyIntentField, EnemyIntentInput,
    EnemyIntentLiveBinding, EnemyIntentSnapshotInput, EnemyIntentSource,
    EnemyIntentVisibilityScope, source::map_error, validation::validate_snapshot,
};

/// Immutable validated coherent live snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentLiveSnapshot {
    binding: EnemyIntentLiveBinding,
    enemies: BTreeMap<String, EnemyIntentEnemyInput>,
}

impl EnemyIntentLiveSnapshot {
    /// Validates and adopts source-owned input without retaining host objects.
    pub fn from_input(input: EnemyIntentSnapshotInput) -> Result<Self, EnemyIntentError> {
        validate_snapshot(&input)?;
        if input.enemies.len() > ENEMY_INTENT_MAX_ENEMIES {
            return Err(EnemyIntentError::InvalidInput("enemies"));
        }
        let mut enemies = BTreeMap::new();
        for enemy in input.enemies {
            let id = enemy.enemy_instance_id.clone();
            if enemies.insert(id.clone(), enemy).is_some() {
                return Err(EnemyIntentError::DuplicateEnemy(id));
            }
        }
        Ok(Self {
            binding: input.binding,
            enemies,
        })
    }

    /// Returns the exact snapshot identity fence.
    #[must_use]
    pub fn binding(&self) -> &EnemyIntentLiveBinding {
        &self.binding
    }

    /// Returns the number of retained enemies.
    #[must_use]
    pub fn len(&self) -> usize {
        self.enemies.len()
    }

    /// Returns whether no enemy was retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.enemies.is_empty()
    }
}

/// Live enemy detail joined to an exact snapshot reference.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemyIntentEnemy {
    /// Exact live enemy identity.
    pub reference: EnemyIntentEnemyReference,
    /// Localized enemy name.
    pub name: EnemyIntentField<String>,
    /// Current hit points.
    pub hp: EnemyIntentField<u32>,
    /// Maximum hit points.
    pub max_hp: EnemyIntentField<u32>,
    /// Current block.
    pub block: EnemyIntentField<u32>,
    /// Current visible statuses.
    pub statuses: EnemyIntentField<Vec<super::EnemyIntentStatus>>,
    /// Current structured intent.
    pub intent: EnemyIntentField<EnemyIntentInput>,
}

/// Read-only reader enforcing all live identity and visibility fences.
#[derive(Clone, Debug)]
pub struct EnemyIntentLiveReader {
    snapshot: EnemyIntentLiveSnapshot,
    scope: EnemyIntentVisibilityScope,
}

impl EnemyIntentLiveReader {
    /// Adopts a validated snapshot with public visibility.
    pub fn new(snapshot: EnemyIntentLiveSnapshot) -> Result<Self, EnemyIntentError> {
        Self::new_with_scope(snapshot, EnemyIntentVisibilityScope::Public)
    }

    /// Adopts a validated snapshot with explicit visibility scope.
    pub fn new_with_scope(
        snapshot: EnemyIntentLiveSnapshot,
        scope: EnemyIntentVisibilityScope,
    ) -> Result<Self, EnemyIntentError> {
        if snapshot.binding.catalog.producer_version != ENEMY_INTENT_PRODUCER_VERSION {
            return Err(EnemyIntentError::InvalidBinding("producer_version"));
        }
        Ok(Self { snapshot, scope })
    }

    /// Reads and validates a source snapshot against an exact identity fence.
    pub fn from_source<S: EnemyIntentSource>(
        source: &S,
        expected: &EnemyIntentLiveBinding,
    ) -> Result<Self, EnemyIntentError> {
        Self::from_source_with_scope(source, expected, EnemyIntentVisibilityScope::Public)
    }

    /// Reads one source snapshot with explicit visibility scope.
    pub fn from_source_with_scope<S: EnemyIntentSource>(
        source: &S,
        expected: &EnemyIntentLiveBinding,
        scope: EnemyIntentVisibilityScope,
    ) -> Result<Self, EnemyIntentError> {
        if let super::EnemyIntentCapability::Unavailable(reason) = source.capability() {
            return Err(EnemyIntentError::Unavailable(reason));
        }
        let input = source.read_snapshot(expected, scope).map_err(map_error)?;
        if input.binding != *expected {
            return Err(EnemyIntentError::StaleSource);
        }
        Self::new_with_scope(EnemyIntentLiveSnapshot::from_input(input)?, scope)
    }

    /// Returns the current exact identity fence.
    #[must_use]
    pub fn binding(&self) -> &EnemyIntentLiveBinding {
        self.snapshot.binding()
    }

    /// Returns the selected visibility scope.
    #[must_use]
    pub const fn scope(&self) -> EnemyIntentVisibilityScope {
        self.scope
    }

    /// Replaces the snapshot only with the same catalog/game/run/combat and a newer epoch.
    pub fn replace_snapshot(
        &mut self,
        next: EnemyIntentLiveSnapshot,
    ) -> Result<(), EnemyIntentError> {
        if next.binding.catalog != self.snapshot.binding.catalog {
            return Err(EnemyIntentError::CatalogMismatch);
        }
        if next.binding.game_instance_id != self.snapshot.binding.game_instance_id {
            return Err(EnemyIntentError::GameInstanceMismatch);
        }
        if next.binding.run_id != self.snapshot.binding.run_id {
            return Err(EnemyIntentError::RunMismatch);
        }
        if next.binding.combat_id != self.snapshot.binding.combat_id {
            return Err(EnemyIntentError::CombatMismatch);
        }
        if next.binding.epoch <= self.snapshot.binding.epoch {
            return Err(EnemyIntentError::NonMonotonicEpoch {
                current: self.snapshot.binding.epoch,
                supplied: next.binding.epoch,
            });
        }
        self.snapshot = next;
        Ok(())
    }

    /// Returns live enemy references in deterministic instance-identity order.
    #[must_use]
    pub fn references(&self) -> Vec<EnemyIntentEnemyReference> {
        self.snapshot
            .enemies
            .values()
            .map(|enemy| self.reference_for(enemy))
            .collect()
    }

    /// Reads one enemy detail without mutating the snapshot or source.
    pub fn get(
        &self,
        reference: &EnemyIntentEnemyReference,
    ) -> Result<EnemyIntentEnemy, EnemyIntentError> {
        if reference.binding != self.snapshot.binding {
            return Err(EnemyIntentError::StaleReference);
        }
        let enemy = self
            .snapshot
            .enemies
            .get(&reference.enemy_instance_id)
            .ok_or(EnemyIntentError::EnemyNotFound)?;
        if enemy.enemy_definition_id != reference.enemy_definition_id {
            return Err(EnemyIntentError::StaleReference);
        }
        let actual = detail_bytes(enemy);
        if actual > ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES {
            return Err(EnemyIntentError::DetailTooLarge {
                limit: ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES,
                actual,
            });
        }
        Ok(EnemyIntentEnemy {
            reference: reference.clone(),
            name: enemy.name.clone(),
            hp: enemy.hp.clone(),
            max_hp: enemy.max_hp.clone(),
            block: enemy.block.clone(),
            statuses: enemy.statuses.clone(),
            intent: enemy.intent.clone(),
        })
    }

    /// Returns ordered component references for one currently visible intent.
    pub fn component_references(
        &self,
        reference: &EnemyIntentEnemyReference,
    ) -> Result<Vec<EnemyIntentComponentReference>, EnemyIntentError> {
        let enemy = self
            .snapshot
            .enemies
            .get(&reference.enemy_instance_id)
            .ok_or(EnemyIntentError::EnemyNotFound)?;
        if reference.binding != self.snapshot.binding
            || enemy.enemy_definition_id != reference.enemy_definition_id
        {
            return Err(EnemyIntentError::StaleReference);
        }
        let Some(intent) = enemy.intent.value() else {
            return Err(EnemyIntentError::VisibilityDenied("intent"));
        };
        Ok(intent
            .components
            .iter()
            .map(|component| EnemyIntentComponentReference {
                binding: self.snapshot.binding.clone(),
                enemy_instance_id: enemy.enemy_instance_id.clone(),
                intent_id: intent.linkage.intent_id.clone(),
                component_id: component.component_id.clone(),
            })
            .collect())
    }

    /// Reads one ordered component by its exact snapshot-bound identity.
    pub fn get_component(
        &self,
        reference: &EnemyIntentComponentReference,
    ) -> Result<EnemyIntentComponent, EnemyIntentError> {
        if reference.binding != self.snapshot.binding {
            return Err(EnemyIntentError::StaleReference);
        }
        let enemy = self
            .snapshot
            .enemies
            .get(&reference.enemy_instance_id)
            .ok_or(EnemyIntentError::EnemyNotFound)?;
        let Some(intent) = enemy.intent.value() else {
            return Err(EnemyIntentError::VisibilityDenied("intent"));
        };
        if intent.linkage.intent_id != reference.intent_id {
            return Err(EnemyIntentError::StaleReference);
        }
        intent
            .components
            .iter()
            .find(|component| component.component_id == reference.component_id)
            .cloned()
            .ok_or(EnemyIntentError::ComponentNotFound)
    }

    fn reference_for(&self, enemy: &EnemyIntentEnemyInput) -> EnemyIntentEnemyReference {
        EnemyIntentEnemyReference {
            binding: self.snapshot.binding.clone(),
            enemy_instance_id: enemy.enemy_instance_id.clone(),
            enemy_definition_id: enemy.enemy_definition_id.clone(),
        }
    }
}

fn detail_bytes(enemy: &EnemyIntentEnemyInput) -> usize {
    let mut bytes = enemy.enemy_instance_id.len() + enemy.enemy_definition_id.len();
    if let Some(value) = enemy.name.value() {
        bytes += value.len();
    }
    if let Some(statuses) = enemy.statuses.value() {
        for status in statuses {
            bytes += status.definition_id.len();
            if let Some(value) = status.instance_id.value() {
                bytes += value.len();
            }
            if let Some(value) = status.label.value() {
                bytes += value.len();
            }
        }
    }
    if let Some(intent) = enemy.intent.value() {
        bytes += intent.linkage.intent_id.len();
        for field in [
            &intent.linkage.move_id,
            &intent.linkage.definition_id,
            &intent.linkage.label,
        ] {
            if let Some(value) = field.value() {
                bytes += value.len();
            }
        }
        for component in &intent.components {
            bytes += component.component_id.len();
            if let Some(value) = component.description.value() {
                bytes += value.len();
            }
            if let Some(effects) = component.effects.value() {
                bytes += effects.iter().map(|effect| effect.id.len()).sum::<usize>();
            }
            if let Some(parameters) = component.parameters.value() {
                for parameter in parameters {
                    bytes += parameter.id.len();
                    if let Some(value) = parameter.label.value() {
                        bytes += value.len();
                    }
                    if let super::EnemyIntentParameterValue::Text(value) = &parameter.value {
                        bytes += value.len();
                    }
                }
            }
            bytes += target_bytes(&component.targets);
        }
        bytes += target_bytes(&intent.targets);
    }
    bytes
}

fn target_bytes(field: &EnemyIntentField<super::EnemyIntentTargetInfo>) -> usize {
    field
        .value()
        .and_then(|info| match &info.targets {
            super::EnemyIntentTargets::Visible(targets) => Some(
                targets
                    .iter()
                    .map(|target| {
                        target.target_id.len() + target.label.value().map_or(0, String::len)
                    })
                    .sum(),
            ),
            _ => Some(0),
        })
        .unwrap_or(0)
}
