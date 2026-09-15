// SPDX-License-Identifier: MIT

#[path = "potion_manifest.rs"]
mod potion_manifest;
pub use potion_manifest::manifest;

use sts2_game_mod::{
    ContentManifest, ContentUnlockState, PotionAcquisition, PotionAcquisitionRule, PotionCatalog,
    PotionCatalogSnapshot, PotionCatalogSource, PotionCondition, PotionDefinitionInput,
    PotionEffect, PotionEffectAlternative, PotionEffectKind, PotionEffectMagnitude,
    PotionFamilyCoverage, PotionFamilyState, PotionField, PotionInstanceInput, PotionLiveBinding,
    PotionLiveSnapshotInput, PotionModifier, PotionModifierScope, PotionModifierValue,
    PotionOfferInput, PotionOfferKind, PotionOrigin, PotionParameterDefinition,
    PotionParameterValue, PotionPool, PotionPrice, PotionRarity, PotionSemanticReference,
    PotionSemanticReferenceKind, PotionSlotReference, PotionSlotState, PotionTargetKind,
    PotionTargetMode, PotionTargetReference, PotionUnit, PotionUnlock, PotionUsabilityResult,
    PotionUseRule, PotionUseState, PotionVisibility,
};

#[derive(Clone, Debug)]
pub struct CatalogSource {
    pub snapshot: Result<PotionCatalogSnapshot, sts2_game_mod::PotionSourceError>,
}

impl PotionCatalogSource for CatalogSource {
    fn read_catalog(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<PotionCatalogSnapshot, sts2_game_mod::PotionSourceError> {
        self.snapshot.clone()
    }
}

#[derive(Clone, Debug)]
pub struct LiveSource {
    pub snapshot: Result<PotionLiveSnapshotInput, sts2_game_mod::PotionSourceError>,
}

impl sts2_game_mod::PotionLiveSource for LiveSource {
    fn read_live(
        &self,
        _expected: &PotionLiveBinding,
    ) -> Result<PotionLiveSnapshotInput, sts2_game_mod::PotionSourceError> {
        self.snapshot.clone()
    }
}

pub fn unit(value: &str) -> PotionUnit {
    PotionUnit::new(value).expect("unit")
}

pub fn slot(
    collection: sts2_game_mod::PotionCollectionKind,
    id: &str,
    index: u16,
) -> PotionSlotReference {
    PotionSlotReference {
        collection,
        slot_id: id.to_owned(),
        index,
    }
}

pub fn owner(value: &str) -> sts2_game_mod::PotionOwnerId {
    sts2_game_mod::PotionOwnerId::new(value).expect("owner")
}

fn direct_effect(
    id: &str,
    label: &str,
    amount: PotionField<PotionEffectMagnitude>,
) -> PotionEffect {
    PotionEffect {
        id: id.to_owned(),
        label: label.to_owned(),
        kind: PotionEffectKind::Direct,
        target: PotionTargetMode::SelfPlayer,
        magnitude: amount,
        condition: None,
        alternatives: Vec::new(),
        references: vec![PotionSemanticReference {
            kind: PotionSemanticReferenceKind::Effect,
            id: format!("effect:{id}"),
            label: label.to_owned(),
        }],
        visibility: PotionVisibility::Visible,
    }
}

pub fn input(
    id: &str,
    unlock: ContentUnlockState,
    kind: PotionEffectKind,
) -> PotionDefinitionInput {
    let mut effects = vec![direct_effect(
        "base",
        "Base amount",
        PotionField::Available(PotionEffectMagnitude::Fixed(5)),
    )];
    let target_mode = if id.ends_with("choice") {
        effects.push(direct_effect(
            "damage",
            "Damage",
            PotionField::Available(PotionEffectMagnitude::Fixed(10)),
        ));
        effects.push(direct_effect(
            "block",
            "Block",
            PotionField::Available(PotionEffectMagnitude::Fixed(8)),
        ));
        effects.push(PotionEffect {
            id: "choice".to_owned(),
            label: "Random choice".to_owned(),
            kind: PotionEffectKind::RandomChoice,
            target: PotionTargetMode::SingleEnemy,
            magnitude: PotionField::Unknown,
            condition: None,
            alternatives: vec![
                PotionEffectAlternative {
                    id: "damage-choice".to_owned(),
                    label: "Damage".to_owned(),
                    effect_ids: vec!["damage".to_owned()],
                    rule_reference: Some("rule:random".to_owned()),
                },
                PotionEffectAlternative {
                    id: "block-choice".to_owned(),
                    label: "Block".to_owned(),
                    effect_ids: vec!["block".to_owned()],
                    rule_reference: Some("rule:random".to_owned()),
                },
            ],
            references: vec![PotionSemanticReference {
                kind: PotionSemanticReferenceKind::Rule,
                id: "rule:random".to_owned(),
                label: "Game-selected alternative".to_owned(),
            }],
            visibility: PotionVisibility::Visible,
        });
        PotionTargetMode::SingleEnemy
    } else if id.ends_with("conditional") {
        effects.push(PotionEffect {
            id: "conditional".to_owned(),
            label: "Conditional amount".to_owned(),
            kind: PotionEffectKind::Conditional,
            target: PotionTargetMode::SelfPlayer,
            magnitude: PotionField::Available(PotionEffectMagnitude::Formula(
                "if condition:fixture then 12 else 0".to_owned(),
            )),
            condition: Some(PotionCondition {
                id: "condition:fixture".to_owned(),
                label: "When fixture condition holds".to_owned(),
            }),
            alternatives: vec![],
            references: vec![],
            visibility: PotionVisibility::Visible,
        });
        PotionTargetMode::SelfPlayer
    } else {
        PotionTargetMode::SelfPlayer
    };
    if kind == PotionEffectKind::Multiple {
        effects[0].kind = PotionEffectKind::Multiple;
        effects[0].alternatives = vec![PotionEffectAlternative {
            id: "base-branch".to_owned(),
            label: "Base branch".to_owned(),
            effect_ids: vec!["base".to_owned()],
            rule_reference: None,
        }];
    }
    PotionDefinitionInput {
        potion_id: id.to_owned(),
        title: id.rsplit(':').next().expect("title").to_owned(),
        description: "Static effect structure remains separate from live values.".to_owned(),
        rarity: PotionRarity::new("common").expect("rarity"),
        pool: Some(PotionPool::new("synthetic").expect("pool")),
        origin: PotionOrigin {
            kind: "fixture".to_owned(),
            package_id: Some("mod:synthetic".to_owned()),
            package_version: None,
        },
        acquisition: PotionAcquisition {
            rules: vec![
                PotionAcquisitionRule {
                    kind: "reward".to_owned(),
                    reference: Some("reward:synthetic".to_owned()),
                    requirement: Some("requires:test".to_owned()),
                },
                PotionAcquisitionRule {
                    kind: "shop".to_owned(),
                    reference: Some("shop:synthetic".to_owned()),
                    requirement: None,
                },
            ],
            unlock: PotionUnlock {
                state: unlock,
                requirements: vec!["profile:fixture".to_owned()],
            },
        },
        target_mode,
        use_rule: if id.ends_with("conditional") {
            PotionUseRule::Conditional("condition:fixture".to_owned())
        } else {
            PotionUseRule::Anytime
        },
        effects,
        parameters: vec![PotionParameterDefinition {
            id: "amount".to_owned(),
            label: "Effective amount".to_owned(),
            unit: unit("count"),
            visibility: PotionVisibility::Visible,
        }],
        references: vec![PotionSemanticReference {
            kind: PotionSemanticReferenceKind::Keyword,
            id: "keyword:fixture".to_owned(),
            label: "Fixture keyword".to_owned(),
        }],
    }
}

pub fn definitions() -> Vec<PotionDefinitionInput> {
    vec![
        input(
            "mod:synthetic:healing",
            ContentUnlockState::Unlocked,
            PotionEffectKind::Direct,
        ),
        input(
            "mod:synthetic:choice",
            ContentUnlockState::Unlocked,
            PotionEffectKind::RandomChoice,
        ),
        input(
            "mod:synthetic:conditional",
            ContentUnlockState::Unlocked,
            PotionEffectKind::Conditional,
        ),
        input(
            "mod:synthetic:locked",
            ContentUnlockState::Locked,
            PotionEffectKind::Direct,
        ),
    ]
}

pub fn catalog() -> PotionCatalog {
    let manifest = manifest();
    let definitions = definitions();
    sts2_game_mod::PotionCatalogProducer::new()
        .produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(PotionCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    producer_version: sts2_game_mod::POTION_PRODUCER_VERSION.to_owned(),
                    family: PotionFamilyCoverage {
                        entity_kind: "potion".to_owned(),
                        state: PotionFamilyState::Handled,
                        definition_count: definitions.len(),
                    },
                    definitions,
                }),
            },
        )
        .expect("catalog")
}

pub fn binding(catalog: &PotionCatalog, epoch: u64) -> PotionLiveBinding {
    PotionLiveBinding {
        catalog: catalog.binding().clone(),
        game_instance_id: "game:1".to_owned(),
        run_id: "run:1".to_owned(),
        snapshot_id: format!("snapshot:{epoch}"),
        epoch,
    }
}

pub fn inventory_slot(index: u16) -> PotionSlotReference {
    slot(
        sts2_game_mod::PotionCollectionKind::Inventory,
        &format!("slot:{index}"),
        index,
    )
}

pub fn instance(
    id: &str,
    definition_id: &str,
    slot: PotionSlotReference,
    owner_id: &str,
) -> PotionInstanceInput {
    PotionInstanceInput {
        instance_id: id.to_owned(),
        definition_id: definition_id.to_owned(),
        owner_id: owner(owner_id),
        slot,
        usability: PotionField::Available(PotionUsabilityResult {
            state: PotionUseState::Usable,
            reason: None,
        }),
        modifiers: PotionField::Available(Vec::new()),
        effective_parameters: PotionField::Available(vec![
            sts2_game_mod::PotionResolvedParameter {
                id: "amount".to_owned(),
                unit: unit("count"),
                value: PotionParameterValue::Integer(5),
            },
        ]),
        permitted_targets: PotionField::Available(Vec::new()),
    }
}

pub fn live_input(catalog: &PotionCatalog, epoch: u64) -> PotionLiveSnapshotInput {
    let occupied_inventory_slot = inventory_slot(0);
    let empty_slot = inventory_slot(1);
    let reward_slot = slot(
        sts2_game_mod::PotionCollectionKind::Reward("reward:1".to_owned()),
        "reward-slot:0",
        0,
    );
    let shop_slot = slot(
        sts2_game_mod::PotionCollectionKind::Shop("shop:1".to_owned()),
        "shop-slot:0",
        0,
    );
    let mut healing = instance(
        "instance:healing",
        "mod:synthetic:healing",
        occupied_inventory_slot.clone(),
        "player:1",
    );
    healing.modifiers = PotionField::Available(vec![PotionModifier {
        source_ref: "modifier:relic".to_owned(),
        order: 0,
        scope: PotionModifierScope::Run,
        amount: Some(2),
        value: PotionModifierValue::Integer(2),
        expiration: sts2_game_mod::PotionExpiration::Permanent,
    }]);
    let mut reward = instance(
        "instance:reward",
        "mod:synthetic:choice",
        reward_slot.clone(),
        "offer:reward",
    );
    reward.permitted_targets = PotionField::Available(vec![PotionTargetReference {
        target_id: "enemy:1".to_owned(),
        kind: PotionTargetKind::Enemy,
        label: Some("Fixture enemy".to_owned()),
    }]);
    let shop = instance(
        "instance:shop",
        "mod:synthetic:conditional",
        shop_slot.clone(),
        "offer:shop",
    );
    PotionLiveSnapshotInput {
        binding: binding(catalog, epoch),
        inventory: sts2_game_mod::PotionInventoryInput {
            max_slots: PotionField::Available(3),
            slots: vec![
                PotionSlotState::Occupied {
                    slot: occupied_inventory_slot,
                    instance_id: "instance:healing".to_owned(),
                },
                PotionSlotState::Empty { slot: empty_slot },
            ],
        },
        offers: vec![
            PotionOfferInput {
                offer_id: "offer:reward".to_owned(),
                kind: PotionOfferKind::Reward,
                slot: reward_slot,
                instance_id: "instance:reward".to_owned(),
                price: PotionField::NotApplicable,
            },
            PotionOfferInput {
                offer_id: "offer:shop".to_owned(),
                kind: PotionOfferKind::Shop,
                slot: shop_slot,
                instance_id: "instance:shop".to_owned(),
                price: PotionField::Available(PotionPrice {
                    currency: "gold".to_owned(),
                    amount: PotionField::Available(30),
                }),
            },
        ],
        instances: vec![healing, reward, shop],
    }
}
