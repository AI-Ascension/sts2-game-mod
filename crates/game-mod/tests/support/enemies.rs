// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    ContentUnlockState, ENEMY_ENTITY_KIND, ENEMY_PRODUCER_VERSION, EnemyBehaviorTransition,
    EnemyCatalog, EnemyCatalogProducer, EnemyCatalogSnapshot, EnemyCatalogSource,
    EnemyConditionReference, EnemyCooldownRule, EnemyDefinitionInput, EnemyDefinitionReference,
    EnemyEncounterReference, EnemyEvidence, EnemyFamilyCoverage, EnemyFamilyState, EnemyField,
    EnemyKind, EnemyMoveDefinitionInput, EnemyMoveEffect, EnemyMoveEffectKind, EnemyMoveReference,
    EnemyNumericValue, EnemyOrigin, EnemyOriginVariant, EnemyParameter, EnemyPhaseDefinition,
    EnemyProbability, EnemyRepetitionRule, EnemySemanticReference, EnemySemanticReferenceKind,
    EnemySourceError, EnemyStat, EnemyStatProfile, EnemyStats, EnemyTag, EnemyTargetDomain,
    EnemyTargeting, EnemyText, EnemyUnavailableReason, EnemyVisibility,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    pub snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

fn content_definition(entity_kind: &str, id: &str) -> ContentDefinitionInput {
    ContentDefinitionInput {
        entity_kind: entity_kind.to_owned(),
        namespaced_id: id.to_owned(),
        semantic_inputs: format!("{entity_kind}={id}"),
        localized_text: Some(id.to_owned()),
        origin: ContentOriginInput {
            package_id: Some("base:synthetic".to_owned()),
            package_version: Some("1".to_owned()),
        },
        override_chain: Vec::new(),
    }
}

pub fn manifest_source(
    enemy_ids: &[&str],
    encounter_ids: &[&str],
    status_ids: &[&str],
) -> ManifestSource {
    let mut definitions = Vec::new();
    definitions.extend(enemy_ids.iter().map(|id| content_definition("enemy", id)));
    definitions.extend(
        encounter_ids
            .iter()
            .map(|id| content_definition("encounter", id)),
    );
    definitions.extend(
        status_ids
            .iter()
            .map(|id| content_definition("power_status", id)),
    );
    ManifestSource {
        snapshot: ContentCatalogSnapshot {
            generation_before: 12,
            generation_after: 12,
            game_build: "sts2-build:synthetic".to_owned(),
            locale: "en-US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:synthetic".to_owned(),
                package_version: Some("1".to_owned()),
                order: 0,
            }],
            available_entity_kinds: vec![
                "encounter".to_owned(),
                "enemy".to_owned(),
                "power_status".to_owned(),
            ],
            registry_definition_counts: [
                ("encounter".to_owned(), encounter_ids.len()),
                ("enemy".to_owned(), enemy_ids.len()),
                ("power_status".to_owned(), status_ids.len()),
            ]
            .into(),
            definitions,
        },
    }
}

pub fn manifest(
    enemy_ids: &[&str],
    encounter_ids: &[&str],
    status_ids: &[&str],
) -> ContentManifest {
    ContentManifestProducer::new(
        "adapter-v1",
        [
            "encounter".to_owned(),
            "enemy".to_owned(),
            "power_status".to_owned(),
        ],
    )
    .expect("producer")
    .produce(&manifest_source(enemy_ids, encounter_ids, status_ids))
    .expect("manifest")
}

pub fn text(value: &str) -> EnemyText {
    EnemyText::available(value).expect("text")
}

pub fn origin() -> EnemyOrigin {
    EnemyOrigin {
        kind: "base".to_owned(),
        package_id: Some("base:synthetic".to_owned()),
        package_version: Some("1".to_owned()),
    }
}

pub fn condition(id: &str) -> EnemyConditionReference {
    EnemyConditionReference {
        condition_id: id.to_owned(),
        label: text(id),
        parameters: Vec::new(),
    }
}

pub fn reference(kind: EnemySemanticReferenceKind, id: &str) -> EnemySemanticReference {
    EnemySemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

pub fn targeting(domain: EnemyTargetDomain, count: i64) -> EnemyTargeting {
    EnemyTargeting {
        domain,
        count: EnemyNumericValue::Fixed(count),
    }
}

pub fn stat(stat_id: &str, unit: &str, value: i64) -> EnemyStat {
    EnemyStat {
        stat_id: stat_id.to_owned(),
        unit: Some(unit.to_owned()),
        value: EnemyNumericValue::Fixed(value),
    }
}

pub fn stats() -> EnemyStats {
    EnemyStats {
        base: EnemyField::Available(vec![stat("hp", "hp", 120)]),
        scaled: EnemyField::Available(vec![
            EnemyStatProfile {
                profile_id: "normal".to_owned(),
                mode: Some("normal".to_owned()),
                difficulty: Some("normal".to_owned()),
                stats: vec![stat("hp", "hp", 120)],
            },
            EnemyStatProfile {
                profile_id: "hard".to_owned(),
                mode: Some("hard".to_owned()),
                difficulty: Some("hard".to_owned()),
                stats: vec![stat("hp", "hp", 160)],
            },
        ]),
    }
}

pub fn effect(
    effect_id: &str,
    kind: EnemyMoveEffectKind,
    description: &str,
    references: Vec<EnemySemanticReference>,
) -> EnemyMoveEffect {
    EnemyMoveEffect {
        effect_id: effect_id.to_owned(),
        kind,
        description: text(description),
        targeting: targeting(EnemyTargetDomain::AnyPlayer, 1),
        parameters: Vec::new(),
        references,
    }
}

pub fn move_input(
    move_id: &str,
    name: &str,
    effects: Vec<EnemyMoveEffect>,
    phase_ids: &[&str],
) -> EnemyMoveDefinitionInput {
    EnemyMoveDefinitionInput {
        move_id: move_id.to_owned(),
        name: text(name),
        description: text("A synthetic move."),
        effects,
        targeting: targeting(EnemyTargetDomain::AnyPlayer, 1),
        phase_ids: phase_ids.iter().map(|id| (*id).to_owned()).collect(),
        conditions: vec![condition("condition:reachable")],
        cooldown: EnemyCooldownRule::None,
        repetition: EnemyRepetitionRule::Allow,
        probability: EnemyProbability::Exact {
            numerator: 1,
            denominator: 2,
            evidence: EnemyEvidence::SourceDerived,
        },
        references: Vec::new(),
        visibility: EnemyVisibility::Visible,
    }
}

fn phase(phase_id: &str, order: u16, move_ids: &[&str]) -> EnemyPhaseDefinition {
    EnemyPhaseDefinition {
        phase_id: phase_id.to_owned(),
        name: text(phase_id),
        description: text("A synthetic phase."),
        order,
        move_ids: move_ids.iter().map(|id| (*id).to_owned()).collect(),
        entry_condition: EnemyField::Unavailable(EnemyUnavailableReason::NotApplicable),
    }
}

pub fn boss(enemy_id: &str) -> EnemyDefinitionInput {
    let mut slam = move_input(
        "move:slam",
        "Slam",
        vec![
            effect(
                "effect:damage",
                EnemyMoveEffectKind::Attack,
                "Deal 12 damage.",
                vec![reference(
                    EnemySemanticReferenceKind::Status,
                    "power_status:weak",
                )],
            ),
            effect(
                "effect:debuff",
                EnemyMoveEffectKind::ApplyStatus,
                "Apply 2 Weak.",
                vec![reference(
                    EnemySemanticReferenceKind::Status,
                    "power_status:weak",
                )],
            ),
        ],
        &["phase:one"],
    );
    slam.cooldown = EnemyCooldownRule::Turns(EnemyNumericValue::Fixed(2));
    slam.repetition = EnemyRepetitionRule::NoImmediateRepeat;
    let summon = move_input(
        "move:summon",
        "Summon",
        vec![effect(
            "effect:summon",
            EnemyMoveEffectKind::Summon,
            "Summon a minion.",
            Vec::new(),
        )],
        &["phase:one"],
    );
    let mut enrage = move_input(
        "move:enrage",
        "Enrage",
        vec![effect(
            "effect:block",
            EnemyMoveEffectKind::Block,
            "Gain 8 block.",
            Vec::new(),
        )],
        &["phase:two"],
    );
    enrage.probability = EnemyProbability::Unavailable(EnemyUnavailableReason::NotApplicable);
    EnemyDefinitionInput {
        enemy_id: enemy_id.to_owned(),
        name: text("Fixture Boss"),
        description: text("A multi-phase boss."),
        kind: EnemyKind::Boss,
        origin: origin(),
        unlock_state: ContentUnlockState::Unlocked,
        visibility: EnemyVisibility::Visible,
        tags: vec![EnemyTag {
            tag_id: "tag:boss".to_owned(),
            label: text("Boss"),
        }],
        stats: stats(),
        spawn_conditions: EnemyField::Available(vec![condition("condition:act")]),
        encounters: EnemyField::Available(vec![EnemyEncounterReference {
            encounter_id: "encounter:boss".to_owned(),
            label: text("Boss Encounter"),
            role: Some("boss".to_owned()),
        }]),
        origin_variants: vec![EnemyOriginVariant {
            variant_id: "variant:default".to_owned(),
            label: text("Default origin"),
            origin: origin(),
            stats: EnemyField::Unavailable(EnemyUnavailableReason::NotApplicable),
            move_ids: EnemyField::Unavailable(EnemyUnavailableReason::NotApplicable),
        }],
        phases: vec![
            phase("phase:one", 0, &["move:slam", "move:summon"]),
            phase("phase:two", 1, &["move:enrage"]),
        ],
        moves: vec![slam, summon, enrage],
        transitions: vec![EnemyBehaviorTransition {
            transition_id: "transition:enrage".to_owned(),
            from_phase: Some("phase:one".to_owned()),
            to_phase: "phase:two".to_owned(),
            condition: condition("condition:half-hp"),
            probability: EnemyProbability::Exact {
                numerator: 1,
                denominator: 1,
                evidence: EnemyEvidence::IndependentlyAuthored,
            },
            references: vec![reference(EnemySemanticReferenceKind::Rule, "rule:enrage")],
        }],
        references: vec![reference(
            EnemySemanticReferenceKind::Encounter,
            "encounter:boss",
        )],
    }
}

pub fn enemy(
    enemy_id: &str,
    kind: EnemyKind,
    visibility: EnemyVisibility,
    unlock_state: ContentUnlockState,
) -> EnemyDefinitionInput {
    EnemyDefinitionInput {
        enemy_id: enemy_id.to_owned(),
        name: text(enemy_id),
        description: text("A synthetic enemy."),
        kind,
        origin: origin(),
        unlock_state,
        visibility,
        tags: Vec::new(),
        stats: EnemyStats {
            base: EnemyField::Available(vec![stat("hp", "hp", 30)]),
            scaled: EnemyField::Unavailable(EnemyUnavailableReason::NotObserved),
        },
        spawn_conditions: EnemyField::Unavailable(EnemyUnavailableReason::NotObserved),
        encounters: EnemyField::Unavailable(EnemyUnavailableReason::NotApplicable),
        origin_variants: Vec::new(),
        phases: vec![phase("phase:one", 0, &["move:strike"])],
        moves: vec![move_input(
            "move:strike",
            "Strike",
            vec![effect(
                "effect:strike",
                EnemyMoveEffectKind::Attack,
                "Deal 5 damage.",
                Vec::new(),
            )],
            &["phase:one"],
        )],
        transitions: Vec::new(),
        references: Vec::new(),
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<EnemyDefinitionInput>,
) -> EnemyCatalogSnapshot {
    EnemyCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: ENEMY_PRODUCER_VERSION.to_owned(),
        family: EnemyFamilyCoverage {
            entity_kind: ENEMY_ENTITY_KIND.to_owned(),
            state: EnemyFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnemySource {
    pub snapshot: Result<EnemyCatalogSnapshot, EnemySourceError>,
}

impl EnemyCatalogSource for EnemySource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<EnemyCatalogSnapshot, EnemySourceError> {
        self.snapshot.clone()
    }
}

pub fn catalog(manifest: &ContentManifest, definitions: Vec<EnemyDefinitionInput>) -> EnemyCatalog {
    EnemyCatalogProducer::new()
        .produce(
            manifest,
            &EnemySource {
                snapshot: Ok(snapshot(manifest, definitions)),
            },
        )
        .expect("catalog")
}

pub fn definition_reference(catalog: &EnemyCatalog, enemy_id: &str) -> EnemyDefinitionReference {
    EnemyDefinitionReference {
        catalog: catalog.binding().clone(),
        enemy_id: enemy_id.to_owned(),
    }
}

pub fn move_reference(catalog: &EnemyCatalog, enemy_id: &str, move_id: &str) -> EnemyMoveReference {
    EnemyMoveReference {
        catalog: catalog.binding().clone(),
        enemy_id: enemy_id.to_owned(),
        move_id: move_id.to_owned(),
    }
}

pub fn parameter(parameter_id: &str, value: i64) -> EnemyParameter {
    EnemyParameter {
        parameter_id: parameter_id.to_owned(),
        label: text(parameter_id),
        unit: Some("count".to_owned()),
        value: EnemyNumericValue::Fixed(value),
    }
}
