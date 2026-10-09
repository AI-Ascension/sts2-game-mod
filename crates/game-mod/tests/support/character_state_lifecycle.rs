// SPDX-License-Identifier: MIT

use super::character_state_support::{
    CatalogFixture, catalog_snapshot, coverage, live_binding, manifest, unit,
};
use sts2_game_mod::{
    CharacterResourceInput, CharacterResourceReference, CharacterResourceValue,
    CharacterSecondaryEntityReference, CharacterStateCatalog, CharacterStateCatalogProducer,
    CharacterStateField, CharacterStateLiveReader, CharacterStateLiveSnapshot,
    CharacterStateLiveSnapshotInput, CharacterStateOwner, CharacterStateOwnerId,
    CharacterStateOwnerKind, SecondaryEntityControllerReference, SecondaryEntityInput,
};

fn catalog_with_two_characters() -> CharacterStateCatalog {
    let manifest = manifest();
    let mut source = catalog_snapshot(&manifest);
    source.coverage.push(coverage("char-b"));

    let mut resource = source.resource_definitions[0].clone();
    resource.definition_id = "resource:charge-b".to_owned();
    resource.character_id = "char-b".to_owned();
    resource.label = "Charge B".to_owned();
    resource.rule_reference = "rule:charge-b".to_owned();
    source.resource_definitions.push(resource);

    let mut entity = source.secondary_entity_definitions[0].clone();
    entity.definition_id = "entity:orb-b".to_owned();
    entity.character_id = "char-b".to_owned();
    entity.label = "Orb B".to_owned();
    entity.rule_reference = "rule:orb-b".to_owned();
    source.secondary_entity_definitions.push(entity);

    CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(source))
        .expect("two-character catalog")
}

fn owner(character_id: &str, kind: CharacterStateOwnerKind, owner_id: &str) -> CharacterStateOwner {
    CharacterStateOwner {
        kind,
        id: CharacterStateOwnerId::new(owner_id).expect("synthetic owner ID"),
        character_id: character_id.to_owned(),
        label: CharacterStateField::Available(owner_id.to_owned()),
    }
}

fn integer(value: i64) -> CharacterStateField<CharacterResourceValue> {
    CharacterStateField::Available(CharacterResourceValue::Integer {
        value,
        unit: unit("charge"),
    })
}

fn resource_input(
    instance_id: &str,
    character_id: &str,
    owner_id: &str,
    definition_id: &str,
    current: i64,
) -> CharacterResourceInput {
    let resource_unit = unit("charge");
    CharacterResourceInput {
        instance_id: instance_id.to_owned(),
        definition_id: definition_id.to_owned(),
        owner: owner(character_id, CharacterStateOwnerKind::Player, owner_id),
        current: integer(current),
        maximum: CharacterStateField::Available(CharacterResourceValue::Integer {
            value: 3,
            unit: resource_unit,
        }),
        slots: CharacterStateField::NotApplicable,
        active: CharacterStateField::Available(true),
    }
}

fn entity_input(
    instance_id: &str,
    character_id: &str,
    entity_owner_id: &str,
    definition_id: &str,
    controller_owner_id: &str,
    hp: i64,
    block: i64,
) -> SecondaryEntityInput {
    SecondaryEntityInput {
        instance_id: instance_id.to_owned(),
        definition_id: definition_id.to_owned(),
        owner: owner(
            character_id,
            CharacterStateOwnerKind::Secondary,
            entity_owner_id,
        ),
        controller: CharacterStateField::Available(SecondaryEntityControllerReference {
            owner_kind: CharacterStateOwnerKind::Player,
            owner_id: CharacterStateOwnerId::new(controller_owner_id)
                .expect("synthetic controller ID"),
            character_id: character_id.to_owned(),
            label: CharacterStateField::Available(format!("{character_id} controller")),
        }),
        hp: CharacterStateField::Available(hp),
        maximum_hp: CharacterStateField::Available(12),
        block: CharacterStateField::Available(block),
        statuses: CharacterStateField::NotObserved,
        intent: CharacterStateField::NotObserved,
        active: CharacterStateField::Available(true),
    }
}

fn live_snapshot(
    catalog: &CharacterStateCatalog,
    epoch: u64,
    resources: Vec<CharacterResourceInput>,
    secondary_entities: Vec<SecondaryEntityInput>,
) -> CharacterStateLiveSnapshot {
    CharacterStateLiveSnapshot::from_input(CharacterStateLiveSnapshotInput {
        binding: live_binding(catalog, epoch),
        resources,
        secondary_entities,
    })
    .expect("coherent synthetic snapshot")
}

fn only_resource(
    reader: &CharacterStateLiveReader,
    character_id: &str,
) -> CharacterResourceReference {
    let references = reader
        .resource_references_for(character_id)
        .expect("resource references");
    assert_eq!(references.len(), 1);
    references
        .into_iter()
        .next()
        .expect("one resource reference")
}

fn only_entity(
    reader: &CharacterStateLiveReader,
    character_id: &str,
) -> CharacterSecondaryEntityReference {
    let references = reader
        .secondary_entity_references_for(character_id)
        .expect("secondary-entity references");
    assert_eq!(references.len(), 1);
    references.into_iter().next().expect("one entity reference")
}

#[test]
fn live_snapshot_replacement_tracks_mechanic_add_change_and_removal() {
    let catalog = catalog_with_two_characters();
    let first = live_snapshot(
        &catalog,
        1,
        vec![resource_input(
            "resource-a",
            "char-a",
            "owner-a",
            "resource:charge",
            1,
        )],
        vec![entity_input(
            "entity-a",
            "char-a",
            "entity-owner-a",
            "entity:orb",
            "owner-a",
            9,
            1,
        )],
    );
    let mut reader = CharacterStateLiveReader::new(&catalog, first).expect("live reader");
    let first_resource = only_resource(&reader, "char-a");
    let first_entity = only_entity(&reader, "char-a");
    assert_eq!(
        reader
            .get_resource(&first_resource)
            .expect("resource")
            .current,
        integer(1)
    );
    assert_eq!(
        reader
            .get_secondary_entity(&first_entity)
            .expect("entity")
            .hp,
        CharacterStateField::Available(9)
    );

    reader
        .replace_snapshot(live_snapshot(
            &catalog,
            2,
            vec![
                resource_input("resource-a", "char-a", "owner-a", "resource:charge", 2),
                resource_input("resource-b", "char-b", "owner-b", "resource:charge-b", 3),
            ],
            vec![
                entity_input(
                    "entity-a",
                    "char-a",
                    "entity-owner-a",
                    "entity:orb",
                    "owner-a",
                    7,
                    3,
                ),
                entity_input(
                    "entity-b",
                    "char-b",
                    "entity-owner-b",
                    "entity:orb-b",
                    "owner-b",
                    5,
                    1,
                ),
            ],
        ))
        .expect("higher epoch with changed and added mechanics");
    let resource_a_epoch2 = only_resource(&reader, "char-a");
    let resource_b_epoch2 = only_resource(&reader, "char-b");
    let entity_a_epoch2 = only_entity(&reader, "char-a");
    let entity_b_epoch2 = only_entity(&reader, "char-b");
    let resource_a = reader
        .get_resource(&resource_a_epoch2)
        .expect("changed resource");
    assert_eq!(resource_a.current, integer(2));
    assert_eq!(resource_a.slots, CharacterStateField::NotApplicable);
    assert_eq!(
        reader
            .get_resource(&resource_b_epoch2)
            .expect("added resource")
            .current,
        integer(3)
    );
    let entity_a = reader
        .get_secondary_entity(&entity_a_epoch2)
        .expect("changed entity");
    assert_eq!(entity_a.hp, CharacterStateField::Available(7));
    assert_eq!(entity_a.block, CharacterStateField::Available(3));
    assert_eq!(entity_a.statuses, CharacterStateField::NotObserved);
    assert_eq!(entity_a.intent, CharacterStateField::NotObserved);
    assert_eq!(
        reader
            .get_secondary_entity(&entity_b_epoch2)
            .expect("added entity")
            .hp,
        CharacterStateField::Available(5)
    );
    assert_eq!(
        reader.get_resource(&first_resource),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );
    assert_eq!(
        reader.get_secondary_entity(&first_entity),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );

    reader
        .replace_snapshot(live_snapshot(
            &catalog,
            3,
            vec![resource_input(
                "resource-b",
                "char-b",
                "owner-b",
                "resource:charge-b",
                1,
            )],
            vec![entity_input(
                "entity-a",
                "char-a",
                "entity-owner-a",
                "entity:orb",
                "owner-a",
                6,
                4,
            )],
        ))
        .expect("next higher epoch with removals");
    assert!(
        reader
            .resource_references_for("char-a")
            .expect("char-a resources")
            .is_empty()
    );
    assert!(
        reader
            .secondary_entity_references_for("char-b")
            .expect("char-b entities")
            .is_empty()
    );
    assert_eq!(
        reader.get_resource(&resource_a_epoch2),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );
    assert_eq!(
        reader.get_secondary_entity(&entity_b_epoch2),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );
    assert_eq!(
        reader
            .get_resource(&only_resource(&reader, "char-b"))
            .expect("retained resource")
            .current,
        integer(1)
    );
    assert_eq!(
        reader
            .get_secondary_entity(&only_entity(&reader, "char-a"))
            .expect("retained entity")
            .hp,
        CharacterStateField::Available(6)
    );
}

#[test]
fn live_queries_keep_each_character_owners_resources_and_entities_isolated() {
    let catalog = catalog_with_two_characters();
    let reader = CharacterStateLiveReader::new(
        &catalog,
        live_snapshot(
            &catalog,
            1,
            vec![
                resource_input("resource-a", "char-a", "owner-a", "resource:charge", 1),
                resource_input("resource-b", "char-b", "owner-b", "resource:charge-b", 3),
            ],
            vec![
                entity_input(
                    "entity-a",
                    "char-a",
                    "entity-owner-a",
                    "entity:orb",
                    "owner-a",
                    9,
                    1,
                ),
                entity_input(
                    "entity-b",
                    "char-b",
                    "entity-owner-b",
                    "entity:orb-b",
                    "owner-b",
                    5,
                    2,
                ),
            ],
        ),
    )
    .expect("two-owner reader");

    let resource_a = only_resource(&reader, "char-a");
    let resource_b = only_resource(&reader, "char-b");
    assert_eq!(resource_a.instance_id, "resource-a");
    assert_eq!(resource_b.instance_id, "resource-b");
    let detail_a = reader.get_resource(&resource_a).expect("char-a resource");
    let detail_b = reader.get_resource(&resource_b).expect("char-b resource");
    assert_eq!(detail_a.owner.id.as_str(), "owner-a");
    assert_eq!(detail_b.owner.id.as_str(), "owner-b");
    assert_eq!(detail_a.current, integer(1));
    assert_eq!(detail_b.current, integer(3));

    let entity_a = only_entity(&reader, "char-a");
    let entity_b = only_entity(&reader, "char-b");
    assert_eq!(entity_a.instance_id, "entity-a");
    assert_eq!(entity_b.instance_id, "entity-b");
    let detail_a = reader
        .get_secondary_entity(&entity_a)
        .expect("char-a entity");
    let detail_b = reader
        .get_secondary_entity(&entity_b)
        .expect("char-b entity");
    assert_eq!(detail_a.owner.id.as_str(), "entity-owner-a");
    assert_eq!(detail_b.owner.id.as_str(), "entity-owner-b");
    assert_eq!(
        detail_a
            .controller
            .value()
            .expect("char-a controller")
            .owner_id
            .as_str(),
        "owner-a"
    );
    assert_eq!(
        detail_b
            .controller
            .value()
            .expect("char-b controller")
            .owner_id
            .as_str(),
        "owner-b"
    );
    assert_eq!(detail_a.hp, CharacterStateField::Available(9));
    assert_eq!(detail_b.hp, CharacterStateField::Available(5));

    let mut cross_owner_resource = resource_a;
    cross_owner_resource.owner_id = resource_b.owner_id;
    assert_eq!(
        reader.get_resource(&cross_owner_resource),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );
    let mut cross_owner_entity = entity_a;
    cross_owner_entity.owner_id = entity_b.owner_id;
    assert_eq!(
        reader.get_secondary_entity(&cross_owner_entity),
        Err(sts2_game_mod::CharacterStateLiveError::StaleReference)
    );
}
