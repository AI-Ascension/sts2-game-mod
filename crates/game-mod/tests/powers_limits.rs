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
fn live_reader_rejects_unsupported_and_unavailable_empty_catalogs() {
    let manifest = manifest();
    for state in [
        PowerStatusFamilyState::Unsupported,
        PowerStatusFamilyState::Unavailable,
    ] {
        let catalog = sts2_game_mod::PowerStatusCatalogProducer::new()
            .produce(
                &manifest,
                &fixture::CatalogSource {
                    snapshot: Ok(PowerStatusCatalogSnapshot {
                        manifest: manifest.cursor_binding(),
                        locale: manifest.locale.clone(),
                        producer_version: POWER_STATUS_PRODUCER_VERSION.to_owned(),
                        family: PowerStatusFamilyCoverage {
                            entity_kind: "power_status".to_owned(),
                            state,
                            definition_count: 6,
                        },
                        definitions: Vec::new(),
                    }),
                },
            )
            .expect("catalog");
        let snapshot = PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
            binding: binding(&catalog, 1),
            instances: Vec::new(),
        })
        .expect("empty snapshot");
        let result = PowerStatusLiveReader::new(&catalog, snapshot);
        match state {
            PowerStatusFamilyState::Unsupported => {
                assert!(matches!(
                    result,
                    Err(PowerStatusLiveError::UnsupportedFamily)
                ));
            }
            PowerStatusFamilyState::Unavailable => {
                assert!(matches!(
                    result,
                    Err(PowerStatusLiveError::UnavailableFamily)
                ));
            }
            PowerStatusFamilyState::Handled => unreachable!("handled state is not exercised"),
        }
    }
}

#[test]
fn conditional_duration_ids_must_match_static_definition() {
    let manifest = manifest();
    let mut definitions = definitions();
    definitions[0].duration = sts2_game_mod::PowerStatusDurationDefinition {
        default: None,
        rule: sts2_game_mod::PowerStatusDurationRule::Condition("condition:expected".to_owned()),
        reset: sts2_game_mod::PowerStatusReset::Activation,
    };
    let catalog = sts2_game_mod::PowerStatusCatalogProducer::new()
        .produce(
            &manifest,
            &fixture::CatalogSource {
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
            },
        )
        .expect("conditional catalog");
    let mut conditional = instance(
        "instance:conditional",
        "mod:synthetic:strength",
        sts2_game_mod::PowerStatusOwnerKind::Player,
        PowerStatusField::Available(integer(1, "count")),
        PowerStatusField::Available(sts2_game_mod::PowerStatusDurationState::Condition {
            id: "condition:wrong".to_owned(),
            label: "Wrong condition".to_owned(),
        }),
    );
    assert!(matches!(
        PowerStatusLiveReader::new(
            &catalog,
            PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
                binding: binding(&catalog, 1),
                instances: vec![conditional.clone()],
            })
            .expect("wrong condition snapshot"),
        ),
        Err(PowerStatusLiveError::DurationMismatch("duration_rule"))
    ));
    conditional.duration =
        PowerStatusField::Available(sts2_game_mod::PowerStatusDurationState::Condition {
            id: "condition:expected".to_owned(),
            label: "Expected condition".to_owned(),
        });
    assert!(
        PowerStatusLiveReader::new(
            &catalog,
            PowerStatusLiveSnapshot::from_input(PowerStatusLiveSnapshotInput {
                binding: binding(&catalog, 2),
                instances: vec![conditional],
            })
            .expect("matching condition snapshot"),
        )
        .is_ok()
    );
}

#[test]
fn zero_decay_floor_is_allowed_but_decrement_requires_positive_amount() {
    let manifest = manifest();
    let mut definitions = definitions();
    definitions[2].decay.rule = sts2_game_mod::PowerStatusDecayRule::To {
        amount: 0,
        unit: unit("damage"),
    };
    let floor_source = fixture::CatalogSource {
        snapshot: Ok(PowerStatusCatalogSnapshot {
            manifest: manifest.cursor_binding(),
            locale: manifest.locale.clone(),
            producer_version: POWER_STATUS_PRODUCER_VERSION.to_owned(),
            family: PowerStatusFamilyCoverage {
                entity_kind: "power_status".to_owned(),
                state: PowerStatusFamilyState::Handled,
                definition_count: definitions.len(),
            },
            definitions: definitions.clone(),
        }),
    };
    assert!(
        sts2_game_mod::PowerStatusCatalogProducer::new()
            .produce(&manifest, &floor_source)
            .is_ok()
    );

    definitions[2].decay.rule = sts2_game_mod::PowerStatusDecayRule::By {
        amount: 0,
        unit: unit("damage"),
    };
    let decrement_source = fixture::CatalogSource {
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
    assert_eq!(
        sts2_game_mod::PowerStatusCatalogProducer::new().produce(&manifest, &decrement_source),
        Err(PowerStatusCatalogError::InvalidInput("decay_amount"))
    );
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
