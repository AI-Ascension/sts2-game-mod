// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use super::{
    ENEMY_INTENT_MAX_COMPONENTS, ENEMY_INTENT_MAX_EFFECT_REFERENCES, ENEMY_INTENT_MAX_ENEMIES,
    ENEMY_INTENT_MAX_KIND_BYTES, ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES, ENEMY_INTENT_MAX_PARAMETERS,
    ENEMY_INTENT_MAX_SNAPSHOT_BYTES, ENEMY_INTENT_MAX_STATUSES, ENEMY_INTENT_PRODUCER_VERSION,
    EnemyIntentAmount, EnemyIntentComponent, EnemyIntentComponentKind, EnemyIntentDamage,
    EnemyIntentEffectReference, EnemyIntentEnemyInput, EnemyIntentError, EnemyIntentField,
    EnemyIntentInput, EnemyIntentLiveBinding, EnemyIntentParameter, EnemyIntentParameterValue,
    EnemyIntentSnapshotInput,
    measure::enemy_bytes,
    model::{validate_identity, validate_kind, validate_text},
    validation_fields::{
        validate_identity_field, validate_target_field, validate_text_field, validate_unit_field,
    },
};

pub(super) fn validate_snapshot(input: &EnemyIntentSnapshotInput) -> Result<(), EnemyIntentError> {
    validate_binding(&input.binding)?;
    if input.enemies.len() > ENEMY_INTENT_MAX_ENEMIES {
        return Err(EnemyIntentError::InvalidInput("enemies"));
    }
    let mut enemy_ids = BTreeSet::new();
    let mut snapshot_bytes = 0usize;
    for enemy in &input.enemies {
        validate_enemy(enemy)?;
        if !enemy_ids.insert(enemy.enemy_instance_id.as_str()) {
            return Err(EnemyIntentError::DuplicateEnemy(
                enemy.enemy_instance_id.clone(),
            ));
        }
        let detail_bytes = enemy_bytes(enemy);
        if detail_bytes > ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES {
            return Err(EnemyIntentError::DetailTooLarge {
                limit: ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES,
                actual: detail_bytes,
            });
        }
        snapshot_bytes = snapshot_bytes
            .checked_add(detail_bytes)
            .ok_or(EnemyIntentError::InvalidInput("snapshot bytes"))?;
        if snapshot_bytes > ENEMY_INTENT_MAX_SNAPSHOT_BYTES {
            return Err(EnemyIntentError::DetailTooLarge {
                limit: ENEMY_INTENT_MAX_SNAPSHOT_BYTES,
                actual: snapshot_bytes,
            });
        }
    }
    Ok(())
}

pub(super) fn validate_binding(binding: &EnemyIntentLiveBinding) -> Result<(), EnemyIntentError> {
    if binding.catalog.producer_version != ENEMY_INTENT_PRODUCER_VERSION {
        return Err(EnemyIntentError::InvalidBinding("producer_version"));
    }
    for (field, value) in [
        (
            "content_manifest.adapter_compatibility",
            binding
                .catalog
                .content_manifest
                .adapter_compatibility
                .as_str(),
        ),
        (
            "content_manifest.content_set_revision",
            binding
                .catalog
                .content_manifest
                .content_set_revision
                .as_str(),
        ),
        (
            "content_manifest.localized_text_revision",
            binding
                .catalog
                .content_manifest
                .localized_text_revision
                .as_str(),
        ),
        (
            "content_manifest.inventory_revision",
            binding.catalog.content_manifest.inventory_revision.as_str(),
        ),
        ("locale", binding.catalog.locale.as_str()),
        ("game_instance_id", binding.game_instance_id.as_str()),
        ("run_id", binding.run_id.as_str()),
        ("combat_id", binding.combat_id.as_str()),
        ("snapshot_id", binding.snapshot_id.as_str()),
    ] {
        validate_identity(value, field).map_err(EnemyIntentError::InvalidBinding)?;
    }
    Ok(())
}

fn validate_enemy(enemy: &EnemyIntentEnemyInput) -> Result<(), EnemyIntentError> {
    validate_identity(&enemy.enemy_instance_id, "enemy_instance_id")
        .map_err(EnemyIntentError::InvalidInput)?;
    validate_identity(&enemy.enemy_definition_id, "enemy_definition_id")
        .map_err(EnemyIntentError::InvalidInput)?;
    if enemy.enemy_instance_id == enemy.enemy_definition_id {
        return Err(EnemyIntentError::AmbiguousIdentity(
            "enemy instance/definition",
        ));
    }
    validate_text_field(&enemy.name, "enemy_name")?;
    if let (Some(hp), Some(max_hp)) = (enemy.hp.value(), enemy.max_hp.value())
        && hp > max_hp
    {
        return Err(EnemyIntentError::InvalidInput("hp exceeds max_hp"));
    }
    validate_statuses(&enemy.statuses)?;
    if let Some(intent) = enemy.intent.value() {
        validate_intent(intent, &enemy.enemy_instance_id, &enemy.enemy_definition_id)?;
    }
    Ok(())
}

fn validate_statuses(
    statuses: &EnemyIntentField<Vec<super::EnemyIntentStatus>>,
) -> Result<(), EnemyIntentError> {
    let Some(statuses) = statuses.value() else {
        return Ok(());
    };
    if statuses.len() > ENEMY_INTENT_MAX_STATUSES {
        return Err(EnemyIntentError::InvalidInput("statuses"));
    }
    let mut ids = BTreeSet::new();
    for status in statuses {
        validate_identity(&status.definition_id, "status_definition_id")
            .map_err(EnemyIntentError::InvalidInput)?;
        if let Some(instance_id) = status.instance_id.value() {
            validate_identity(instance_id, "status_instance_id")
                .map_err(EnemyIntentError::InvalidInput)?;
            if !ids.insert(instance_id.as_str()) {
                return Err(EnemyIntentError::DuplicateStatus(instance_id.clone()));
            }
        }
        validate_text_field(&status.label, "status_label")?;
    }
    Ok(())
}

fn validate_intent(
    intent: &EnemyIntentInput,
    enemy_instance_id: &str,
    enemy_definition_id: &str,
) -> Result<(), EnemyIntentError> {
    validate_linkage(&intent.linkage, enemy_instance_id, enemy_definition_id)?;
    if intent.components.is_empty() || intent.components.len() > ENEMY_INTENT_MAX_COMPONENTS {
        return Err(EnemyIntentError::InvalidInput("components"));
    }
    let mut component_ids = BTreeSet::new();
    for component in &intent.components {
        validate_component(
            component,
            &intent.linkage.intent_id,
            enemy_instance_id,
            &mut component_ids,
        )?;
    }
    validate_target_field(&intent.targets)?;
    Ok(())
}

fn validate_linkage(
    linkage: &super::EnemyIntentLinkage,
    enemy_instance_id: &str,
    enemy_definition_id: &str,
) -> Result<(), EnemyIntentError> {
    validate_identity(&linkage.intent_id, "intent_id").map_err(EnemyIntentError::InvalidLinkage)?;
    if linkage.intent_id == enemy_instance_id || linkage.intent_id == enemy_definition_id {
        return Err(EnemyIntentError::AmbiguousIdentity("intent_id"));
    }
    validate_identity_field(&linkage.move_id, "move_id")?;
    validate_identity_field(&linkage.definition_id, "intent_definition_id")?;
    validate_text_field(&linkage.label, "intent_label")?;
    Ok(())
}

fn validate_component(
    component: &EnemyIntentComponent,
    intent_id: &str,
    enemy_instance_id: &str,
    component_ids: &mut BTreeSet<String>,
) -> Result<(), EnemyIntentError> {
    validate_identity(&component.component_id, "component_id")
        .map_err(EnemyIntentError::InvalidInput)?;
    if component.component_id == intent_id || component.component_id == enemy_instance_id {
        return Err(EnemyIntentError::AmbiguousIdentity("component_id"));
    }
    if !component_ids.insert(component.component_id.clone()) {
        return Err(EnemyIntentError::DuplicateComponent(
            component.component_id.clone(),
        ));
    }
    match &component.kind {
        EnemyIntentComponentKind::Custom(value) | EnemyIntentComponentKind::Unsupported(value) => {
            if value.is_empty() || value.len() > ENEMY_INTENT_MAX_KIND_BYTES {
                return Err(EnemyIntentError::InvalidInput("component_kind"));
            }
            validate_kind(value, "component_kind").map_err(EnemyIntentError::InvalidInput)?;
        }
        _ => {}
    }
    validate_text_field(&component.description, "component_description")?;
    validate_damage_field(&component.damage)?;
    validate_amount_field(&component.amount)?;
    validate_effects(&component.effects)?;
    validate_parameters(&component.parameters)?;
    validate_target_field(&component.targets)?;
    Ok(())
}

fn validate_damage_field(
    field: &EnemyIntentField<EnemyIntentDamage>,
) -> Result<(), EnemyIntentError> {
    let Some(damage) = field.value() else {
        return Ok(());
    };
    validate_amount_field(&damage.per_hit)?;
    validate_amount_field(&damage.total)?;
    if let Some(hits) = damage.hits.value()
        && *hits == 0
    {
        return Err(EnemyIntentError::InvalidInput("damage hits"));
    }
    Ok(())
}

fn validate_amount_field(
    field: &EnemyIntentField<EnemyIntentAmount>,
) -> Result<(), EnemyIntentError> {
    let Some(amount) = field.value() else {
        return Ok(());
    };
    validate_identity(amount.unit.as_str(), "amount_unit").map_err(EnemyIntentError::InvalidInput)
}

fn validate_effects(
    field: &EnemyIntentField<Vec<EnemyIntentEffectReference>>,
) -> Result<(), EnemyIntentError> {
    let Some(effects) = field.value() else {
        return Ok(());
    };
    if effects.len() > ENEMY_INTENT_MAX_EFFECT_REFERENCES {
        return Err(EnemyIntentError::InvalidInput("effects"));
    }
    let mut ids = BTreeSet::new();
    for effect in effects {
        validate_identity(&effect.id, "effect_reference_id")
            .map_err(EnemyIntentError::InvalidInput)?;
        if !ids.insert(effect.id.as_str()) {
            return Err(EnemyIntentError::DuplicateTarget(effect.id.clone()));
        }
    }
    Ok(())
}

fn validate_parameters(
    field: &EnemyIntentField<Vec<EnemyIntentParameter>>,
) -> Result<(), EnemyIntentError> {
    let Some(parameters) = field.value() else {
        return Ok(());
    };
    if parameters.len() > ENEMY_INTENT_MAX_PARAMETERS {
        return Err(EnemyIntentError::InvalidInput("parameters"));
    }
    let mut ids = BTreeSet::new();
    for parameter in parameters {
        validate_parameter(parameter)?;
        if !ids.insert(parameter.id.as_str()) {
            return Err(EnemyIntentError::DuplicateParameter(parameter.id.clone()));
        }
    }
    Ok(())
}

fn validate_parameter(parameter: &EnemyIntentParameter) -> Result<(), EnemyIntentError> {
    validate_identity(&parameter.id, "parameter_id").map_err(EnemyIntentError::InvalidInput)?;
    validate_text_field(&parameter.label, "parameter_label")?;
    if let EnemyIntentParameterValue::Text(value) = &parameter.value {
        validate_text(value, "parameter_text").map_err(EnemyIntentError::InvalidInput)?;
    }
    validate_unit_field(&parameter.unit, "parameter_unit")?;
    Ok(())
}
