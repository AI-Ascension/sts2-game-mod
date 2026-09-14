// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/act_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ACT_MAX_ENCOUNTERS, ACT_MAX_TEXT_BYTES, ActCatalog, ActCatalogProducer, ActDefinitionInput,
    ActFamilyState, ActField, ActNumericValue, ActReferenceError, ActSemanticReferenceKind,
    ContentManifest, ContentManifestProducer, EncounterKind, EncounterPoolKind, GenerationWeight,
    RoomCategoryKind,
};

fn produce(
    content: &ContentManifest,
    definition: ActDefinitionInput,
) -> Result<ActCatalog, ActReferenceError> {
    ActCatalogProducer::new().produce(
        content,
        &ActSource {
            snapshot: Ok(snapshot(content, vec![definition])),
        },
    )
}

fn full_manifest() -> ContentManifest {
    manifest(
        &["act:one"],
        &["encounter:shared"],
        &["enemy:slime", "enemy:brute", "enemy:lord"],
    )
}

#[test]
fn manifest_locale_and_producer_fences_are_rejected() {
    let content = full_manifest();
    let other = manifest(
        &["act:two"],
        &["encounter:shared"],
        &["enemy:slime", "enemy:brute", "enemy:lord"],
    );

    let mut wrong_manifest = snapshot(&content, vec![rich_act("act:one")]);
    wrong_manifest.manifest = other.cursor_binding();
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Ok(wrong_manifest),
            }
        ),
        Err(ActReferenceError::ManifestMismatch)
    );

    let mut wrong_locale = snapshot(&content, vec![rich_act("act:one")]);
    wrong_locale.locale = "fr-FR".to_owned();
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Ok(wrong_locale),
            }
        ),
        Err(ActReferenceError::LocaleMismatch)
    );

    let mut wrong_producer = snapshot(&content, vec![rich_act("act:one")]);
    wrong_producer.producer_version = "game-act-encounter-reference-producer-v9".to_owned();
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Ok(wrong_producer),
            }
        ),
        Err(ActReferenceError::ProducerVersionMismatch)
    );

    let mut wrong_family = snapshot(&content, vec![rich_act("act:one")]);
    wrong_family.family.entity_kind = "enemy".to_owned();
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Ok(wrong_family),
            }
        ),
        Err(ActReferenceError::FamilyIdentityMismatch)
    );
}

#[test]
fn unsupported_and_unavailable_families_fail_closed() {
    let content = manifest(&[], &[], &[]);
    let mut unsupported = snapshot(&content, Vec::new());
    unsupported.family.state = ActFamilyState::Unsupported;
    let catalog = ActCatalogProducer::new()
        .produce(
            &content,
            &ActSource {
                snapshot: Ok(unsupported),
            },
        )
        .expect("unsupported catalog");
    assert_eq!(catalog.family().state, ActFamilyState::Unsupported);
    assert_eq!(
        catalog.reader().list(&sts2_game_mod::ActListQuery {
            locale: "en-US".to_owned(),
            scope: sts2_game_mod::ActVisibilityScope::Owner,
            limit: 8,
            continuation: None,
        }),
        Err(ActReferenceError::UnsupportedFamily)
    );
    assert_eq!(
        catalog.get(
            &act_reference(&catalog, "act:x"),
            sts2_game_mod::ActVisibilityScope::Owner
        ),
        Err(ActReferenceError::UnsupportedFamily)
    );

    let mut unavailable = snapshot(&content, Vec::new());
    unavailable.family.state = ActFamilyState::Unavailable;
    let catalog = ActCatalogProducer::new()
        .produce(
            &content,
            &ActSource {
                snapshot: Ok(unavailable),
            },
        )
        .expect("unavailable catalog");
    assert_eq!(catalog.family().state, ActFamilyState::Unavailable);

    let content = manifest(&["act:one"], &[], &["enemy:slime"]);
    let mut carrying = snapshot(&content, vec![simple_act("act:one")]);
    carrying.family.state = ActFamilyState::Unsupported;
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Ok(carrying),
            }
        ),
        Err(ActReferenceError::UnknownDefinition("act:one".to_owned()))
    );
}

#[test]
fn missing_family_and_family_count_are_rejected() {
    let mut source = manifest_source(&[], &["encounter:x"], &["enemy:slime"]);
    source
        .snapshot
        .available_entity_kinds
        .retain(|kind| kind != "act");
    let no_act =
        ContentManifestProducer::new("adapter-v1", ["encounter".to_owned(), "enemy".to_owned()])
            .expect("producer")
            .produce(&source)
            .expect("manifest");
    assert_eq!(
        ActCatalogProducer::new().produce(
            &no_act,
            &ActSource {
                snapshot: Ok(snapshot(&no_act, Vec::new())),
            }
        ),
        Err(ActReferenceError::MissingFamily)
    );

    let content = manifest(&["act:one"], &[], &["enemy:slime"]);
    let mut wrong_count = snapshot(&content, vec![simple_act("act:one")]);
    wrong_count.family.definition_count = 5;
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Ok(wrong_count),
            }
        ),
        Err(ActReferenceError::FamilyCountMismatch)
    );
}

#[test]
fn duplicate_identities_are_rejected() {
    let content = manifest(&["act:dup"], &[], &["enemy:slime"]);
    let mut two = snapshot(&content, vec![simple_act("act:dup")]);
    two.definitions.push(simple_act("act:dup"));
    assert_eq!(
        ActCatalogProducer::new().produce(&content, &ActSource { snapshot: Ok(two) }),
        Err(ActReferenceError::DuplicateDefinition("act:dup".to_owned()))
    );

    let mut duplicate_category = simple_act("act:dup");
    duplicate_category
        .room_categories
        .push(room_category("category:normal", RoomCategoryKind::Normal));
    assert_eq!(
        produce(&content, duplicate_category),
        Err(ActReferenceError::InvalidInput("duplicate_room_category"))
    );

    let mut duplicate_encounter = simple_act("act:dup");
    duplicate_encounter.encounters.push(encounter(
        "encounter:normal",
        EncounterKind::Normal,
        Some("category:normal"),
        vec![group("group:two", vec![enemy("enemy:slime", 1)])],
    ));
    assert_eq!(
        produce(&content, duplicate_encounter),
        Err(ActReferenceError::InvalidInput("duplicate_encounter"))
    );

    let mut duplicate_group = simple_act("act:dup");
    duplicate_group.encounters[0]
        .groups
        .push(group("group:one", vec![enemy("enemy:slime", 1)]));
    assert_eq!(
        produce(&content, duplicate_group),
        Err(ActReferenceError::InvalidInput("duplicate_group"))
    );

    let mut duplicate_enemy = simple_act("act:dup");
    duplicate_enemy.encounters[0].groups[0]
        .enemies
        .push(enemy("enemy:slime", 1));
    assert_eq!(
        produce(&content, duplicate_enemy),
        Err(ActReferenceError::InvalidInput("duplicate_enemy"))
    );

    let mut duplicate_entry = simple_act("act:dup");
    duplicate_entry.pools = ActField::Available(vec![pool(
        "pool:normal",
        EncounterPoolKind::Normal,
        "category:normal",
        vec![
            entry("entry:normal", "encounter:normal"),
            entry("entry:normal", "encounter:normal"),
        ],
    )]);
    assert_eq!(
        produce(&content, duplicate_entry),
        Err(ActReferenceError::InvalidInput("duplicate_pool_entry"))
    );

    let mut duplicate_reference = simple_act("act:dup");
    duplicate_reference.references = vec![
        reference(ActSemanticReferenceKind::Enemy, "enemy:slime"),
        reference(ActSemanticReferenceKind::Enemy, "enemy:slime"),
    ];
    assert_eq!(
        produce(&content, duplicate_reference),
        Err(ActReferenceError::InvalidInput("duplicate_reference"))
    );
}

#[test]
fn malformed_identities_and_collection_bounds_are_rejected() {
    let content = manifest(&["act:ok"], &[], &["enemy:slime"]);

    let mut bad_identity = simple_act("act:ok");
    bad_identity.act_id = "act:bad\u{1}".to_owned();
    assert_eq!(
        produce(&content, bad_identity),
        Err(ActReferenceError::InvalidInput("act_id"))
    );

    let mut empty_name = simple_act("act:ok");
    empty_name.name = sts2_game_mod::ActText::Available(String::new());
    assert_eq!(
        produce(&content, empty_name),
        Err(ActReferenceError::InvalidInput("name"))
    );

    let mut oversized_text = simple_act("act:ok");
    oversized_text.name = sts2_game_mod::ActText::Available("x".repeat(ACT_MAX_TEXT_BYTES + 1));
    assert_eq!(
        produce(&content, oversized_text),
        Err(ActReferenceError::InvalidInput("name"))
    );

    let mut bad_quantity = simple_act("act:ok");
    bad_quantity.encounters[0].groups[0].enemies[0].quantity = ActNumericValue::Fixed(0);
    assert_eq!(
        produce(&content, bad_quantity),
        Err(ActReferenceError::InvalidInput("enemy_quantity"))
    );

    let mut bad_weight = simple_act("act:ok");
    bad_weight.encounters[0].weight = GenerationWeight::Exact {
        numerator: 1,
        denominator: 0,
        evidence: sts2_game_mod::ActEvidence::SourceDerived,
    };
    assert_eq!(
        produce(&content, bad_weight),
        Err(ActReferenceError::InvalidInput("weight"))
    );

    let mut too_many_categories = simple_act("act:ok");
    too_many_categories.room_categories = (0..sts2_game_mod::ACT_MAX_ROOM_CATEGORIES + 1)
        .map(|index| room_category(&format!("category:{index}"), RoomCategoryKind::Normal))
        .collect();
    assert_eq!(
        produce(&content, too_many_categories),
        Err(ActReferenceError::InvalidInput("room_categories"))
    );

    let mut too_many_encounters = simple_act("act:ok");
    too_many_encounters.encounters = (0..ACT_MAX_ENCOUNTERS + 1)
        .map(|index| {
            encounter(
                &format!("encounter:{index}"),
                EncounterKind::Normal,
                Some("category:normal"),
                vec![group("group:one", vec![enemy("enemy:slime", 1)])],
            )
        })
        .collect();
    assert_eq!(
        produce(&content, too_many_encounters),
        Err(ActReferenceError::InvalidInput("encounters"))
    );

    let mut too_many_enemies = simple_act("act:ok");
    too_many_enemies.encounters[0].groups[0].enemies = (0..sts2_game_mod::ACT_MAX_GROUP_ENEMIES
        + 1)
        .map(|index| enemy(&format!("enemy:{index}"), 1))
        .collect();
    assert_eq!(
        produce(&content, too_many_enemies),
        Err(ActReferenceError::InvalidInput("enemies"))
    );
}

#[test]
fn dangling_references_are_rejected() {
    let content = full_manifest();

    let mut unknown_enemy = rich_act("act:one");
    unknown_enemy.encounters[0].groups[0].enemies[0].enemy_id = "enemy:missing".to_owned();
    assert_eq!(
        produce(&content, unknown_enemy),
        Err(ActReferenceError::UnknownManifestReference {
            entity_kind: "enemy".to_owned(),
            namespaced_id: "enemy:missing".to_owned(),
        })
    );

    let mut unknown_reference = rich_act("act:one");
    unknown_reference
        .references
        .push(reference(ActSemanticReferenceKind::Enemy, "enemy:missing"));
    assert_eq!(
        produce(&content, unknown_reference),
        Err(ActReferenceError::UnknownManifestReference {
            entity_kind: "enemy".to_owned(),
            namespaced_id: "enemy:missing".to_owned(),
        })
    );

    let mut unknown_encounter = rich_act("act:one");
    unknown_encounter.pools = ActField::Available(vec![pool(
        "pool:normal",
        EncounterPoolKind::Normal,
        "category:normal",
        vec![entry("entry:normal", "encounter:missing")],
    )]);
    assert_eq!(
        produce(&content, unknown_encounter),
        Err(ActReferenceError::UnknownEncounterReference {
            act_id: "act:one".to_owned(),
            encounter_id: "encounter:missing".to_owned(),
        })
    );

    let mut unknown_category = rich_act("act:one");
    unknown_category.encounters[0].room_category_id = Some("category:missing".to_owned());
    assert_eq!(
        produce(&content, unknown_category),
        Err(ActReferenceError::UnknownRoomCategoryReference {
            act_id: "act:one".to_owned(),
            category_id: "category:missing".to_owned(),
        })
    );

    let mut dangling_category_reference = rich_act("act:one");
    dangling_category_reference.references.push(reference(
        ActSemanticReferenceKind::RoomCategory,
        "category:missing",
    ));
    assert_eq!(
        produce(&content, dangling_category_reference),
        Err(ActReferenceError::UnknownRoomCategoryReference {
            act_id: "act:one".to_owned(),
            category_id: "category:missing".to_owned(),
        })
    );
}

#[test]
fn act_scoped_room_category_references_resolve() {
    let content = full_manifest();

    let mut local_category_reference = rich_act("act:one");
    local_category_reference.references.push(reference(
        ActSemanticReferenceKind::RoomCategory,
        "category:normal",
    ));
    assert!(produce(&content, local_category_reference).is_ok());
}
