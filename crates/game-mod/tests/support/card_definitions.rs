// SPDX-License-Identifier: MIT

use sts2_game_mod::{
    CardAcquisitionKind, CardAcquisitionRule, CardCost, CardDefinitionInput,
    CardDefinitionSnapshot, CardDefinitionSource, CardDefinitionSourceError, CardEffectParameter,
    CardEffectValue, CardFormula, CardNumericValue, CardOptionalText, CardRarity,
    CardRuleCondition, CardRuleProvenance, CardStructuralModifier, CardTargeting, CardTextValue,
    CardType, CardUnavailableReason, CardUpgradePath, CardVariantInput, CardVariantKind,
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifest,
    ContentManifestProducer, ContentOriginInput, ContentPackageInput,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CardSource {
    pub snapshot: Result<CardDefinitionSnapshot, CardDefinitionSourceError>,
}

impl CardDefinitionSource for CardSource {
    fn read_definitions(
        &self,
        _manifest: &ContentManifest,
    ) -> Result<CardDefinitionSnapshot, CardDefinitionSourceError> {
        self.snapshot.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestSource {
    snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for ManifestSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, sts2_game_mod::ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

pub fn manifest_source(card_ids: &[&str]) -> ManifestSource {
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
            available_entity_kinds: vec!["card".to_owned()],
            definitions: card_ids
                .iter()
                .map(|id| ContentDefinitionInput {
                    entity_kind: "card".to_owned(),
                    namespaced_id: (*id).to_owned(),
                    semantic_inputs: format!("card={id}"),
                    localized_text: Some((*id).to_owned()),
                    origin: ContentOriginInput {
                        package_id: Some("base:synthetic".to_owned()),
                        package_version: Some("1".to_owned()),
                    },
                    override_chain: Vec::new(),
                })
                .collect(),
        },
    }
}

pub fn manifest(card_ids: &[&str]) -> ContentManifest {
    ContentManifestProducer::new("adapter-v1", [String::from("card")])
        .expect("producer")
        .produce(&manifest_source(card_ids))
        .expect("manifest")
}

pub fn text(value: &str) -> CardTextValue {
    CardTextValue::available(value).expect("text")
}

fn formula(rule: &str, inputs: &[&str]) -> CardFormula {
    CardFormula::new(
        rule,
        inputs
            .iter()
            .map(|input| (*input).to_owned())
            .collect::<Vec<_>>(),
    )
    .expect("formula")
}

fn cost(energy: CardNumericValue) -> CardCost {
    CardCost {
        energy,
        x_cost: false,
        additional: Vec::new(),
    }
}

fn provenance() -> CardRuleProvenance {
    CardRuleProvenance::new("synthetic_catalog", "fixture-v1").expect("provenance")
}

pub fn base_variant(id: &str, title: &str, energy: i32) -> CardVariantInput {
    CardVariantInput {
        variant_id: id.to_owned(),
        kind: CardVariantKind::Base,
        upgrade_level: None,
        title: text(title),
        description: text("Deal damage."),
        cost: cost(CardNumericValue::Fixed(energy)),
        targeting: CardTargeting::SingleEnemy,
        keywords: vec!["attack".to_owned()],
        effects: vec![CardEffectParameter {
            key: "damage".to_owned(),
            value: CardEffectValue::Number(CardNumericValue::Fixed(6)),
        }],
        structural_modifiers: Vec::new(),
    }
}

pub fn card_input(id: &str, variant_ids: &[&str]) -> CardDefinitionInput {
    let mut variants = vec![base_variant(variant_ids[0], id, 1)];
    for (level, variant_id) in variant_ids.iter().skip(1).enumerate() {
        variants.push(CardVariantInput {
            variant_id: (*variant_id).to_owned(),
            kind: CardVariantKind::Upgrade,
            upgrade_level: Some((level + 1) as u16),
            title: text("Upgraded"),
            description: text("Deal more damage."),
            cost: cost(CardNumericValue::Fixed(0)),
            targeting: CardTargeting::SingleEnemy,
            keywords: vec!["attack".to_owned(), "upgraded".to_owned()],
            effects: vec![CardEffectParameter {
                key: "damage".to_owned(),
                value: CardEffectValue::Number(CardNumericValue::Fixed(9 + level as i32)),
            }],
            structural_modifiers: vec![CardStructuralModifier {
                key: "retains".to_owned(),
                value: Some(CardEffectValue::Boolean(true)),
            }],
        });
    }
    CardDefinitionInput {
        namespaced_id: id.to_owned(),
        card_type: CardType::new("attack").expect("type"),
        rarity: CardRarity::new("common").expect("rarity"),
        character_or_pool: CardOptionalText::available("ironclad").expect("pool"),
        variants,
        upgrade_paths: if variant_ids.len() > 1 {
            vec![CardUpgradePath {
                path_id: "standard".to_owned(),
                variant_ids: variant_ids[1..].iter().map(|id| (*id).to_owned()).collect(),
            }]
        } else {
            Vec::new()
        },
        acquisition: vec![CardAcquisitionRule {
            kind: CardAcquisitionKind::CharacterPool,
            condition: CardRuleCondition::Always,
            provenance: provenance(),
        }],
        unlock: None,
    }
}

pub fn snapshot(manifest: &ContentManifest) -> CardDefinitionSnapshot {
    let mut ordinary = card_input("card:ordinary", &["base", "upgrade"]);
    ordinary.variants[1].title = text("Ordinary+");
    ordinary.variants[1].description = text("Deal nine damage.");
    ordinary.variants[1].effects[0].value = CardEffectValue::Number(CardNumericValue::Fixed(9));

    let mut x_cost = card_input("card:x-cost", &["base"]);
    x_cost.card_type = CardType::new("skill").expect("type");
    x_cost.variants[0].cost = CardCost {
        energy: CardNumericValue::Formula(formula("x_energy", &["energy_input"])),
        x_cost: true,
        additional: Vec::new(),
    };
    x_cost.variants[0].effects[0].value = CardEffectValue::Number(CardNumericValue::Formula(
        formula("x_damage", &["energy_input"]),
    ));

    let mut multiple_upgrade = card_input(
        "card:multiple-upgrade",
        &["base", "path-a-1", "path-a-2", "path-b-1"],
    );
    multiple_upgrade.variants[3].upgrade_level = Some(1);
    multiple_upgrade.upgrade_paths = vec![
        CardUpgradePath {
            path_id: "path-a".to_owned(),
            variant_ids: vec!["path-a-1".to_owned(), "path-a-2".to_owned()],
        },
        CardUpgradePath {
            path_id: "path-b".to_owned(),
            variant_ids: vec!["path-b-1".to_owned()],
        },
    ];
    let mut generated = card_input("card:generated", &["base"]);
    generated.character_or_pool = CardOptionalText::not_applicable();
    generated.variants.push(CardVariantInput {
        variant_id: "generated-copy".to_owned(),
        kind: CardVariantKind::Generated,
        upgrade_level: None,
        title: text("Generated Copy"),
        description: text("Generated by another effect."),
        cost: cost(CardNumericValue::Fixed(0)),
        targeting: CardTargeting::None,
        keywords: vec!["generated".to_owned()],
        effects: vec![CardEffectParameter {
            key: "source".to_owned(),
            value: CardEffectValue::Text("runtime_rule".to_owned()),
        }],
        structural_modifiers: Vec::new(),
    });
    generated.acquisition[0].kind = CardAcquisitionKind::Generated;

    let mut dynamic = card_input("card:dynamic", &["base"]);
    dynamic.variants[0].effects.push(CardEffectParameter {
        key: "missing_formula".to_owned(),
        value: CardEffectValue::Unavailable(CardUnavailableReason::FormulaNotObserved),
    });
    dynamic.unlock = Some(sts2_game_mod::CardUnlockRule {
        condition: CardRuleCondition::Formula(formula("unlock_rule", &["profile_flag"])),
        provenance: provenance(),
    });

    let mut definitions = vec![
        ordinary,
        x_cost,
        multiple_upgrade,
        generated,
        card_input("card:duplicate-a", &["base"]),
        card_input("card:duplicate-b", &["base"]),
        dynamic,
    ];
    for definition in &mut definitions {
        if matches!(
            definition.namespaced_id.as_str(),
            "card:duplicate-a" | "card:duplicate-b"
        ) {
            definition.variants[0].title = text("Duplicate");
        }
    }
    let manifest_ids = manifest
        .definitions
        .iter()
        .filter(|definition| definition.entity_kind == "card")
        .map(|definition| definition.namespaced_id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    definitions.retain(|definition| manifest_ids.contains(definition.namespaced_id.as_str()));
    CardDefinitionSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        definitions,
    }
}
