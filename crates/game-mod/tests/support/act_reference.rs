// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    ACT_REFERENCE_ENTITY_KIND, ACT_REFERENCE_PRODUCER_VERSION, ActCatalog, ActCatalogProducer,
    ActCatalogSnapshot, ActCatalogSource, ActDefinitionInput, ActDefinitionReference,
    ActEncounterReference, ActEvidence, ActFamilyCoverage, ActFamilyState, ActField,
    ActNumericValue, ActParameter, ActPoolReference, ActRoomCategoryReference,
    ActSemanticReference, ActSemanticReferenceKind, ActSourceError, ActText, ActVisibility,
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput, ContentSourceError,
    ContentUnlockState, EligibilityCondition, EligibilityKind, EncounterDefinitionInput,
    EncounterEnemy, EncounterEnemyGroup, EncounterKind, EncounterPool, EncounterPoolEntry,
    EncounterPoolKind, GenerationWeight, MapConstraintKind, MapGenerationConstraint,
    RoomCategoryDefinition, RoomCategoryKind,
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
    act_ids: &[&str],
    encounter_ids: &[&str],
    enemy_ids: &[&str],
) -> ManifestSource {
    let mut definitions = Vec::new();
    definitions.extend(act_ids.iter().map(|id| content_definition("act", id)));
    definitions.extend(
        encounter_ids
            .iter()
            .map(|id| content_definition("encounter", id)),
    );
    definitions.extend(enemy_ids.iter().map(|id| content_definition("enemy", id)));
    ManifestSource {
        snapshot: ContentCatalogSnapshot {
            generation_before: 7,
            generation_after: 7,
            game_build: "sts2-build:synthetic".to_owned(),
            locale: "en-US".to_owned(),
            packages: vec![ContentPackageInput {
                package_id: "base:synthetic".to_owned(),
                package_version: Some("1".to_owned()),
                order: 0,
            }],
            available_entity_kinds: vec![
                "act".to_owned(),
                "encounter".to_owned(),
                "enemy".to_owned(),
            ],
            definitions,
        },
    }
}

pub fn manifest(act_ids: &[&str], encounter_ids: &[&str], enemy_ids: &[&str]) -> ContentManifest {
    ContentManifestProducer::new(
        "adapter-v1",
        ["act".to_owned(), "encounter".to_owned(), "enemy".to_owned()],
    )
    .expect("producer")
    .produce(&manifest_source(act_ids, encounter_ids, enemy_ids))
    .expect("manifest")
}

pub fn text(value: &str) -> ActText {
    ActText::available(value).expect("text")
}

pub fn reference(kind: ActSemanticReferenceKind, id: &str) -> ActSemanticReference {
    ActSemanticReference {
        kind,
        id: id.to_owned(),
        label: text(id),
    }
}

pub fn parameter(parameter_id: &str, value: i64) -> ActParameter {
    ActParameter {
        parameter_id: parameter_id.to_owned(),
        label: text(parameter_id),
        unit: Some("count".to_owned()),
        value: ActNumericValue::Fixed(value),
    }
}

pub fn eligibility(condition_id: &str, kind: EligibilityKind) -> EligibilityCondition {
    EligibilityCondition {
        condition_id: condition_id.to_owned(),
        kind,
        label: text(condition_id),
        parameters: vec![parameter("min", 1)],
        references: Vec::new(),
        visibility: ActVisibility::Visible,
    }
}

pub fn room_category(category_id: &str, kind: RoomCategoryKind) -> RoomCategoryDefinition {
    RoomCategoryDefinition {
        category_id: category_id.to_owned(),
        name: text(category_id),
        kind,
        description: text("A synthetic category."),
        references: Vec::new(),
        visibility: ActVisibility::Visible,
    }
}

pub fn enemy(enemy_id: &str, quantity: i64) -> EncounterEnemy {
    EncounterEnemy {
        enemy_id: enemy_id.to_owned(),
        quantity: ActNumericValue::Fixed(quantity),
        variant_ids: Vec::new(),
        references: Vec::new(),
    }
}

pub fn group(group_id: &str, enemies: Vec<EncounterEnemy>) -> EncounterEnemyGroup {
    EncounterEnemyGroup {
        group_id: group_id.to_owned(),
        label: text(group_id),
        enemies,
        references: Vec::new(),
    }
}

pub fn encounter(
    encounter_id: &str,
    kind: EncounterKind,
    category_id: Option<&str>,
    groups: Vec<EncounterEnemyGroup>,
) -> EncounterDefinitionInput {
    EncounterDefinitionInput {
        encounter_id: encounter_id.to_owned(),
        name: text(encounter_id),
        kind,
        room_category_id: category_id.map(str::to_owned),
        groups,
        eligibility: vec![eligibility("condition:act", EligibilityKind::ActOrder)],
        weight: exact_weight(1, 1),
        references: Vec::new(),
        visibility: ActVisibility::Visible,
    }
}

pub fn entry(entry_id: &str, encounter_id: &str) -> EncounterPoolEntry {
    EncounterPoolEntry {
        entry_id: entry_id.to_owned(),
        encounter_id: encounter_id.to_owned(),
        weight: exact_weight(1, 1),
    }
}

pub fn pool(
    pool_id: &str,
    kind: EncounterPoolKind,
    category_id: &str,
    entries: Vec<EncounterPoolEntry>,
) -> EncounterPool {
    EncounterPool {
        pool_id: pool_id.to_owned(),
        name: text(pool_id),
        kind,
        room_category_id: Some(category_id.to_owned()),
        entries,
        references: Vec::new(),
        visibility: ActVisibility::Visible,
    }
}

pub fn constraint(constraint_id: &str, kind: MapConstraintKind) -> MapGenerationConstraint {
    MapGenerationConstraint {
        constraint_id: constraint_id.to_owned(),
        kind,
        label: text(constraint_id),
        rule_reference: ActField::Available("rule:paths".to_owned()),
        mode: Some("standard".to_owned()),
        difficulty: None,
        parameters: vec![parameter("count", 6)],
        references: Vec::new(),
        evidence: ActEvidence::SourceDerived,
        visibility: ActVisibility::Visible,
    }
}

pub fn exact_weight(numerator: u32, denominator: u32) -> GenerationWeight {
    GenerationWeight::Exact {
        numerator,
        denominator,
        evidence: ActEvidence::SourceDerived,
    }
}

pub fn simple_act(act_id: &str) -> ActDefinitionInput {
    ActDefinitionInput {
        act_id: act_id.to_owned(),
        name: text(act_id),
        description: text("A synthetic act."),
        order: 1,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: ActVisibility::Visible,
        room_categories: vec![room_category("category:normal", RoomCategoryKind::Normal)],
        encounters: vec![encounter(
            "encounter:normal",
            EncounterKind::Normal,
            Some("category:normal"),
            vec![group("group:one", vec![enemy("enemy:slime", 1)])],
        )],
        pools: ActField::Available(vec![pool(
            "pool:normal",
            EncounterPoolKind::Normal,
            "category:normal",
            vec![entry("entry:normal", "encounter:normal")],
        )]),
        constraints: ActField::Available(vec![constraint(
            "constraint:paths",
            MapConstraintKind::PathCount,
        )]),
        references: Vec::new(),
    }
}

pub fn rich_act(act_id: &str) -> ActDefinitionInput {
    let mut normal = encounter(
        "encounter:normal",
        EncounterKind::Normal,
        Some("category:normal"),
        vec![group(
            "group:normal",
            vec![enemy("enemy:slime", 2), enemy("enemy:brute", 1)],
        )],
    );
    normal
        .references
        .push(reference(ActSemanticReferenceKind::Enemy, "enemy:slime"));
    let mut elite = encounter(
        "encounter:elite",
        EncounterKind::Elite,
        Some("category:elite"),
        vec![group("group:elite", vec![enemy("enemy:brute", 1)])],
    );
    elite.groups[0].enemies[0].variant_ids = vec!["variant:armored".to_owned()];
    let boss = encounter(
        "encounter:boss",
        EncounterKind::Boss,
        Some("category:boss"),
        vec![group("group:boss", vec![enemy("enemy:lord", 1)])],
    );
    ActDefinitionInput {
        act_id: act_id.to_owned(),
        name: text("Synthetic Act"),
        description: text("A synthetic act with pools and constraints."),
        order: 1,
        unlock_state: ContentUnlockState::Unlocked,
        visibility: ActVisibility::Visible,
        room_categories: vec![
            room_category("category:normal", RoomCategoryKind::Normal),
            room_category("category:elite", RoomCategoryKind::Elite),
            room_category("category:boss", RoomCategoryKind::Boss),
        ],
        encounters: vec![normal, elite, boss],
        pools: ActField::Available(vec![
            pool(
                "pool:normal",
                EncounterPoolKind::Normal,
                "category:normal",
                vec![entry("entry:normal", "encounter:normal")],
            ),
            pool(
                "pool:elite",
                EncounterPoolKind::Elite,
                "category:elite",
                vec![entry("entry:elite", "encounter:elite")],
            ),
            pool(
                "pool:boss",
                EncounterPoolKind::Boss,
                "category:boss",
                vec![entry("entry:boss", "encounter:boss")],
            ),
        ]),
        constraints: ActField::Available(vec![
            constraint("constraint:paths", MapConstraintKind::PathCount),
            constraint("constraint:boss", MapConstraintKind::BossPlacement),
        ]),
        references: vec![
            reference(ActSemanticReferenceKind::Enemy, "enemy:slime"),
            reference(ActSemanticReferenceKind::Encounter, "encounter:shared"),
        ],
    }
}

pub fn snapshot(
    manifest: &ContentManifest,
    definitions: Vec<ActDefinitionInput>,
) -> ActCatalogSnapshot {
    ActCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: ACT_REFERENCE_PRODUCER_VERSION.to_owned(),
        family: ActFamilyCoverage {
            entity_kind: ACT_REFERENCE_ENTITY_KIND.to_owned(),
            state: ActFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActSource {
    pub snapshot: Result<ActCatalogSnapshot, ActSourceError>,
}

impl ActCatalogSource for ActSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<ActCatalogSnapshot, ActSourceError> {
        self.snapshot.clone()
    }
}

pub fn catalog(manifest: &ContentManifest, definitions: Vec<ActDefinitionInput>) -> ActCatalog {
    ActCatalogProducer::new()
        .produce(
            manifest,
            &ActSource {
                snapshot: Ok(snapshot(manifest, definitions)),
            },
        )
        .expect("catalog")
}

pub fn act_reference(catalog: &ActCatalog, act_id: &str) -> ActDefinitionReference {
    ActDefinitionReference {
        catalog: catalog.binding().clone(),
        act_id: act_id.to_owned(),
    }
}

pub fn encounter_reference(
    catalog: &ActCatalog,
    act_id: &str,
    encounter_id: &str,
) -> ActEncounterReference {
    ActEncounterReference {
        catalog: catalog.binding().clone(),
        act_id: act_id.to_owned(),
        encounter_id: encounter_id.to_owned(),
    }
}

pub fn category_reference(
    catalog: &ActCatalog,
    act_id: &str,
    category_id: &str,
) -> ActRoomCategoryReference {
    ActRoomCategoryReference {
        catalog: catalog.binding().clone(),
        act_id: act_id.to_owned(),
        category_id: category_id.to_owned(),
    }
}

pub fn pool_reference(catalog: &ActCatalog, act_id: &str, pool_id: &str) -> ActPoolReference {
    ActPoolReference {
        catalog: catalog.binding().clone(),
        act_id: act_id.to_owned(),
        pool_id: pool_id.to_owned(),
    }
}
