// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/potions.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, PotionCatalogError, PotionCatalogProducer, PotionCatalogSnapshot,
    PotionDefinitionReference, PotionEffectKind, PotionFamilyCoverage, PotionFamilyState,
    PotionField, PotionListQuery, PotionLiveError, PotionLiveReader, PotionLiveSnapshot,
    PotionSourceError, PotionVisibilityScope,
};

#[test]
fn definitions_expose_structured_effects_and_fenced_reference_search() {
    let catalog = catalog();
    let mut reader = catalog.reader();
    let first = reader
        .list(&PotionListQuery {
            scope: PotionVisibilityScope::Public,
            limit: 2,
            continuation: None,
        })
        .expect("first page");
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&PotionListQuery {
            scope: PotionVisibilityScope::Public,
            limit: 2,
            continuation: Some(continuation.clone()),
        })
        .expect("second page");
    assert!(second.complete);
    assert_eq!(
        reader.list(&PotionListQuery {
            scope: PotionVisibilityScope::Public,
            limit: 2,
            continuation: Some(continuation),
        }),
        Err(PotionCatalogError::InvalidContinuation)
    );

    let reference_page = catalog
        .reader()
        .list(&PotionListQuery {
            scope: PotionVisibilityScope::Reference,
            limit: 8,
            continuation: None,
        })
        .expect("reference page");
    assert_eq!(reference_page.total, 4);
    let locked = reference_page
        .entries
        .iter()
        .find(|entry| entry.unlock_state == ContentUnlockState::Locked)
        .expect("locked reference");
    let locked_detail = catalog
        .get(&locked.reference, PotionVisibilityScope::Reference)
        .expect("locked detail");
    assert_eq!(locked_detail.acquisition.rules[0].kind, "reward");

    let choice = catalog
        .get(
            &catalog
                .reader()
                .list(&PotionListQuery {
                    scope: PotionVisibilityScope::Public,
                    limit: 8,
                    continuation: None,
                })
                .expect("all")
                .entries
                .into_iter()
                .find(|entry| entry.reference.potion_id == "mod:synthetic:choice")
                .expect("choice")
                .reference,
            PotionVisibilityScope::Public,
        )
        .expect("choice detail");
    let choice_effect = choice
        .effects
        .iter()
        .find(|effect| effect.id == "choice")
        .expect("choice effect");
    assert_eq!(choice_effect.kind, PotionEffectKind::RandomChoice);
    assert_eq!(choice_effect.alternatives.len(), 2);
    assert!(matches!(choice_effect.magnitude, PotionField::Unknown));
}

#[test]
fn live_state_keeps_slots_offers_effective_values_and_targets_distinct() {
    let catalog = catalog();
    let snapshot = PotionLiveSnapshot::from_input(live_input(&catalog, 7)).expect("snapshot");
    let reader = PotionLiveReader::new(&catalog, snapshot).expect("reader");
    assert_eq!(reader.references().len(), 3);
    assert_eq!(reader.inventory().slots.len(), 2);
    assert_eq!(reader.inventory().max_slots.value(), Some(&3));
    assert_eq!(reader.reward_offers().len(), 1);
    assert_eq!(reader.shop_offers().len(), 1);

    let healing_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:healing")
        .expect("healing reference");
    assert_ne!(
        healing_reference.instance_id, healing_reference.slot.slot_id,
        "slot identity must remain distinct from instance identity"
    );
    let healing = reader.get(&healing_reference).expect("healing detail");
    assert_eq!(
        healing
            .effective_parameters
            .value()
            .expect("effective parameters")[0]
            .value,
        sts2_game_mod::PotionParameterValue::Integer(5)
    );
    assert_eq!(
        healing.modifiers.value().expect("modifier")[0].amount,
        Some(2)
    );
    assert_eq!(
        reader.get(&healing_reference).expect("repeat read"),
        healing
    );

    let reward_reference = reader
        .references()
        .into_iter()
        .find(|reference| reference.instance_id == "instance:reward")
        .expect("reward reference");
    let reward = reader.get(&reward_reference).expect("reward detail");
    assert_eq!(
        reward.permitted_targets.value().expect("target")[0].target_id,
        "enemy:1"
    );
}

#[test]
fn live_fences_expire_references_and_reject_run_or_epoch_reuse() {
    let catalog = catalog();
    let snapshot = PotionLiveSnapshot::from_input(live_input(&catalog, 1)).expect("snapshot");
    let mut reader = PotionLiveReader::new(&catalog, snapshot).expect("reader");
    let old_reference = reader.references().into_iter().next().expect("reference");

    let next = PotionLiveSnapshot::from_input(live_input(&catalog, 2)).expect("next");
    reader.replace_snapshot(next).expect("replace");
    assert_eq!(
        reader.get(&old_reference),
        Err(PotionLiveError::StaleReference)
    );

    let same_epoch = PotionLiveSnapshot::from_input(live_input(&catalog, 2)).expect("same epoch");
    assert_eq!(
        reader.replace_snapshot(same_epoch),
        Err(PotionLiveError::NonMonotonicEpoch {
            current: 2,
            supplied: 2
        })
    );
    let mut wrong_run_input = live_input(&catalog, 3);
    wrong_run_input.binding.run_id = "run:other".to_owned();
    let wrong_run = PotionLiveSnapshot::from_input(wrong_run_input).expect("wrong run");
    assert_eq!(
        reader.replace_snapshot(wrong_run),
        Err(PotionLiveError::RunMismatch)
    );
}

#[test]
fn source_and_family_fences_fail_closed() {
    let manifest = manifest();
    let source = CatalogSource {
        snapshot: Err(PotionSourceError::NoActiveSource),
    };
    assert_eq!(
        PotionCatalogProducer::new().produce(&manifest, &source),
        Err(PotionCatalogError::NoActiveSource)
    );

    let definitions = definitions();
    let unsupported = PotionCatalogProducer::new()
        .produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(PotionCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    producer_version: sts2_game_mod::POTION_PRODUCER_VERSION.to_owned(),
                    family: PotionFamilyCoverage {
                        entity_kind: "potion".to_owned(),
                        state: PotionFamilyState::Unsupported,
                        definition_count: definitions.len(),
                    },
                    definitions: Vec::new(),
                }),
            },
        )
        .expect("unsupported family");
    assert_eq!(
        unsupported.reader().list(&PotionListQuery {
            scope: PotionVisibilityScope::Reference,
            limit: 8,
            continuation: None,
        }),
        Err(PotionCatalogError::UnsupportedFamily)
    );

    let mut wrong_manifest = PotionCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: manifest.locale.clone(),
        producer_version: sts2_game_mod::POTION_PRODUCER_VERSION.to_owned(),
        family: PotionFamilyCoverage {
            entity_kind: "potion".to_owned(),
            state: PotionFamilyState::Handled,
            definition_count: definitions.len(),
        },
        definitions,
    };
    wrong_manifest.manifest.catalog_generation += 1;
    assert_eq!(
        PotionCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(wrong_manifest),
            },
        ),
        Err(PotionCatalogError::ManifestMismatch)
    );
}

#[test]
fn source_live_identity_mismatch_is_rejected_before_publishing_state() {
    let catalog = catalog();
    let expected = binding(&catalog, 4);
    let mut input = live_input(&catalog, 5);
    input.binding.snapshot_id = "snapshot:wrong".to_owned();
    let source = LiveSource {
        snapshot: Ok(input),
    };
    assert!(matches!(
        PotionLiveReader::from_source(&catalog, &expected, &source),
        Err(PotionLiveError::StaleReference)
    ));

    let missing_catalog = PotionDefinitionReference {
        catalog: expected.catalog.clone(),
        potion_id: "mod:synthetic:missing".to_owned(),
    };
    assert_eq!(
        catalog.get(&missing_catalog, PotionVisibilityScope::Public),
        Err(PotionCatalogError::NotFound)
    );
}
