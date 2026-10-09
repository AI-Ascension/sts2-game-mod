// SPDX-License-Identifier: MIT

use super::fixture::{EnemySource, boss, definition_reference, manifest, move_reference, snapshot};
use sts2_game_mod::{
    EnemyCatalogError, EnemyCatalogProducer, EnemyMoveEffectKind, EnemyNumericValue,
    EnemyProbability, EnemySemanticReferenceKind, EnemyUnavailableReason, EnemyVisibilityScope,
};

#[test]
fn unknown_transition_and_named_boss_facts_survive_catalog_lookup() {
    let content = manifest(&["enemy:boss"], &["encounter:boss"], &["power_status:weak"]);
    let mut input = boss("enemy:boss");
    input.transitions[0].probability =
        EnemyProbability::Unavailable(EnemyUnavailableReason::Unknown);
    let source = EnemySource {
        snapshot: Ok(snapshot(&content, vec![input])),
    };
    let catalog = EnemyCatalogProducer::new()
        .produce(&content, &source)
        .expect("synthetic boss catalog");
    let reference = definition_reference(&catalog, "enemy:boss");
    let reader = catalog.reader();
    let definition = reader
        .get(&reference, EnemyVisibilityScope::Reference)
        .expect("exact synthetic definition");

    assert_eq!(
        definition
            .phases
            .iter()
            .map(|phase| (phase.phase_id.as_str(), phase.order))
            .collect::<Vec<_>>(),
        vec![("phase:one", 0), ("phase:two", 1)]
    );
    assert_eq!(
        definition.phases[0].move_ids,
        vec!["move:slam", "move:summon"]
    );
    assert_eq!(definition.phases[1].move_ids, vec!["move:enrage"]);

    let transition = &definition.transitions[0];
    assert_eq!(transition.transition_id, "transition:enrage");
    assert_eq!(transition.from_phase.as_deref(), Some("phase:one"));
    assert_eq!(transition.to_phase, "phase:two");
    assert_eq!(transition.condition.condition_id, "condition:half-hp");
    assert_eq!(
        transition.probability,
        EnemyProbability::Unavailable(EnemyUnavailableReason::Unknown)
    );

    let summon = reader
        .get_move(
            &move_reference(&catalog, "enemy:boss", "move:summon"),
            EnemyVisibilityScope::Reference,
        )
        .expect("exact synthetic summon");
    assert_eq!(summon.phase_ids, vec!["phase:one"]);
    assert_eq!(summon.effects.len(), 1);
    assert_eq!(summon.effects[0].effect_id, "effect:summon");
    assert_eq!(summon.effects[0].kind, EnemyMoveEffectKind::Summon);

    let slam = reader
        .get_move(
            &move_reference(&catalog, "enemy:boss", "move:slam"),
            EnemyVisibilityScope::Reference,
        )
        .expect("exact synthetic compound move");
    assert_eq!(slam.effects.len(), 2);
    assert_eq!(slam.effects[0].kind, EnemyMoveEffectKind::Attack);
    assert_eq!(slam.effects[1].kind, EnemyMoveEffectKind::ApplyStatus);
    assert_eq!(
        slam.effects[1].references[0].kind,
        EnemySemanticReferenceKind::Status
    );
    assert_eq!(slam.effects[1].references[0].id, "power_status:weak");

    let profiles = definition.stats.scaled.value().expect("scaled profiles");
    assert_eq!(profiles.len(), 2);
    for (profile, (profile_id, hp)) in profiles.iter().zip([("normal", 120), ("hard", 160)]) {
        assert_eq!(profile.profile_id, profile_id);
        assert_eq!(profile.mode.as_deref(), Some(profile_id));
        assert_eq!(profile.difficulty.as_deref(), Some(profile_id));
        assert_eq!(profile.stats.len(), 1);
        assert_eq!(profile.stats[0].stat_id, "hp");
        assert_eq!(profile.stats[0].unit.as_deref(), Some("hp"));
        assert_eq!(profile.stats[0].value, EnemyNumericValue::Fixed(hp));
    }
}

#[test]
fn manifest_enemy_without_source_record_rejects_the_catalog() {
    let content = manifest(&["enemy:omitted"], &[], &[]);
    let source = EnemySource {
        snapshot: Ok(snapshot(&content, Vec::new())),
    };

    assert_eq!(
        EnemyCatalogProducer::new().produce(&content, &source),
        Err(EnemyCatalogError::FamilyCountMismatch)
    );
}
