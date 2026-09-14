// SPDX-License-Identifier: MIT

use super::{
    EnemyIntentComponentKind, EnemyIntentEnemyInput, EnemyIntentField, EnemyIntentInput,
    EnemyIntentParameterValue, EnemyIntentTargetInfo, EnemyIntentTargets,
};

pub(super) fn enemy_bytes(enemy: &EnemyIntentEnemyInput) -> usize {
    let mut bytes = enemy.enemy_instance_id.len() + enemy.enemy_definition_id.len();
    add_field_text(&mut bytes, &enemy.name);
    if let Some(statuses) = enemy.statuses.value() {
        for status in statuses {
            bytes += status.definition_id.len();
            add_field_text(&mut bytes, &status.instance_id);
            add_field_text(&mut bytes, &status.label);
        }
    }
    if let Some(intent) = enemy.intent.value() {
        bytes += intent_bytes(intent);
    }
    bytes
}

fn intent_bytes(intent: &EnemyIntentInput) -> usize {
    let mut bytes = intent.linkage.intent_id.len();
    add_field_text(&mut bytes, &intent.linkage.move_id);
    add_field_text(&mut bytes, &intent.linkage.definition_id);
    add_field_text(&mut bytes, &intent.linkage.label);
    for component in &intent.components {
        bytes += component.component_id.len();
        if let EnemyIntentComponentKind::Custom(value)
        | EnemyIntentComponentKind::Unsupported(value) = &component.kind
        {
            bytes += value.len();
        }
        add_field_text(&mut bytes, &component.description);
        if let Some(amount) = component.amount.value() {
            bytes += amount.unit.as_str().len();
        }
        if let Some(damage) = component.damage.value() {
            if let Some(per_hit) = damage.per_hit.value() {
                bytes += per_hit.unit.as_str().len();
            }
            if let Some(total) = damage.total.value() {
                bytes += total.unit.as_str().len();
            }
        }
        if let Some(effects) = component.effects.value() {
            bytes += effects.iter().map(|effect| effect.id.len()).sum::<usize>();
        }
        if let Some(parameters) = component.parameters.value() {
            for parameter in parameters {
                bytes += parameter.id.len();
                add_field_text(&mut bytes, &parameter.label);
                if let Some(unit) = parameter.unit.value() {
                    bytes += unit.as_str().len();
                }
                if let EnemyIntentParameterValue::Text(value) = &parameter.value {
                    bytes += value.len();
                }
            }
        }
        add_target_bytes(&mut bytes, &component.targets);
    }
    add_target_bytes(&mut bytes, &intent.targets);
    bytes
}

fn add_target_bytes(bytes: &mut usize, field: &EnemyIntentField<EnemyIntentTargetInfo>) {
    if let Some(info) = field.value()
        && let EnemyIntentTargets::Visible(targets) = &info.targets
    {
        for target in targets {
            *bytes += target.target_id.len();
            add_field_text(bytes, &target.label);
        }
    }
}

fn add_field_text(bytes: &mut usize, field: &EnemyIntentField<String>) {
    if let Some(value) = field.value() {
        *bytes += value.len();
    }
}
