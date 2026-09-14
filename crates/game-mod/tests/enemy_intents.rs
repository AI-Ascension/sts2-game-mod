// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/enemy_intents.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ENEMY_INTENT_MAX_COMPONENTS, ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES, ENEMY_INTENT_MAX_TEXT_BYTES,
    EnemyIntentCapability, EnemyIntentComponentKind, EnemyIntentError, EnemyIntentField,
    EnemyIntentLiveReader, EnemyIntentLiveSnapshot, EnemyIntentSource, EnemyIntentStatus,
    EnemyIntentTargets, EnemyIntentUnavailableReason, EnemyIntentVisibilityScope,
    FixtureEnemyIntentSource, UnavailableEnemyIntentSource,
};

#[test]
fn compound_intent_preserves_ordered_effects_damage_and_public_coop_targets() {
    let snapshot = EnemyIntentLiveSnapshot::from_input(snapshot(4)).expect("snapshot");
    let reader = EnemyIntentLiveReader::new(snapshot).expect("reader");
    let reference = reader.references().into_iter().next().expect("enemy");
    let detail = reader.get(&reference).expect("detail");
    assert_eq!(detail.hp.value(), Some(&20));
    assert_eq!(detail.block.value(), Some(&4));
    let intent = detail.intent.value().expect("intent");
    assert_eq!(intent.linkage.intent_id, "intent:slime:1");
    assert_eq!(intent.components.len(), 2);
    assert!(matches!(
        intent.components[0].kind,
        EnemyIntentComponentKind::Attack
    ));
    let damage = intent.components[0].damage.value().expect("damage");
    assert_eq!(damage.per_hit.value().expect("per hit").value, 7);
    assert_eq!(damage.hits.value(), Some(&2));
    assert_eq!(damage.total.value().expect("total").value, 14);
    assert_eq!(
        intent.targets.value().expect("aggregate targets").targets,
        EnemyIntentTargets::Visible(vec![
            target("player:local", sts2_game_mod::EnemyIntentTargetKind::Player),
            target("player:ally", sts2_game_mod::EnemyIntentTargetKind::Ally),
        ])
    );
    let component_refs = reader
        .component_references(&reference)
        .expect("component refs");
    assert_eq!(
        component_refs
            .iter()
            .map(|reference| reference.component_id.as_str())
            .collect::<Vec<_>>(),
        ["component:attack", "component:debuff"]
    );
    let debuff = reader
        .get_component(&component_refs[1])
        .expect("debuff component");
    assert!(matches!(debuff.kind, EnemyIntentComponentKind::Debuff));
    assert_eq!(
        debuff.effects.value().expect("effects")[0].id,
        "status:vulnerable"
    );
}

#[test]
fn identity_and_epoch_fences_expire_enemy_and_component_references() {
    let first = EnemyIntentLiveSnapshot::from_input(snapshot(1)).expect("snapshot");
    let mut reader = EnemyIntentLiveReader::new(first).expect("reader");
    let old_enemy = reader.references().into_iter().next().expect("enemy");
    let old_component = reader
        .component_references(&old_enemy)
        .expect("components")
        .into_iter()
        .next()
        .expect("component");

    reader
        .replace_snapshot(EnemyIntentLiveSnapshot::from_input(snapshot(2)).expect("next"))
        .expect("replace");
    assert_eq!(
        reader.get(&old_enemy),
        Err(EnemyIntentError::StaleReference)
    );
    assert_eq!(
        reader.get_component(&old_component),
        Err(EnemyIntentError::StaleReference)
    );
    assert_eq!(
        reader.replace_snapshot(EnemyIntentLiveSnapshot::from_input(snapshot(2)).expect("same")),
        Err(EnemyIntentError::NonMonotonicEpoch {
            current: 2,
            supplied: 2
        })
    );

    let mut wrong_run = snapshot(3);
    wrong_run.binding.run_id = "run:other".to_owned();
    assert_eq!(
        reader.replace_snapshot(EnemyIntentLiveSnapshot::from_input(wrong_run).expect("wrong run")),
        Err(EnemyIntentError::RunMismatch)
    );
    let mut wrong_combat = snapshot(3);
    wrong_combat.binding.combat_id = "combat:other".to_owned();
    assert_eq!(
        reader.replace_snapshot(
            EnemyIntentLiveSnapshot::from_input(wrong_combat).expect("wrong combat")
        ),
        Err(EnemyIntentError::CombatMismatch)
    );
}

#[test]
fn hidden_unknown_and_not_observed_targets_remain_explicit_without_fake_ids() {
    let mut input = snapshot(8);
    let intent = input.enemies[0].intent.value().expect("intent").clone();
    let mut hidden = intent;
    hidden.targets = EnemyIntentField::Available(sts2_game_mod::EnemyIntentTargetInfo {
        domain: sts2_game_mod::EnemyIntentTargetDomain::Players,
        targets: EnemyIntentTargets::Hidden,
    });
    hidden.components[0].targets =
        EnemyIntentField::Available(sts2_game_mod::EnemyIntentTargetInfo {
            domain: sts2_game_mod::EnemyIntentTargetDomain::Players,
            targets: EnemyIntentTargets::Unknown,
        });
    input.enemies[0].intent = EnemyIntentField::Available(hidden);
    input.enemies[0].statuses = EnemyIntentField::NotObserved;

    let reader =
        EnemyIntentLiveReader::new(EnemyIntentLiveSnapshot::from_input(input).expect("snapshot"))
            .expect("reader");
    let detail = reader
        .get(&reader.references().into_iter().next().expect("enemy"))
        .expect("detail");
    assert_eq!(
        detail.statuses.status(),
        sts2_game_mod::EnemyIntentFieldStatus::NotObserved
    );
    let intent = detail.intent.value().expect("intent");
    assert!(matches!(
        intent.targets.value().expect("targets").targets,
        EnemyIntentTargets::Hidden
    ));
    assert!(matches!(
        intent.components[0]
            .targets
            .value()
            .expect("targets")
            .targets,
        EnemyIntentTargets::Unknown
    ));
}

#[test]
fn validation_rejects_oversized_and_ambiguous_input_without_clamping() {
    let mut oversized = snapshot(9);
    let large = "x".repeat(ENEMY_INTENT_MAX_TEXT_BYTES);
    for index in 0..5 {
        let statuses = oversized.enemies[0].statuses.value().expect("statuses");
        let mut statuses = statuses.clone();
        statuses.push(EnemyIntentStatus {
            instance_id: EnemyIntentField::Available(format!("status:large:{index}")),
            definition_id: format!("status:large:{index}"),
            label: EnemyIntentField::Available(large.clone()),
            amount: EnemyIntentField::Unknown,
        });
        oversized.enemies[0].statuses = EnemyIntentField::Available(statuses);
    }
    let oversized_result = EnemyIntentLiveSnapshot::from_input(oversized);
    dbg!(&oversized_result);
    assert!(matches!(
        oversized_result,
        Err(EnemyIntentError::DetailTooLarge {
            limit: ENEMY_INTENT_MAX_LIVE_DETAIL_BYTES,
            ..
        })
    ));

    let mut duplicate = snapshot(10);
    let mut duplicate_intent = duplicate.enemies[0].intent.value().expect("intent").clone();
    duplicate_intent.components[1].component_id =
        duplicate_intent.components[0].component_id.clone();
    duplicate.enemies[0].intent = EnemyIntentField::Available(duplicate_intent);
    assert_eq!(
        EnemyIntentLiveSnapshot::from_input(duplicate),
        Err(EnemyIntentError::DuplicateComponent(
            "component:attack".to_owned()
        ))
    );

    let mut too_many = snapshot(11);
    let component = too_many.enemies[0]
        .intent
        .value()
        .expect("intent")
        .components[0]
        .clone();
    let intent = too_many.enemies[0].intent.value().expect("intent").clone();
    let mut components = vec![component; ENEMY_INTENT_MAX_COMPONENTS + 1];
    for (index, component) in components.iter_mut().enumerate() {
        component.component_id = format!("component:overflow:{index}");
    }
    too_many.enemies[0].intent = EnemyIntentField::Available(sts2_game_mod::EnemyIntentInput {
        components,
        ..intent
    });
    assert_eq!(
        EnemyIntentLiveSnapshot::from_input(too_many),
        Err(EnemyIntentError::InvalidInput("components"))
    );
}

#[test]
fn source_capability_identity_and_read_errors_are_typed() {
    let source = FixtureEnemyIntentSource::new(snapshot(3));
    assert_eq!(
        source.capability(),
        EnemyIntentCapability::SyntheticFixtureOnly
    );
    let reader = EnemyIntentLiveReader::from_source(&source, &binding(3)).expect("source reader");
    assert_eq!(reader.scope(), EnemyIntentVisibilityScope::Public);
    assert!(matches!(
        EnemyIntentLiveReader::from_source(&source, &binding(4)),
        Err(EnemyIntentError::SourceStale)
    ));

    let unavailable = UnavailableEnemyIntentSource;
    assert!(matches!(
        EnemyIntentLiveReader::from_source(&unavailable, &binding(3)),
        Err(EnemyIntentError::Unavailable(
            EnemyIntentUnavailableReason::ExactHostEvidenceRequired
        ))
    ));
}
