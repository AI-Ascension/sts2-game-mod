// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/content_index.rs"]
mod content_fixture;
#[path = "support/powers.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    POWER_STATUS_MAX_DEFINITION_BYTES, POWER_STATUS_MAX_LIVE_DETAIL_BYTES,
    POWER_STATUS_PRODUCER_VERSION, PowerStatusAmount, PowerStatusCatalogError,
    PowerStatusCatalogSnapshot, PowerStatusFamilyCoverage, PowerStatusFamilyState,
    PowerStatusField, PowerStatusLiveError, PowerStatusLiveReader, PowerStatusLiveSnapshot,
    PowerStatusLiveSnapshotInput, PowerStatusSemanticReference, PowerStatusSourceError,
    PowerStatusVisibilityScope,
};

#[test]
fn oversized_static_definition_fails_before_catalog_publication() {
    let manifest = manifest();
    let mut definitions = definitions();
    definitions[0].references = (0..5)
        .map(|index| PowerStatusSemanticReference {
            kind: sts2_game_mod::PowerStatusReferenceKind::Rule,
            id: format!("rule:oversized:{index}"),
            label: "x".repeat(16_000),
        })
        .collect();
    let source = fixture::CatalogSource {
        snapshot: Ok(PowerStatusCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: POWER_STATUS_PRODUCER_VERSION.to_owned(),
            family: PowerStatusFamilyCoverage {
                entity_kind: "power_status".to_owned(),
                state: PowerStatusFamilyState::Handled,
                definition_count: definitions.len(),
            },
            definitions,
        }),
    };
    assert!(matches!(
        sts2_game_mod::PowerStatusCatalogProducer::new().produce(&manifest, &source),
        Err(PowerStatusCatalogError::DefinitionTooLarge {
            limit: POWER_STATUS_MAX_DEFINITION_BYTES,
            ..
        })
    ));
}

#[test]
fn oversized_live_text_fails_after_static_visibility_is_authorized() {
    let catalog = catalog();
    let mut instance = instance(
        "instance:hidden",
        "mod:synthetic:hidden",
        sts2_game_mod::PowerStatusOwnerKind::Player,
        PowerStatusField::Available(PowerStatusAmount::Text {
            value: "x".repeat(POWER_STATUS_MAX_LIVE_DETAIL_BYTES),
            unit: unit("label"),
        }),
        PowerStatusField::Available(sts2_game_mod::PowerStatusDurationState::Permanent),
    );
    instance.source = PowerStatusField::Denied;
    let snapshot = PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
        binding: binding(&catalog, 4),
        instances: vec![instance],
    })
    .expect("snapshot");
    assert!(matches!(
        PowerStatusLiveReader::new_with_scope(
            &catalog,
            snapshot,
            PowerStatusVisibilityScope::Owner,
        ),
        Err(PowerStatusLiveError::DetailTooLarge {
            limit: POWER_STATUS_MAX_LIVE_DETAIL_BYTES,
            ..
        })
    ));
}

#[test]
fn duplicate_instances_and_bad_identity_are_rejected() {
    let catalog = catalog();
    let input = instance(
        "instance:duplicate",
        "mod:synthetic:strength",
        sts2_game_mod::PowerStatusOwnerKind::Enemy,
        PowerStatusField::Available(integer(1, "count")),
        PowerStatusField::Available(duration(1, "turn")),
    );
    assert_eq!(
        PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
            binding: binding(&catalog, 1),
            instances: vec![input.clone(), input],
        }),
        Err(PowerStatusLiveError::DuplicateInstance(
            "instance:duplicate".to_owned()
        ))
    );
    let mut invalid = instance(
        "instance:invalid",
        "mod:synthetic:strength",
        sts2_game_mod::PowerStatusOwnerKind::Player,
        PowerStatusField::Available(integer(1, "count")),
        PowerStatusField::Available(duration(1, "turn")),
    );
    assert!(sts2_game_mod::PowerStatusOwnerId::new("bad owner").is_err());
    invalid.instance_id = "bad identity".to_owned();
    assert!(matches!(
        PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
            binding: binding(&catalog, 2),
            instances: vec![invalid],
        }),
        Err(PowerStatusLiveError::InvalidInput("instance_id"))
    ));
}

#[test]
fn source_errors_and_wrong_catalog_locale_do_not_publish_partial_data() {
    let manifest = manifest();
    let mut wrong_locale = PowerStatusCatalogSnapshot {
        manifest: manifest.cursor_binding(),
        locale: "fr-FR".to_owned(),
        producer_version: POWER_STATUS_PRODUCER_VERSION.to_owned(),
        family: PowerStatusFamilyCoverage {
            entity_kind: "power_status".to_owned(),
            state: PowerStatusFamilyState::Handled,
            definition_count: definitions().len(),
        },
        definitions: definitions(),
    };
    let source = fixture::CatalogSource {
        snapshot: Ok(wrong_locale.clone()),
    };
    assert_eq!(
        sts2_game_mod::PowerStatusCatalogProducer::new().produce(&manifest, &source),
        Err(PowerStatusCatalogError::LocaleMismatch)
    );
    wrong_locale.locale = manifest.locale.clone();
    wrong_locale.family.entity_kind = "status".to_owned();
    assert_eq!(
        sts2_game_mod::PowerStatusCatalogProducer::new().produce(
            &manifest,
            &fixture::CatalogSource {
                snapshot: Ok(wrong_locale),
            },
        ),
        Err(PowerStatusCatalogError::FamilyIdentityMismatch)
    );
    assert_eq!(
        sts2_game_mod::PowerStatusCatalogProducer::new().produce(
            &manifest,
            &fixture::CatalogSource {
                snapshot: Err(PowerStatusSourceError::AccessDenied),
            },
        ),
        Err(PowerStatusCatalogError::SourceAccessDenied)
    );
}

#[test]
fn unsupported_family_is_explicit() {
    let manifest = manifest();
    let source = fixture::CatalogSource {
        snapshot: Ok(PowerStatusCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: POWER_STATUS_PRODUCER_VERSION.to_owned(),
            family: PowerStatusFamilyCoverage {
                entity_kind: "power_status".to_owned(),
                state: PowerStatusFamilyState::Unsupported,
                definition_count: 6,
            },
            definitions: Vec::new(),
        }),
    };
    let catalog = sts2_game_mod::PowerStatusCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("unsupported catalog");
    assert_eq!(
        catalog.reader().list(&sts2_game_mod::PowerStatusListQuery {
            scope: PowerStatusVisibilityScope::Public,
            limit: 1,
            continuation: None,
        }),
        Err(PowerStatusCatalogError::UnsupportedFamily)
    );
    assert_eq!(catalog.family().state, PowerStatusFamilyState::Unsupported);
    assert_eq!(catalog.family().definition_count, 6);
}
