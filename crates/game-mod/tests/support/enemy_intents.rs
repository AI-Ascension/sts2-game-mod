// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCursorBinding, ENEMY_INTENT_PRODUCER_VERSION, EnemyIntentAmount,
    EnemyIntentCatalogBinding, EnemyIntentComponent, EnemyIntentComponentKind, EnemyIntentDamage,
    EnemyIntentEffectReference, EnemyIntentEffectReferenceKind, EnemyIntentEnemyInput,
    EnemyIntentField, EnemyIntentInput, EnemyIntentLinkage, EnemyIntentLiveBinding,
    EnemyIntentParameter, EnemyIntentParameterValue, EnemyIntentSnapshotInput, EnemyIntentStatus,
    EnemyIntentTargetDomain, EnemyIntentTargetInfo, EnemyIntentTargetKind,
    EnemyIntentTargetReference, EnemyIntentTargets, EnemyIntentUnit,
};

pub fn content_manifest() -> ContentCursorBinding {
    ContentCursorBinding {
        catalog_generation: 7,
        adapter_compatibility: "adapter:fixture".to_owned(),
        content_set_revision: "content:fixture".to_owned(),
        localized_text_revision: "text:fixture:en-US".to_owned(),
        inventory_revision: "inventory:fixture".to_owned(),
    }
}

pub fn binding(epoch: u64) -> EnemyIntentLiveBinding {
    EnemyIntentLiveBinding {
        catalog: EnemyIntentCatalogBinding {
            content_manifest: content_manifest(),
            locale: "en-US".to_owned(),
            producer_version: ENEMY_INTENT_PRODUCER_VERSION.to_owned(),
        },
        game_instance_id: "game:fixture".to_owned(),
        run_id: "run:fixture".to_owned(),
        combat_id: "combat:fixture".to_owned(),
        snapshot_id: format!("snapshot:{epoch}"),
        epoch,
    }
}

pub fn unit(value: &str) -> EnemyIntentUnit {
    EnemyIntentUnit::new(value).expect("unit")
}

pub fn amount(value: u32, unit_name: &str) -> EnemyIntentAmount {
    EnemyIntentAmount {
        value,
        unit: unit(unit_name),
    }
}

pub fn target(target_id: &str, kind: EnemyIntentTargetKind) -> EnemyIntentTargetReference {
    EnemyIntentTargetReference {
        target_id: target_id.to_owned(),
        kind,
        label: EnemyIntentField::Available(target_id.to_owned()),
    }
}

pub fn players() -> EnemyIntentTargetInfo {
    EnemyIntentTargetInfo {
        domain: EnemyIntentTargetDomain::Players,
        targets: EnemyIntentTargets::Visible(vec![
            target("player:local", EnemyIntentTargetKind::Player),
            target("player:ally", EnemyIntentTargetKind::Ally),
        ]),
    }
}

pub fn attack_component() -> EnemyIntentComponent {
    EnemyIntentComponent {
        component_id: "component:attack".to_owned(),
        kind: EnemyIntentComponentKind::Attack,
        description: EnemyIntentField::Available("A visible multi-hit attack".to_owned()),
        damage: EnemyIntentField::Available(EnemyIntentDamage {
            per_hit: EnemyIntentField::Available(amount(7, "damage")),
            hits: EnemyIntentField::Available(2),
            total: EnemyIntentField::Available(amount(14, "damage")),
        }),
        amount: EnemyIntentField::NotApplicable,
        effects: EnemyIntentField::NotApplicable,
        parameters: EnemyIntentField::Available(vec![EnemyIntentParameter {
            id: "parameter:hit-count".to_owned(),
            label: EnemyIntentField::Available("Hits".to_owned()),
            value: EnemyIntentParameterValue::Integer(2),
            unit: EnemyIntentField::Available(unit("count")),
        }]),
        targets: EnemyIntentField::Available(players()),
    }
}

pub fn debuff_component() -> EnemyIntentComponent {
    EnemyIntentComponent {
        component_id: "component:debuff".to_owned(),
        kind: EnemyIntentComponentKind::Debuff,
        description: EnemyIntentField::Available("Applies Vulnerable".to_owned()),
        damage: EnemyIntentField::NotApplicable,
        amount: EnemyIntentField::NotApplicable,
        effects: EnemyIntentField::Available(vec![EnemyIntentEffectReference {
            id: "status:vulnerable".to_owned(),
            kind: EnemyIntentEffectReferenceKind::Status,
            amount: EnemyIntentField::Available(2),
        }]),
        parameters: EnemyIntentField::NotApplicable,
        targets: EnemyIntentField::Available(players()),
    }
}

pub fn intent() -> EnemyIntentInput {
    EnemyIntentInput {
        linkage: EnemyIntentLinkage {
            intent_id: "intent:slime:1".to_owned(),
            move_id: EnemyIntentField::Available("move:slime:slam".to_owned()),
            definition_id: EnemyIntentField::Available("enemy-move:slime:slam".to_owned()),
            label: EnemyIntentField::Available("Slam".to_owned()),
        },
        components: vec![attack_component(), debuff_component()],
        targets: EnemyIntentField::Available(players()),
    }
}

pub fn enemy() -> EnemyIntentEnemyInput {
    EnemyIntentEnemyInput {
        enemy_instance_id: "enemy:instance:1".to_owned(),
        enemy_definition_id: "enemy:def:slime".to_owned(),
        name: EnemyIntentField::Available("Fixture Slime".to_owned()),
        hp: EnemyIntentField::Available(20),
        max_hp: EnemyIntentField::Available(30),
        block: EnemyIntentField::Available(4),
        statuses: EnemyIntentField::Available(vec![EnemyIntentStatus {
            instance_id: EnemyIntentField::Available("status:instance:1".to_owned()),
            definition_id: "status:poison".to_owned(),
            label: EnemyIntentField::Available("Poison".to_owned()),
            amount: EnemyIntentField::Available(2),
        }]),
        intent: EnemyIntentField::Available(intent()),
    }
}

pub fn snapshot(epoch: u64) -> EnemyIntentSnapshotInput {
    EnemyIntentSnapshotInput {
        binding: binding(epoch),
        enemies: vec![enemy()],
    }
}
