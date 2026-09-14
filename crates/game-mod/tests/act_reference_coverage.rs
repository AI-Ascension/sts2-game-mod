// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/act_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ACT_MAX_DEFINITION_BYTES, ACT_MAX_ELIGIBILITY, ACT_MAX_ENCOUNTERS, ACT_MAX_IDENTITY_BYTES,
    ActCatalog, ActCatalogProducer, ActDefinitionInput, ActField, ActReferenceError,
    ActSourceError, ActUnavailableReason, ContentManifest, EligibilityKind, EncounterKind,
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
fn explicit_unavailable_fields_remain_distinct_from_empty_success() {
    let content = full_manifest();
    let mut definition = rich_act("act:one");
    definition.pools = ActField::Unavailable(ActUnavailableReason::Denied);
    definition.constraints = ActField::Unavailable(ActUnavailableReason::NotObserved);
    let catalog = catalog(&content, vec![definition]);
    let act = act_reference(&catalog, "act:one");

    let mut reader = catalog.reader();
    let page = reader
        .list(&sts2_game_mod::ActListQuery {
            locale: "en-US".to_owned(),
            scope: sts2_game_mod::ActVisibilityScope::Owner,
            limit: 8,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.entries[0].pools, sts2_game_mod::ActFieldStatus::Denied);
    assert_eq!(
        page.entries[0].constraints,
        sts2_game_mod::ActFieldStatus::NotObserved
    );
    assert_eq!(
        catalog.get_pool(
            &pool_reference(&catalog, "act:one", "pool:normal"),
            sts2_game_mod::ActVisibilityScope::Owner
        ),
        Err(ActReferenceError::UnavailableField(
            ActUnavailableReason::Denied
        ))
    );
    assert_eq!(
        catalog.get_constraint(
            &act,
            "constraint:paths",
            sts2_game_mod::ActVisibilityScope::Owner
        ),
        Err(ActReferenceError::UnavailableField(
            ActUnavailableReason::NotObserved
        ))
    );
}

fn bulk_definition(kind_len: usize) -> ActDefinitionInput {
    let encounters = (0..ACT_MAX_ENCOUNTERS)
        .map(|encounter_index| {
            let mut conditions = Vec::new();
            for condition_index in 0..ACT_MAX_ELIGIBILITY {
                let mut condition = eligibility(
                    &format!("condition:{encounter_index}:{condition_index}"),
                    EligibilityKind::Custom("k".repeat(kind_len)),
                );
                condition.parameters.clear();
                conditions.push(condition);
            }
            let mut definition = encounter(
                &format!("encounter:{encounter_index}"),
                EncounterKind::Normal,
                Some("category:normal"),
                vec![group("group:one", vec![enemy("enemy:slime", 1)])],
            );
            definition.eligibility = conditions;
            definition
        })
        .collect();
    ActDefinitionInput {
        act_id: "act:bulk".to_owned(),
        name: text("Bulk"),
        description: text("A bulk act."),
        order: 1,
        unlock_state: sts2_game_mod::ContentUnlockState::Unlocked,
        visibility: sts2_game_mod::ActVisibility::Visible,
        room_categories: vec![room_category("category:normal", RoomCategoryKind::Normal)],
        encounters,
        pools: ActField::Unavailable(ActUnavailableReason::NotApplicable),
        constraints: ActField::Unavailable(ActUnavailableReason::NotApplicable),
        references: Vec::new(),
    }
}

fn bulk_actual(kind_len: usize) -> usize {
    let content = manifest(&["act:bulk"], &[], &["enemy:slime"]);
    match produce(&content, bulk_definition(kind_len)) {
        Err(ActReferenceError::DefinitionTooLarge { actual, .. }) => actual,
        other => {
            assert!(matches!(
                other,
                Err(ActReferenceError::DefinitionTooLarge { .. })
            ));
            0
        }
    }
}

#[test]
fn definition_byte_limit_counts_nested_custom_kind_strings() {
    let long = bulk_actual(ACT_MAX_IDENTITY_BYTES);
    let short = bulk_actual(ACT_MAX_IDENTITY_BYTES - 6);
    assert!(long > ACT_MAX_DEFINITION_BYTES);
    assert_eq!(
        long - short,
        ACT_MAX_ENCOUNTERS * ACT_MAX_ELIGIBILITY * 6,
        "every nested custom condition-kind byte must count toward the definition bound"
    );
}

#[test]
fn source_failures_map_to_typed_errors() {
    let content = manifest(&[], &[], &[]);
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Err(ActSourceError::Malformed),
            }
        ),
        Err(ActReferenceError::MalformedSource)
    );
    assert_eq!(
        ActCatalogProducer::new().produce(
            &content,
            &ActSource {
                snapshot: Err(ActSourceError::NoActiveSource),
            }
        ),
        Err(ActReferenceError::NoActiveSource)
    );
}
