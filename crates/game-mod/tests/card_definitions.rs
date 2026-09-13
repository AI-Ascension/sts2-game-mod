// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/card_definitions.rs"]
mod fixture;

use fixture::{CardSource, base_variant, card_input, manifest, manifest_source, snapshot, text};
use sts2_game_mod::{
    CardDefinitionError, CardDefinitionInputError, CardDefinitionProducer, CardDefinitionSnapshot,
    CardDefinitionSourceError, CardEffectValue, CardNumericValue, CardUnavailableReason,
    CardVariantKind, ContentManifestProducer,
};

#[test]
fn producer_covers_all_manifest_cards_and_keeps_provenance_read_only() {
    let ids = [
        "card:ordinary",
        "card:x-cost",
        "card:multiple-upgrade",
        "card:generated",
        "card:duplicate-a",
        "card:duplicate-b",
        "card:dynamic",
    ];
    let manifest = manifest(&ids);
    let source = CardSource {
        snapshot: Ok(snapshot(&manifest)),
    };
    let before = source.clone();
    let catalog = CardDefinitionProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");

    assert_eq!(catalog.definitions().count(), ids.len());
    assert_eq!(source, before);
    let duplicate_names = catalog
        .definitions()
        .filter(|definition| {
            definition
                .variants
                .iter()
                .any(|variant| variant.title == text("Duplicate"))
        })
        .count();
    assert_eq!(duplicate_names, 2);
    let ordinary = catalog
        .definitions()
        .find(|definition| definition.reference.namespaced_id == "card:ordinary")
        .expect("ordinary");
    assert_eq!(
        ordinary.provenance.origin.package_id.as_deref(),
        Some("base:synthetic")
    );
    assert_eq!(ordinary.upgrade_paths[0].variant_ids, vec!["upgrade"]);
    let multiple = catalog
        .definitions()
        .find(|definition| definition.reference.namespaced_id == "card:multiple-upgrade")
        .expect("multiple upgrade");
    assert_eq!(multiple.upgrade_paths.len(), 2);
    assert_eq!(multiple.variants[1].upgrade_level, Some(1));
    assert_eq!(multiple.variants[2].upgrade_level, Some(2));
    assert_eq!(multiple.variants[3].upgrade_level, Some(1));
    let generated = catalog
        .definitions()
        .find(|definition| definition.reference.namespaced_id == "card:generated")
        .expect("generated");
    assert!(
        generated
            .variants
            .iter()
            .any(|variant| variant.kind == CardVariantKind::Generated)
    );
}

#[test]
fn compare_reports_changed_text_cost_effects_and_structure() {
    let manifest = manifest(&["card:ordinary"]);
    let catalog = CardDefinitionProducer::new()
        .produce(
            &manifest,
            &CardSource {
                snapshot: Ok(snapshot(&manifest)),
            },
        )
        .expect("catalog");
    let definition = catalog.definitions().next().expect("definition");
    let base = sts2_game_mod::CardVariantReference {
        card: definition.reference.clone(),
        variant_id: "base".to_owned(),
    };
    let upgrade = sts2_game_mod::CardVariantReference {
        card: definition.reference.clone(),
        variant_id: "upgrade".to_owned(),
    };
    let comparison = catalog.compare(&base, &upgrade).expect("comparison");
    assert!(comparison.title.is_some());
    assert!(comparison.description.is_some());
    assert!(comparison.cost.is_some());
    assert!(comparison.effects.is_some());
    assert!(comparison.structural_modifiers.is_some());
}

#[test]
fn formulas_and_unavailable_values_remain_explicit() {
    let manifest = manifest(&["card:x-cost", "card:dynamic"]);
    let catalog = CardDefinitionProducer::new()
        .produce(
            &manifest,
            &CardSource {
                snapshot: Ok(snapshot(&manifest)),
            },
        )
        .expect("catalog");
    let x_cost = catalog
        .definitions()
        .find(|definition| definition.reference.namespaced_id == "card:x-cost")
        .expect("x cost");
    assert!(x_cost.variants[0].cost.x_cost);
    assert!(matches!(
        x_cost.variants[0].cost.energy,
        CardNumericValue::Formula(_)
    ));
    let dynamic = catalog
        .definitions()
        .find(|definition| definition.reference.namespaced_id == "card:dynamic")
        .expect("dynamic");
    assert!(dynamic.variants[0].effects.iter().any(|effect| {
        matches!(
            effect.value,
            CardEffectValue::Unavailable(CardUnavailableReason::FormulaNotObserved)
        )
    }));
}

#[test]
fn source_snapshot_and_references_fail_closed() {
    let first_manifest = manifest(&["card:ordinary"]);
    assert_eq!(
        CardDefinitionProducer::new().produce(
            &first_manifest,
            &CardSource {
                snapshot: Err(CardDefinitionSourceError::NoActiveSource),
            }
        ),
        Err(CardDefinitionError::Source(
            CardDefinitionSourceError::NoActiveSource
        ))
    );
    let mut missing = snapshot(&first_manifest);
    missing.definitions.clear();
    assert_eq!(
        CardDefinitionProducer::new().produce(
            &first_manifest,
            &CardSource {
                snapshot: Ok(missing),
            }
        ),
        Err(CardDefinitionError::MissingDefinition {
            namespaced_id: "card:ordinary".to_owned()
        })
    );

    let other_manifest = manifest(&["card:other"]);
    let catalog = CardDefinitionProducer::new()
        .produce(
            &first_manifest,
            &CardSource {
                snapshot: Ok(snapshot(&first_manifest)),
            },
        )
        .expect("catalog");
    let reference = sts2_game_mod::CardVariantReference {
        card: sts2_game_mod::ContentDefinitionReference {
            manifest: other_manifest.cursor_binding(),
            entity_kind: "card".to_owned(),
            namespaced_id: "card:ordinary".to_owned(),
        },
        variant_id: "base".to_owned(),
    };
    assert_eq!(
        catalog.variant(&reference),
        Err(CardDefinitionError::StaleReference)
    );
    let mut wrong_kind = reference;
    wrong_kind.card.manifest = first_manifest.cursor_binding();
    wrong_kind.card.entity_kind = "relic".to_owned();
    assert_eq!(
        catalog.variant(&wrong_kind),
        Err(CardDefinitionError::NotFound)
    );
}

#[test]
fn unsupported_family_and_oversized_variants_are_rejected() {
    let source = manifest_source(&["card:ordinary"]);
    let unsupported_manifest = ContentManifestProducer::new("adapter-v1", Vec::<String>::new())
        .expect("producer")
        .produce(&source)
        .expect("manifest");
    assert_eq!(
        CardDefinitionProducer::new().produce(
            &unsupported_manifest,
            &CardSource {
                snapshot: Ok(snapshot(&unsupported_manifest)),
            }
        ),
        Err(CardDefinitionError::UnsupportedCardFamily)
    );

    let manifest = manifest(&["card:ordinary"]);
    let mut oversized = card_input("card:ordinary", &["base"]);
    oversized.variants.extend(
        (0..70)
            .map(|index| {
                let mut variant = base_variant(&format!("variant-{index}"), "Too many", 1);
                variant.kind = CardVariantKind::Alternate;
                variant
            })
            .collect::<Vec<_>>(),
    );
    let mut snapshot = snapshot(&manifest);
    snapshot.definitions = vec![oversized];
    assert!(matches!(
        CardDefinitionProducer::new().produce(
            &manifest,
            &CardSource {
                snapshot: Ok(snapshot),
            }
        ),
        Err(CardDefinitionError::InvalidInput(_))
    ));
}

#[test]
fn upgrade_paths_require_strictly_increasing_levels() {
    let manifest = manifest(&["card:ordinary"]);
    let mut reversed = card_input("card:ordinary", &["base", "upgrade-1", "upgrade-2"]);
    reversed.upgrade_paths[0].variant_ids.reverse();
    assert_eq!(
        CardDefinitionProducer::new().produce(
            &manifest,
            &CardSource {
                snapshot: Ok(CardDefinitionSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    definitions: vec![reversed],
                }),
            }
        ),
        Err(CardDefinitionError::InvalidInput(
            CardDefinitionInputError::NonIncreasingUpgradeLevel
        ))
    );

    let mut duplicate = card_input("card:ordinary", &["base", "upgrade-1", "upgrade-2"]);
    duplicate.variants[2].upgrade_level = Some(1);
    assert_eq!(
        CardDefinitionProducer::new().produce(
            &manifest,
            &CardSource {
                snapshot: Ok(CardDefinitionSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    definitions: vec![duplicate],
                }),
            }
        ),
        Err(CardDefinitionError::InvalidInput(
            CardDefinitionInputError::NonIncreasingUpgradeLevel
        ))
    );
}
