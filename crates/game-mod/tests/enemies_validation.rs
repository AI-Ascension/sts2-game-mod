// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/enemies.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, ENEMY_MAX_DEFINITION_BYTES, ENEMY_MAX_DEFINITIONS, ENEMY_MAX_EFFECTS,
    ENEMY_MAX_IDENTITY_BYTES, ENEMY_MAX_MOVES, ENEMY_MAX_TAGS, ENEMY_MAX_TEXT_BYTES,
    EnemyCatalogError, EnemyCatalogProducer, EnemyCooldownRule, EnemyDefinitionInput,
    EnemyEvidence, EnemyField, EnemyKind, EnemyMoveDefinitionInput, EnemyMoveEffect,
    EnemyMoveEffectKind, EnemyNumericValue, EnemyOriginVariant, EnemyPhaseDefinition,
    EnemyProbability, EnemyRepetitionRule, EnemySemanticReferenceKind, EnemyTag, EnemyTargetDomain,
    EnemyText, EnemyUnavailableReason, EnemyVisibility,
};

fn visible_enemy(enemy_id: &str) -> EnemyDefinitionInput {
    enemy(
        enemy_id,
        EnemyKind::Normal,
        EnemyVisibility::Visible,
        ContentUnlockState::Unlocked,
    )
}

fn produce(
    content: &sts2_game_mod::ContentManifest,
    definition: EnemyDefinitionInput,
) -> Result<sts2_game_mod::EnemyCatalog, EnemyCatalogError> {
    EnemyCatalogProducer::new().produce(
        content,
        &EnemySource {
            snapshot: Ok(snapshot(content, vec![definition])),
        },
    )
}

#[test]
fn duplicate_enemy_move_phase_effect_and_reference_identities_are_rejected() {
    let content = manifest(&["enemy:dup"], &[], &[]);
    let duplicate_enemy = produce(&content, visible_enemy("enemy:dup"));
    assert!(duplicate_enemy.is_ok());

    let mut two = snapshot(&content, vec![visible_enemy("enemy:dup")]);
    two.definitions.push(visible_enemy("enemy:dup"));
    assert_eq!(
        EnemyCatalogProducer::new().produce(&content, &EnemySource { snapshot: Ok(two) }),
        Err(EnemyCatalogError::DuplicateDefinition(
            "enemy:dup".to_owned()
        ))
    );

    let boss_content = manifest(&["enemy:boss"], &["encounter:boss"], &["power_status:weak"]);

    let mut duplicate_move = boss("enemy:boss");
    duplicate_move.moves[1].move_id = "move:slam".to_owned();
    assert_eq!(
        produce(&boss_content, duplicate_move),
        Err(EnemyCatalogError::InvalidInput("duplicate_move"))
    );

    let mut duplicate_phase = boss("enemy:boss");
    duplicate_phase.phases[1].phase_id = "phase:one".to_owned();
    assert_eq!(
        produce(&boss_content, duplicate_phase),
        Err(EnemyCatalogError::InvalidInput("duplicate_phase"))
    );

    let mut duplicate_effect = boss("enemy:boss");
    duplicate_effect.moves[0].effects[1].effect_id = "effect:damage".to_owned();
    assert_eq!(
        produce(&boss_content, duplicate_effect),
        Err(EnemyCatalogError::InvalidInput("duplicate_effect"))
    );

    let mut duplicate_reference = boss("enemy:boss");
    duplicate_reference.references.push(reference(
        EnemySemanticReferenceKind::Encounter,
        "encounter:boss",
    ));
    assert_eq!(
        produce(&boss_content, duplicate_reference),
        Err(EnemyCatalogError::InvalidInput("duplicate_reference"))
    );
}

#[test]
fn malformed_identity_text_limits_and_probabilities_are_rejected() {
    let content = manifest(&["enemy:ok"], &[], &[]);

    let mut bad_identity = visible_enemy("enemy:ok");
    bad_identity.enemy_id = "enemy:bad\u{1}".to_owned();
    assert_eq!(
        produce(&content, bad_identity),
        Err(EnemyCatalogError::InvalidInput("enemy_id"))
    );

    let mut empty_name = visible_enemy("enemy:ok");
    empty_name.name = EnemyText::Available(String::new());
    assert_eq!(
        produce(&content, empty_name),
        Err(EnemyCatalogError::InvalidInput("name"))
    );

    let mut oversized_text = visible_enemy("enemy:ok");
    oversized_text.name = EnemyText::Available("x".repeat(ENEMY_MAX_TEXT_BYTES + 1));
    assert_eq!(
        produce(&content, oversized_text),
        Err(EnemyCatalogError::InvalidInput("name"))
    );

    let mut too_many_tags = visible_enemy("enemy:ok");
    too_many_tags.tags = (0..ENEMY_MAX_TAGS + 1)
        .map(|index| EnemyTag {
            tag_id: format!("tag:{index}"),
            label: text("tag"),
        })
        .collect();
    assert_eq!(
        produce(&content, too_many_tags),
        Err(EnemyCatalogError::InvalidInput("tags"))
    );

    let boss_content = manifest(&["enemy:boss"], &["encounter:boss"], &["power_status:weak"]);

    let mut too_many_moves = boss("enemy:boss");
    let template = too_many_moves.moves[0].clone();
    too_many_moves.moves = (0..ENEMY_MAX_MOVES + 1)
        .map(|index| EnemyMoveDefinitionInput {
            move_id: format!("move:{index}"),
            ..template.clone()
        })
        .collect();
    assert_eq!(
        produce(&boss_content, too_many_moves),
        Err(EnemyCatalogError::InvalidInput("moves"))
    );

    let mut too_many_effects = boss("enemy:boss");
    too_many_effects.moves[0].effects = (0..ENEMY_MAX_EFFECTS + 1)
        .map(|index| EnemyMoveEffect {
            effect_id: format!("effect:{index}"),
            kind: EnemyMoveEffectKind::Attack,
            description: text("Deal damage."),
            targeting: targeting(EnemyTargetDomain::AnyPlayer, 1),
            parameters: Vec::new(),
            references: Vec::new(),
        })
        .collect();
    assert_eq!(
        produce(&boss_content, too_many_effects),
        Err(EnemyCatalogError::InvalidInput("effects"))
    );

    let mut bad_probability = boss("enemy:boss");
    bad_probability.moves[0].probability = EnemyProbability::Exact {
        numerator: 1,
        denominator: 0,
        evidence: EnemyEvidence::SourceDerived,
    };
    assert_eq!(
        produce(&boss_content, bad_probability),
        Err(EnemyCatalogError::InvalidInput("probability"))
    );

    let mut bad_target = boss("enemy:boss");
    bad_target.moves[0].targeting.count = EnemyNumericValue::Fixed(0);
    assert_eq!(
        produce(&boss_content, bad_target),
        Err(EnemyCatalogError::InvalidInput("target_count"))
    );
}

#[test]
fn too_many_definitions_are_rejected_before_validation() {
    let content = manifest(&[], &[], &[]);
    let mut oversized = snapshot(&content, Vec::new());
    oversized.definitions = (0..ENEMY_MAX_DEFINITIONS + 1)
        .map(|index| visible_enemy(&format!("enemy:{index}")))
        .collect();
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(oversized),
            }
        ),
        Err(EnemyCatalogError::InvalidInput("definitions"))
    );
}

fn bulk_definition(kind_len: usize) -> EnemyDefinitionInput {
    let mut moves = Vec::new();
    for move_index in 0..ENEMY_MAX_MOVES {
        let effects = (0..ENEMY_MAX_EFFECTS)
            .map(|effect_index| EnemyMoveEffect {
                effect_id: format!("effect:{move_index}:{effect_index}"),
                kind: EnemyMoveEffectKind::Custom("k".repeat(kind_len)),
                description: text("custom"),
                targeting: targeting(EnemyTargetDomain::AnyPlayer, 1),
                parameters: Vec::new(),
                references: Vec::new(),
            })
            .collect();
        moves.push(EnemyMoveDefinitionInput {
            move_id: format!("move:{move_index}"),
            name: text("Bulk"),
            description: text("Bulk move."),
            effects,
            targeting: targeting(EnemyTargetDomain::AnyPlayer, 1),
            phase_ids: vec!["phase:only".to_owned()],
            conditions: Vec::new(),
            cooldown: EnemyCooldownRule::None,
            repetition: EnemyRepetitionRule::Allow,
            probability: EnemyProbability::Unavailable(EnemyUnavailableReason::NotApplicable),
            references: Vec::new(),
            visibility: EnemyVisibility::Visible,
        });
    }
    let mut definition = visible_enemy("enemy:bulk");
    definition.phases = vec![EnemyPhaseDefinition {
        phase_id: "phase:only".to_owned(),
        name: text("Only"),
        description: text("Only phase."),
        order: 0,
        move_ids: (0..ENEMY_MAX_MOVES)
            .map(|index| format!("move:{index}"))
            .collect(),
        entry_condition: EnemyField::Unavailable(EnemyUnavailableReason::NotApplicable),
    }];
    definition.moves = moves;
    definition
}

fn bulk_actual(kind_len: usize) -> usize {
    let content = manifest(&["enemy:bulk"], &[], &[]);
    match produce(&content, bulk_definition(kind_len)) {
        Err(EnemyCatalogError::DefinitionTooLarge { actual, .. }) => actual,
        other => {
            assert!(matches!(
                other,
                Err(EnemyCatalogError::DefinitionTooLarge { .. })
            ));
            0
        }
    }
}

#[test]
fn definition_byte_limit_counts_nested_custom_effect_kinds() {
    let long = bulk_actual(ENEMY_MAX_IDENTITY_BYTES);
    let short = bulk_actual(ENEMY_MAX_IDENTITY_BYTES - 6);
    assert!(long > ENEMY_MAX_DEFINITION_BYTES);
    assert_eq!(
        long - short,
        ENEMY_MAX_MOVES * ENEMY_MAX_EFFECTS * 6,
        "every nested custom effect-kind byte must count toward the definition bound"
    );
}

#[test]
fn dangling_phase_and_variant_move_references_are_rejected() {
    let content = manifest(&["enemy:ok"], &[], &[]);

    let mut dangling_phase = visible_enemy("enemy:ok");
    dangling_phase.phases[0].move_ids = vec!["move:missing".to_owned()];
    assert_eq!(
        produce(&content, dangling_phase),
        Err(EnemyCatalogError::UnknownMoveReference {
            enemy_id: "enemy:ok".to_owned(),
            move_id: "move:missing".to_owned(),
        })
    );

    let mut dangling_variant = visible_enemy("enemy:ok");
    dangling_variant.origin_variants.push(EnemyOriginVariant {
        variant_id: "variant:one".to_owned(),
        label: text("Variant"),
        origin: origin(),
        stats: EnemyField::Unavailable(EnemyUnavailableReason::NotObserved),
        move_ids: EnemyField::Available(vec!["move:missing".to_owned()]),
    });
    assert_eq!(
        produce(&content, dangling_variant),
        Err(EnemyCatalogError::UnknownMoveReference {
            enemy_id: "enemy:ok".to_owned(),
            move_id: "move:missing".to_owned(),
        })
    );
}

#[test]
fn oversized_definition_reports_limit_and_actual() {
    let content = manifest(&["enemy:bulk"], &[], &[]);
    let result = produce(&content, bulk_definition(ENEMY_MAX_IDENTITY_BYTES));
    match result {
        Err(EnemyCatalogError::DefinitionTooLarge { limit, actual }) => {
            assert_eq!(limit, ENEMY_MAX_DEFINITION_BYTES);
            assert!(actual > limit);
        }
        other => {
            assert!(matches!(
                other,
                Err(EnemyCatalogError::DefinitionTooLarge { .. })
            ));
        }
    }
}
