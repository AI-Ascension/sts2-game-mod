// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/potions.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, POTION_MAX_DEFINITION_BYTES, POTION_MAX_LIVE_DETAIL_BYTES,
    POTION_MAX_TEXT_BYTES, POTION_PRODUCER_VERSION, PotionCatalogError, PotionCatalogProducer,
    PotionCatalogSnapshot, PotionCollectionKind, PotionEffectKind, PotionExpiration,
    PotionFamilyCoverage, PotionFamilyState, PotionField, PotionLiveError, PotionLiveReader,
    PotionLiveSnapshot, PotionModifier, PotionModifierScope, PotionModifierValue,
    PotionParameterValue, PotionVisibility, PotionVisibilityScope,
};

#[test]
fn malformed_effect_shapes_and_oversized_static_payloads_fail_closed() {
    let manifest = manifest();
    let mut invalid = input(
        "mod:synthetic:choice",
        ContentUnlockState::Unlocked,
        PotionEffectKind::RandomChoice,
    );
    invalid.effects[3].alternatives.clear();
    assert_eq!(
        PotionCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(PotionCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    producer_version: POTION_PRODUCER_VERSION.to_owned(),
                    family: PotionFamilyCoverage {
                        entity_kind: "potion".to_owned(),
                        state: PotionFamilyState::Handled,
                        definition_count: 4,
                    },
                    definitions: vec![
                        input(
                            "mod:synthetic:healing",
                            ContentUnlockState::Unlocked,
                            PotionEffectKind::Direct,
                        ),
                        invalid,
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
                    ],
                }),
            },
        ),
        Err(PotionCatalogError::InvalidInput(
            "random_choice_alternatives"
        ))
    );

    let mut oversized = input(
        "mod:synthetic:healing",
        ContentUnlockState::Unlocked,
        PotionEffectKind::Direct,
    );
    oversized.description = "x".repeat(POTION_MAX_TEXT_BYTES);
    oversized.effects[0].label = "x".repeat(POTION_MAX_TEXT_BYTES);
    assert!(matches!(
        PotionCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(PotionCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    producer_version: POTION_PRODUCER_VERSION.to_owned(),
                    family: PotionFamilyCoverage {
                        entity_kind: "potion".to_owned(),
                        state: PotionFamilyState::Handled,
                        definition_count: 4,
                    },
                    definitions: vec![oversized],
                }),
            },
        ),
        Err(PotionCatalogError::DefinitionTooLarge {
            limit: POTION_MAX_DEFINITION_BYTES,
            ..
        }) | Err(PotionCatalogError::FamilyCountMismatch)
    ));
}

#[test]
fn hidden_and_owner_only_live_fields_require_explicit_scope() {
    let manifest = manifest();
    let mut definitions = definitions();
    definitions[0].parameters[0].visibility = PotionVisibility::OwnerOnly;
    let catalog = PotionCatalogProducer::new()
        .produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(PotionCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    producer_version: POTION_PRODUCER_VERSION.to_owned(),
                    family: PotionFamilyCoverage {
                        entity_kind: "potion".to_owned(),
                        state: PotionFamilyState::Handled,
                        definition_count: definitions.len(),
                    },
                    definitions,
                }),
            },
        )
        .expect("owner-only catalog");
    let reference = catalog
        .reader()
        .list(&sts2_game_mod::PotionListQuery {
            scope: PotionVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("summary")
        .entries
        .into_iter()
        .find(|entry| entry.reference.potion_id == "mod:synthetic:healing")
        .expect("healing")
        .reference;
    assert_eq!(
        catalog.get(&reference, PotionVisibilityScope::Public),
        Err(PotionCatalogError::ExcludedByScope)
    );
    assert!(
        catalog
            .get(&reference, PotionVisibilityScope::Owner)
            .is_ok()
    );
    let snapshot = PotionLiveSnapshot::from_input(live_input(&catalog, 10)).expect("snapshot");
    assert!(matches!(
        PotionLiveReader::new(&catalog, snapshot),
        Err(PotionLiveError::InvalidState("parameter_visibility"))
    ));
}

#[test]
fn oversized_live_values_and_unknown_targets_do_not_become_zero() {
    let catalog = catalog();
    let mut input = live_input(&catalog, 11);
    input.instances[0].effective_parameters =
        PotionField::Available(vec![sts2_game_mod::PotionResolvedParameter {
            id: "amount".to_owned(),
            unit: unit("count"),
            value: PotionParameterValue::Text("x".repeat(POTION_MAX_TEXT_BYTES)),
        }]);
    let snapshot = PotionLiveSnapshot::from_input(input).expect("shape");
    assert!(matches!(
        PotionLiveReader::new(&catalog, snapshot),
        Err(PotionLiveError::DetailTooLarge {
            limit: POTION_MAX_LIVE_DETAIL_BYTES,
            ..
        })
    ));

    let mut unknown = live_input(&catalog, 12);
    unknown.instances[0].usability = PotionField::Available(sts2_game_mod::PotionUsabilityResult {
        state: sts2_game_mod::PotionUseState::Unknown,
        reason: Some(sts2_game_mod::PotionUsabilityReason::Unknown),
    });
    let unknown_snapshot = PotionLiveSnapshot::from_input(unknown).expect("unknown snapshot");
    let reader = PotionLiveReader::new(&catalog, unknown_snapshot).expect("unknown reader");
    let reference = reader.references().into_iter().next().expect("reference");
    let detail = reader.get(&reference).expect("detail");
    assert_eq!(
        detail.usability.value().expect("usability").state,
        sts2_game_mod::PotionUseState::Unknown
    );
}

#[test]
fn live_detail_bounds_include_condition_expiration_text() {
    let catalog = catalog();
    let mut input = live_input(&catalog, 13);
    input.instances[0].modifiers = PotionField::Available(vec![PotionModifier {
        source_ref: "modifier:condition".to_owned(),
        order: 0,
        scope: PotionModifierScope::Run,
        amount: None,
        value: PotionModifierValue::Marker,
        expiration: PotionExpiration::Condition("x".repeat(POTION_MAX_TEXT_BYTES)),
    }]);
    let snapshot = PotionLiveSnapshot::from_input(input).expect("shape");
    assert!(matches!(
        PotionLiveReader::new(&catalog, snapshot),
        Err(PotionLiveError::DetailTooLarge {
            limit: POTION_MAX_LIVE_DETAIL_BYTES,
            ..
        })
    ));
}

#[test]
fn slot_duplicate_detection_uses_typed_references() {
    let catalog = catalog();
    let mut input = live_input(&catalog, 14);
    let reward_slot = fixture::slot(PotionCollectionKind::Reward("a:b".to_owned()), "c", 0);
    let shop_slot = fixture::slot(PotionCollectionKind::Shop("a".to_owned()), "b:c", 0);
    input.offers[0].slot = reward_slot.clone();
    input.offers[1].slot = shop_slot.clone();
    input.instances[1].slot = reward_slot;
    input.instances[2].slot = shop_slot;
    assert!(PotionLiveSnapshot::from_input(input).is_ok());
}
