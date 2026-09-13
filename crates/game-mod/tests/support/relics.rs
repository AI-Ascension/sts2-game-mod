// SPDX-License-Identifier: MIT

use crate::content_fixture::ManifestSource;
use sts2_game_mod::{
    ContentDefinitionInput, ContentManifest, ContentManifestProducer, ContentOriginInput,
    ContentUnlockState, RELIC_PRODUCER_VERSION, RelicAcquisition, RelicAcquisitionRule,
    RelicActivationDefinition, RelicActivationKind, RelicActivationState, RelicCatalog,
    RelicCatalogSnapshot, RelicCatalogSource, RelicCondition, RelicCounterDefinition,
    RelicCounterReset, RelicCounterState, RelicDefinitionInput, RelicFamilyCoverage,
    RelicFamilyState, RelicField, RelicInstanceInput, RelicLiveBinding, RelicOrigin,
    RelicParameterDefinition, RelicPool, RelicRarity, RelicSemanticReference,
    RelicSemanticReferenceKind, RelicTier, RelicTriggerDefinition, RelicUnit, RelicVariant,
    RelicVisibility,
};

#[derive(Clone, Debug)]
pub struct CatalogSource {
    pub snapshot: Result<RelicCatalogSnapshot, sts2_game_mod::RelicSourceError>,
}

impl RelicCatalogSource for CatalogSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<RelicCatalogSnapshot, sts2_game_mod::RelicSourceError> {
        self.snapshot.clone()
    }
}

pub fn manifest() -> ContentManifest {
    let mut snapshot = crate::content_fixture::catalog_snapshot();
    for id in [
        "mod:synthetic:charged",
        "mod:synthetic:conditional",
        "mod:synthetic:turn",
        "mod:synthetic:room",
        "mod:synthetic:multiple",
    ] {
        snapshot.definitions.push(ContentDefinitionInput {
            entity_kind: "relic".to_owned(),
            namespaced_id: id.to_owned(),
            semantic_inputs: format!("kind=relic;id={id}"),
            localized_text: Some(id.to_owned()),
            origin: ContentOriginInput {
                package_id: Some("mod:synthetic".to_owned()),
                package_version: None,
            },
            override_chain: Vec::new(),
        });
    }
    ContentManifestProducer::new("adapter-v1", ["card".to_owned()])
        .expect("manifest producer")
        .produce(&ManifestSource { snapshot })
        .expect("manifest")
}

pub fn unit(value: &str) -> RelicUnit {
    RelicUnit::new(value).expect("unit")
}

pub fn counter(id: &str, label: &str, unit_name: &str) -> RelicCounterDefinition {
    RelicCounterDefinition {
        id: id.to_owned(),
        label: label.to_owned(),
        unit: unit(unit_name),
        visibility: RelicVisibility::Visible,
        reset: RelicCounterReset::Run,
    }
}

pub fn owner() -> sts2_game_mod::RelicOwnerId {
    sts2_game_mod::RelicOwnerId::new("player:1").expect("owner")
}

pub fn input(
    id: &str,
    unlock: ContentUnlockState,
    activation: RelicActivationKind,
    counters: Vec<RelicCounterDefinition>,
) -> RelicDefinitionInput {
    let counter_ids = counters.iter().map(|counter| counter.id.clone()).collect();
    RelicDefinitionInput {
        relic_id: id.to_owned(),
        title: id.rsplit(':').next().expect("title").to_owned(),
        description: "Static rule text remains separate from live amounts.".to_owned(),
        rarity: RelicRarity::new("common").expect("rarity"),
        tier: RelicTier::new("starter").expect("tier"),
        pool: Some(RelicPool::new("synthetic").expect("pool")),
        origin: RelicOrigin {
            kind: "fixture".to_owned(),
            package_id: Some("mod:synthetic".to_owned()),
            package_version: None,
        },
        acquisition: RelicAcquisition {
            rules: vec![RelicAcquisitionRule {
                kind: "reward".to_owned(),
                reference: Some("reward:synthetic".to_owned()),
                requirement: Some("requires:test".to_owned()),
            }],
            unlock: sts2_game_mod::RelicUnlock {
                state: unlock,
                requirements: vec!["profile:fixture".to_owned()],
            },
        },
        references: vec![
            RelicSemanticReference {
                kind: RelicSemanticReferenceKind::Effect,
                id: "effect:fixture".to_owned(),
                label: "Fixture effect".to_owned(),
            },
            RelicSemanticReference {
                kind: RelicSemanticReferenceKind::Keyword,
                id: "keyword:fixture".to_owned(),
                label: "Fixture keyword".to_owned(),
            },
        ],
        variants: vec![RelicVariant {
            id: "default".to_owned(),
            label: "Default".to_owned(),
            description: Some("Default variant".to_owned()),
        }],
        parameters: if activation == RelicActivationKind::Charged {
            vec![RelicParameterDefinition {
                id: "amount".to_owned(),
                label: "Visible amount".to_owned(),
                unit: unit("count"),
                visibility: RelicVisibility::Visible,
            }]
        } else {
            Vec::new()
        },
        counters,
        activation: RelicActivationDefinition {
            kind: activation,
            condition: (activation == RelicActivationKind::Conditional).then(|| RelicCondition {
                id: "condition:fixture".to_owned(),
                label: "When the fixture condition holds".to_owned(),
            }),
            counter_ids,
        },
        triggers: if activation == RelicActivationKind::Multiple {
            vec![RelicTriggerDefinition {
                id: "trigger:fixture".to_owned(),
                label: "Fixture trigger".to_owned(),
                condition: None,
                visibility: RelicVisibility::Visible,
            }]
        } else {
            Vec::new()
        },
    }
}

pub fn catalog() -> RelicCatalog {
    let manifest = manifest();
    let definitions = vec![
        input(
            "mod:synthetic:badge",
            ContentUnlockState::Unlocked,
            RelicActivationKind::Passive,
            Vec::new(),
        ),
        input(
            "mod:synthetic:charged",
            ContentUnlockState::Unlocked,
            RelicActivationKind::Charged,
            vec![RelicCounterDefinition {
                id: "charge".to_owned(),
                label: "Charges".to_owned(),
                unit: unit("count"),
                visibility: RelicVisibility::Visible,
                reset: RelicCounterReset::Activation,
            }],
        ),
        input(
            "mod:synthetic:turn",
            ContentUnlockState::Unlocked,
            RelicActivationKind::TurnCounter,
            vec![RelicCounterDefinition {
                id: "turns".to_owned(),
                label: "Turns".to_owned(),
                unit: unit("turn"),
                visibility: RelicVisibility::Visible,
                reset: RelicCounterReset::Turn,
            }],
        ),
        input(
            "mod:synthetic:conditional",
            ContentUnlockState::Unlocked,
            RelicActivationKind::Conditional,
            vec![RelicCounterDefinition {
                id: "threshold".to_owned(),
                label: "Threshold".to_owned(),
                unit: unit("count"),
                visibility: RelicVisibility::Visible,
                reset: RelicCounterReset::Combat,
            }],
        ),
        input(
            "mod:synthetic:room",
            ContentUnlockState::Unlocked,
            RelicActivationKind::RoomCounter,
            vec![RelicCounterDefinition {
                id: "rooms".to_owned(),
                label: "Rooms".to_owned(),
                unit: unit("room"),
                visibility: RelicVisibility::Visible,
                reset: RelicCounterReset::Room,
            }],
        ),
        input(
            "mod:synthetic:multiple",
            ContentUnlockState::Locked,
            RelicActivationKind::Multiple,
            vec![
                RelicCounterDefinition {
                    id: "first".to_owned(),
                    label: "First".to_owned(),
                    unit: unit("count"),
                    visibility: RelicVisibility::Visible,
                    reset: RelicCounterReset::Run,
                },
                RelicCounterDefinition {
                    id: "second".to_owned(),
                    label: "Second".to_owned(),
                    unit: unit("count"),
                    visibility: RelicVisibility::Visible,
                    reset: RelicCounterReset::Turn,
                },
            ],
        ),
    ];
    let source = CatalogSource {
        snapshot: Ok(RelicCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: RELIC_PRODUCER_VERSION.to_owned(),
            family: RelicFamilyCoverage {
                entity_kind: "relic".to_owned(),
                state: RelicFamilyState::Handled,
                definition_count: definitions.len(),
            },
            definitions,
        }),
    };
    sts2_game_mod::RelicCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog")
}

pub fn binding(catalog: &RelicCatalog, epoch: u64) -> RelicLiveBinding {
    RelicLiveBinding {
        catalog: catalog.binding().clone(),
        game_instance_id: "game:1".to_owned(),
        run_id: "run:1".to_owned(),
        snapshot_id: format!("snapshot:{epoch}"),
        epoch,
    }
}

pub fn instance(
    id: &str,
    definition_id: &str,
    counters: RelicField<Vec<RelicCounterState>>,
    activation: RelicField<RelicActivationState>,
) -> RelicInstanceInput {
    RelicInstanceInput {
        instance_id: id.to_owned(),
        definition_id: definition_id.to_owned(),
        owner_id: owner(),
        counters,
        activation,
        accumulated: RelicField::Available(Vec::new()),
        pending_triggers: RelicField::Available(Vec::new()),
        resolved_parameters: RelicField::Available(Vec::new()),
    }
}
