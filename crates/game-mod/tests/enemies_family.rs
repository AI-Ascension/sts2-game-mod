// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/enemies.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentManifestProducer, ContentUnlockState, EnemyCatalogError, EnemyCatalogProducer,
    EnemyDefinitionReference, EnemyFamilyState, EnemyFieldStatus, EnemyKind, EnemyListQuery,
    EnemySemanticReferenceKind, EnemySourceError, EnemyVisibility, EnemyVisibilityScope,
};

fn list_query(locale: &str, scope: EnemyVisibilityScope, limit: usize) -> EnemyListQuery {
    EnemyListQuery {
        locale: locale.to_owned(),
        scope,
        limit,
        continuation: None,
    }
}

fn visible_enemy(enemy_id: &str) -> sts2_game_mod::EnemyDefinitionInput {
    enemy(
        enemy_id,
        EnemyKind::Normal,
        EnemyVisibility::Visible,
        ContentUnlockState::Unlocked,
    )
}

#[test]
fn unsupported_family_fails_closed_instead_of_empty_success() {
    let content = manifest(&[], &[], &[]);
    let mut incomplete = snapshot(&content, Vec::new());
    incomplete.family.state = EnemyFamilyState::Unsupported;
    let catalog = EnemyCatalogProducer::new()
        .produce(
            &content,
            &EnemySource {
                snapshot: Ok(incomplete),
            },
        )
        .expect("unsupported catalog");
    assert_eq!(catalog.family().state, EnemyFamilyState::Unsupported);

    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&list_query("en-US", EnemyVisibilityScope::Owner, 8)),
        Err(EnemyCatalogError::UnsupportedFamily)
    );
    let reference = EnemyDefinitionReference {
        catalog: catalog.binding().clone(),
        enemy_id: "enemy:x".to_owned(),
    };
    assert_eq!(
        reader.get(&reference, EnemyVisibilityScope::Owner),
        Err(EnemyCatalogError::UnsupportedFamily)
    );

    let content = manifest(&["enemy:a"], &[], &[]);
    let mut carrying = snapshot(&content, vec![visible_enemy("enemy:a")]);
    carrying.family.state = EnemyFamilyState::Unsupported;
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(carrying),
            }
        ),
        Err(EnemyCatalogError::UnknownDefinition("enemy:a".to_owned()))
    );
}

#[test]
fn unavailable_family_fails_closed_instead_of_empty_success() {
    let content = manifest(&[], &[], &[]);
    let mut unavailable = snapshot(&content, Vec::new());
    unavailable.family.state = EnemyFamilyState::Unavailable;
    let catalog = EnemyCatalogProducer::new()
        .produce(
            &content,
            &EnemySource {
                snapshot: Ok(unavailable),
            },
        )
        .expect("unavailable catalog");
    assert_eq!(catalog.family().state, EnemyFamilyState::Unavailable);
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&list_query("en-US", EnemyVisibilityScope::Owner, 8)),
        Err(EnemyCatalogError::UnavailableFamily)
    );
}

#[test]
fn family_identity_count_and_missing_family_are_rejected() {
    let content = manifest(&["enemy:a"], &[], &[]);
    let mut wrong_kind = snapshot(&content, vec![visible_enemy("enemy:a")]);
    wrong_kind.family.entity_kind = "relic".to_owned();
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(wrong_kind),
            }
        ),
        Err(EnemyCatalogError::FamilyIdentityMismatch)
    );

    let mut wrong_count = snapshot(&content, vec![visible_enemy("enemy:a")]);
    wrong_count.family.definition_count = 5;
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(wrong_count),
            }
        ),
        Err(EnemyCatalogError::FamilyCountMismatch)
    );

    let mut source = manifest_source(&[], &["encounter:x"], &[]);
    source
        .snapshot
        .available_entity_kinds
        .retain(|kind| kind != "enemy");
    source.snapshot.registry_definition_counts.remove("enemy");
    let no_enemy = ContentManifestProducer::new("adapter-v1", ["encounter".to_owned()])
        .expect("producer")
        .produce(&source)
        .expect("manifest");
    let missing = snapshot(&no_enemy, Vec::new());
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &no_enemy,
            &EnemySource {
                snapshot: Ok(missing),
            }
        ),
        Err(EnemyCatalogError::MissingFamily)
    );
}

#[test]
fn manifest_locale_and_producer_fences_are_rejected() {
    let content = manifest(&["enemy:a"], &[], &[]);
    let other = manifest(&["enemy:b"], &[], &[]);

    let mut wrong_manifest = snapshot(&content, vec![visible_enemy("enemy:a")]);
    wrong_manifest.manifest = other.cursor_binding();
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(wrong_manifest),
            }
        ),
        Err(EnemyCatalogError::ManifestMismatch)
    );

    let mut wrong_locale = snapshot(&content, vec![visible_enemy("enemy:a")]);
    wrong_locale.locale = "fr-FR".to_owned();
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(wrong_locale),
            }
        ),
        Err(EnemyCatalogError::LocaleMismatch)
    );

    let mut wrong_producer = snapshot(&content, vec![visible_enemy("enemy:a")]);
    wrong_producer.producer_version = "game-enemy-reference-producer-v9".to_owned();
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(wrong_producer),
            }
        ),
        Err(EnemyCatalogError::ProducerVersionMismatch)
    );
}

#[test]
fn unavailable_and_not_applicable_fields_remain_explicit() {
    let content = manifest(
        &["enemy:a", "enemy:boss"],
        &["encounter:boss"],
        &["power_status:weak"],
    );
    let catalog = catalog(&content, vec![visible_enemy("enemy:a"), boss("enemy:boss")]);
    let mut reader = catalog.reader();
    let page = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Owner, 8))
        .expect("page");
    assert_eq!(page.entries.len(), 2);
    let plain = page
        .entries
        .iter()
        .find(|entry| entry.reference.enemy_id == "enemy:a")
        .expect("plain entry");
    assert_eq!(plain.spawn_conditions, EnemyFieldStatus::NotObserved);
    assert_eq!(plain.encounters, EnemyFieldStatus::NotApplicable);

    let definition = catalog
        .get(
            &definition_reference(&catalog, "enemy:a"),
            EnemyVisibilityScope::Owner,
        )
        .expect("definition");
    assert_eq!(definition.stats.base.status(), EnemyFieldStatus::Available);
    assert_eq!(
        definition.stats.scaled.status(),
        EnemyFieldStatus::NotObserved
    );
    assert_eq!(
        definition.spawn_conditions.status(),
        EnemyFieldStatus::NotObserved
    );
    assert_eq!(
        definition.encounters.status(),
        EnemyFieldStatus::NotApplicable
    );

    let boss_definition = catalog
        .get(
            &definition_reference(&catalog, "enemy:boss"),
            EnemyVisibilityScope::Owner,
        )
        .expect("boss");
    assert_eq!(
        boss_definition.phases[0].entry_condition.status(),
        EnemyFieldStatus::NotApplicable
    );
    assert_eq!(
        boss_definition.encounters.status(),
        EnemyFieldStatus::Available
    );
    assert_eq!(
        boss_definition.moves[2].probability,
        sts2_game_mod::EnemyProbability::Unavailable(
            sts2_game_mod::EnemyUnavailableReason::NotApplicable
        )
    );
}

#[test]
fn unknown_manifest_reference_origin_and_package_are_rejected() {
    let content = manifest(&["enemy:boss"], &["encounter:boss"], &["power_status:weak"]);

    let mut unknown_reference = boss("enemy:boss");
    unknown_reference.references.push(reference(
        EnemySemanticReferenceKind::Status,
        "power_status:missing",
    ));
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(snapshot(&content, vec![unknown_reference])),
            }
        ),
        Err(EnemyCatalogError::UnknownManifestReference {
            entity_kind: "power_status".to_owned(),
            namespaced_id: "power_status:missing".to_owned(),
        })
    );

    let mut wrong_origin = boss("enemy:boss");
    wrong_origin.origin.package_id = Some("other:synthetic".to_owned());
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(snapshot(&content, vec![wrong_origin])),
            }
        ),
        Err(EnemyCatalogError::OriginMismatch("enemy:boss".to_owned()))
    );

    let mut unknown_package = boss("enemy:boss");
    unknown_package.origin_variants[0].origin.package_id = Some("package:missing".to_owned());
    unknown_package.origin_variants[0].origin.package_version = None;
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Ok(snapshot(&content, vec![unknown_package])),
            }
        ),
        Err(EnemyCatalogError::UnknownOriginPackage(
            "package:missing".to_owned()
        ))
    );
}

#[test]
fn source_failures_map_to_typed_errors() {
    let content = manifest(&[], &[], &[]);
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Err(EnemySourceError::Malformed),
            }
        ),
        Err(EnemyCatalogError::MalformedSource)
    );
    assert_eq!(
        EnemyCatalogProducer::new().produce(
            &content,
            &EnemySource {
                snapshot: Err(EnemySourceError::NoActiveSource),
            }
        ),
        Err(EnemyCatalogError::NoActiveSource)
    );
}
