// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    CHARACTER_STATE_PRODUCER_VERSION, CharacterMechanicCoverage, CharacterMechanicState,
    CharacterResourceDefinitionInput, CharacterResourceKind, CharacterResourceSlot,
    CharacterResourceSlotContent, CharacterResourceValue, CharacterResourceValueDefinition,
    CharacterStateCatalog, CharacterStateCatalogProducer, CharacterStateCatalogSnapshot,
    CharacterStateCatalogSource, CharacterStateField, CharacterStateLiveBinding,
    CharacterStateLiveSnapshot, CharacterStateLiveSnapshotInput, CharacterStateOwner,
    CharacterStateOwnerId, CharacterStateOwnerKind, CharacterStateSourceError, CharacterStateUnit,
    CharacterStateVisibility, ContentCursorBinding, ContentManifest,
    SecondaryEntityControllerReference, SecondaryEntityDefinitionInput, SecondaryEntityInput,
    SecondaryEntityIntent, SecondaryEntityIntentKind, SecondaryEntityKind, SecondaryEntityStatus,
    SecondaryEntityTarget,
};

pub(crate) fn manifest() -> ContentManifest {
    ContentManifest {
        game_build: "synthetic-build".to_owned(),
        adapter_compatibility: "synthetic-adapter".to_owned(),
        catalog_generation: 7,
        locale: "en".to_owned(),
        packages: Vec::new(),
        families: Vec::new(),
        definitions: Vec::new(),
        content_set_revision: "content-r1".to_owned(),
        localized_text_revision: "text-r1".to_owned(),
        inventory_revision: "inventory-r1".to_owned(),
    }
}

pub(crate) fn binding(manifest: &ContentManifest) -> ContentCursorBinding {
    manifest.cursor_binding()
}

pub(crate) fn unit(value: &str) -> CharacterStateUnit {
    CharacterStateUnit::new(value).expect("unit")
}

pub(crate) fn coverage(character_id: &str) -> CharacterMechanicCoverage {
    CharacterMechanicCoverage {
        character_id: character_id.to_owned(),
        mode_id: "standard".to_owned(),
        resources: CharacterMechanicState::Supported,
        secondary_entities: CharacterMechanicState::Supported,
    }
}

pub(crate) fn catalog_snapshot(manifest: &ContentManifest) -> CharacterStateCatalogSnapshot {
    CharacterStateCatalogSnapshot {
        manifest: binding(manifest),
        locale: manifest.locale.clone(),
        producer_version: CHARACTER_STATE_PRODUCER_VERSION.to_owned(),
        coverage: vec![coverage("char-a")],
        resource_definitions: vec![CharacterResourceDefinitionInput {
            definition_id: "resource:charge".to_owned(),
            character_id: "char-a".to_owned(),
            mode_id: "standard".to_owned(),
            kind: CharacterResourceKind::Meter,
            label: "Charge".to_owned(),
            rule_reference: "rule:charge".to_owned(),
            value: CharacterResourceValueDefinition::Integer {
                unit: unit("charge"),
            },
            maximum: Some(3),
            visibility: CharacterStateVisibility::Visible,
            slots_visibility: CharacterStateVisibility::Visible,
        }],
        secondary_entity_definitions: vec![SecondaryEntityDefinitionInput {
            definition_id: "entity:orb".to_owned(),
            character_id: "char-a".to_owned(),
            mode_id: "standard".to_owned(),
            kind: SecondaryEntityKind::Orb,
            label: "Orb".to_owned(),
            rule_reference: "rule:orb".to_owned(),
            visibility: CharacterStateVisibility::Visible,
        }],
    }
}

#[derive(Clone)]
pub(crate) struct CatalogFixture(pub(crate) CharacterStateCatalogSnapshot);

impl CharacterStateCatalogSource for CatalogFixture {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<CharacterStateCatalogSnapshot, CharacterStateSourceError> {
        Ok(self.0.clone())
    }
}

pub(crate) fn catalog() -> CharacterStateCatalog {
    let manifest = manifest();
    CharacterStateCatalogProducer::new()
        .produce(&manifest, &CatalogFixture(catalog_snapshot(&manifest)))
        .expect("catalog")
}

pub(crate) fn owner(kind: CharacterStateOwnerKind, id: &str) -> CharacterStateOwner {
    CharacterStateOwner {
        kind,
        id: CharacterStateOwnerId::new(id).expect("owner id"),
        character_id: "char-a".to_owned(),
        label: CharacterStateField::Available(id.to_owned()),
    }
}

pub(crate) fn live_binding(
    catalog: &CharacterStateCatalog,
    epoch: u64,
) -> CharacterStateLiveBinding {
    CharacterStateLiveBinding {
        catalog: catalog.binding().clone(),
        game_instance_id: "game-1".to_owned(),
        run_id: "run-1".to_owned(),
        mode_id: "standard".to_owned(),
        snapshot_id: format!("snapshot-{epoch}"),
        epoch,
    }
}

pub(crate) fn snapshot(catalog: &CharacterStateCatalog, epoch: u64) -> CharacterStateLiveSnapshot {
    let binding = live_binding(catalog, epoch);
    let mut resources = vec![sts2_game_mod::CharacterResourceInput {
        instance_id: "resource-live-1".to_owned(),
        definition_id: "resource:charge".to_owned(),
        owner: owner(CharacterStateOwnerKind::Player, "player-1"),
        current: CharacterStateField::Available(CharacterResourceValue::Integer {
            value: 2,
            unit: unit("charge"),
        }),
        maximum: CharacterStateField::Available(CharacterResourceValue::Integer {
            value: 3,
            unit: unit("charge"),
        }),
        slots: CharacterStateField::Available(vec![
            CharacterResourceSlot {
                slot_id: "slot-0".to_owned(),
                position: 0,
                content: CharacterStateField::Available(CharacterResourceSlotContent::Definition(
                    "resource:charge".to_owned(),
                )),
            },
            CharacterResourceSlot {
                slot_id: "slot-1".to_owned(),
                position: 1,
                content: CharacterStateField::Available(CharacterResourceSlotContent::Empty),
            },
        ]),
        active: CharacterStateField::Available(true),
    }];
    let mut ally_resource = resources[0].clone();
    ally_resource.instance_id = "resource-live-2".to_owned();
    ally_resource.owner = owner(CharacterStateOwnerKind::Ally, "ally-1");
    resources.push(ally_resource);
    let mut secondary_entities = vec![SecondaryEntityInput {
        instance_id: "entity-live-1".to_owned(),
        definition_id: "entity:orb".to_owned(),
        owner: owner(CharacterStateOwnerKind::Secondary, "orb-owner"),
        controller: CharacterStateField::Available(SecondaryEntityControllerReference {
            owner_kind: CharacterStateOwnerKind::Player,
            owner_id: CharacterStateOwnerId::new("player-1").expect("controller"),
            character_id: "char-a".to_owned(),
            label: CharacterStateField::Available("Player".to_owned()),
        }),
        hp: CharacterStateField::Available(12),
        maximum_hp: CharacterStateField::Available(12),
        block: CharacterStateField::Available(4),
        statuses: CharacterStateField::Available(vec![SecondaryEntityStatus {
            instance_id: "status-live-1".to_owned(),
            definition_id: "status:charged".to_owned(),
            amount: CharacterStateField::Available(2),
            visibility: CharacterStateVisibility::Visible,
            label: CharacterStateField::Available("Charged".to_owned()),
        }]),
        intent: CharacterStateField::Available(SecondaryEntityIntent {
            kind: SecondaryEntityIntentKind::Attack,
            rule_reference: CharacterStateField::Available("rule:orb-attack".to_owned()),
            target: CharacterStateField::Available(SecondaryEntityTarget::Owner(
                "player-1".to_owned(),
            )),
            amount: CharacterStateField::Available(5),
            label: CharacterStateField::Available("Zap".to_owned()),
            visibility: CharacterStateVisibility::Visible,
        }),
        active: CharacterStateField::Available(true),
    }];
    let mut second_entity = secondary_entities[0].clone();
    second_entity.instance_id = "entity-live-2".to_owned();
    second_entity.owner = owner(CharacterStateOwnerKind::Secondary, "orb-owner-2");
    secondary_entities.push(second_entity);
    CharacterStateLiveSnapshot::from_input(CharacterStateLiveSnapshotInput {
        binding,
        resources,
        secondary_entities,
    })
    .expect("snapshot")
}
