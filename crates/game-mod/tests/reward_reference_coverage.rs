// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/reward_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    REWARD_MAX_DEFINITION_BYTES, REWARD_MAX_IDENTITY_BYTES, REWARD_MAX_ITEMS,
    REWARD_MAX_REQUIREMENTS, REWARD_MAX_RULES, RewardCatalog, RewardCatalogError,
    RewardCatalogProducer, RewardCatalogSnapshot, RewardFamilyState, RewardOfferDefinitionInput,
    RewardRequirementKind, RewardSemanticReferenceKind, RewardSourceError, RewardVisibility,
    RewardVisibilityScope,
};

fn produce(
    content: &sts2_game_mod::ContentManifest,
    definition: RewardOfferDefinitionInput,
) -> Result<RewardCatalog, RewardCatalogError> {
    RewardCatalogProducer::new().produce(
        content,
        &RewardSource {
            snapshot: Ok(snapshot(content, vec![definition])),
        },
    )
}

fn reward_manifest() -> sts2_game_mod::ContentManifest {
    manifest(&[("reward", "reward:bulk"), ("card", "card:strike")])
}

#[test]
fn source_failures_map_to_typed_errors() {
    let content = manifest(&[]);
    assert_eq!(
        RewardCatalogProducer::new().produce(
            &content,
            &RewardSource {
                snapshot: Err(RewardSourceError::Malformed),
            }
        ),
        Err(RewardCatalogError::MalformedSource)
    );
    assert_eq!(
        RewardCatalogProducer::new().produce(
            &content,
            &RewardSource {
                snapshot: Err(RewardSourceError::NoActiveSource),
            }
        ),
        Err(RewardCatalogError::NoActiveSource)
    );
    assert_eq!(
        RewardCatalogProducer::new().produce(
            &content,
            &RewardSource {
                snapshot: Err(RewardSourceError::AccessDenied),
            }
        ),
        Err(RewardCatalogError::SourceAccessDenied)
    );
}

#[test]
fn unsupported_and_unavailable_families_fail_closed() {
    let content = reward_manifest();
    for state in [
        RewardFamilyState::Unsupported,
        RewardFamilyState::Unavailable,
    ] {
        let mut parts = snapshot(
            &content,
            vec![simple_reward(
                "reward:bulk",
                "item:one",
                RewardSemanticReferenceKind::Card,
                "card:strike",
            )],
        );
        parts.family.state = state;
        parts.definitions.clear();
        let catalog = RewardCatalogProducer::new()
            .produce(
                &content,
                &RewardSource {
                    snapshot: Ok(parts),
                },
            )
            .expect("family catalog retained");
        assert_eq!(catalog.family().state, state);
        assert_eq!(
            catalog
                .reader()
                .list(&list_query("en-US", RewardVisibilityScope::Owner, 8)),
            Err(match state {
                RewardFamilyState::Unsupported => RewardCatalogError::UnsupportedFamily,
                RewardFamilyState::Unavailable => RewardCatalogError::UnavailableFamily,
                RewardFamilyState::Handled => unreachable!(),
            })
        );
    }
}

#[test]
fn non_handled_family_with_definitions_is_rejected() {
    let content = reward_manifest();
    let mut parts = snapshot(
        &content,
        vec![simple_reward(
            "reward:bulk",
            "item:one",
            RewardSemanticReferenceKind::Card,
            "card:strike",
        )],
    );
    parts.family.state = RewardFamilyState::Unsupported;
    assert_eq!(
        RewardCatalogProducer::new().produce(
            &content,
            &RewardSource {
                snapshot: Ok(parts),
            }
        ),
        Err(RewardCatalogError::UnknownDefinition(
            "reward:bulk".to_owned()
        ))
    );
}

#[test]
fn manifest_locale_producer_and_family_fences_are_enforced() {
    let content = reward_manifest();
    let good = simple_reward(
        "reward:bulk",
        "item:one",
        RewardSemanticReferenceKind::Card,
        "card:strike",
    );

    let mut mismatched_manifest = snapshot(&content, vec![good.clone()]);
    mismatched_manifest.manifest.catalog_generation = 999;
    assert_eq!(
        produce_snapshot(&content, mismatched_manifest),
        Err(RewardCatalogError::ManifestMismatch)
    );

    let mut locale = snapshot(&content, vec![good.clone()]);
    locale.locale = "fr-FR".to_owned();
    assert_eq!(
        produce_snapshot(&content, locale),
        Err(RewardCatalogError::LocaleMismatch)
    );

    let mut producer = snapshot(&content, vec![good.clone()]);
    producer.producer_version = "other".to_owned();
    assert_eq!(
        produce_snapshot(&content, producer),
        Err(RewardCatalogError::ProducerVersionMismatch)
    );

    let mut family = snapshot(&content, vec![good.clone()]);
    family.family.entity_kind = "not-reward".to_owned();
    assert_eq!(
        produce_snapshot(&content, family),
        Err(RewardCatalogError::FamilyIdentityMismatch)
    );

    let mut count = snapshot(&content, vec![good]);
    count.family.definition_count = 5;
    assert_eq!(
        produce_snapshot(&content, count),
        Err(RewardCatalogError::FamilyCountMismatch)
    );
}

fn produce_snapshot(
    content: &sts2_game_mod::ContentManifest,
    parts: RewardCatalogSnapshot,
) -> Result<RewardCatalog, RewardCatalogError> {
    RewardCatalogProducer::new().produce(
        content,
        &RewardSource {
            snapshot: Ok(parts),
        },
    )
}

fn bulk_definition(kind_len: usize) -> RewardOfferDefinitionInput {
    let items = (0..REWARD_MAX_ITEMS)
        .map(|index| {
            item(
                &format!("item:{index}"),
                RewardSemanticReferenceKind::Card,
                "card:strike",
                1,
                RewardVisibility::Visible,
            )
        })
        .collect::<Vec<_>>();
    let rules = (0..REWARD_MAX_RULES)
        .map(|index| {
            let item_id = format!("item:{index}");
            let mut generation = rule(&format!("rule:{index}"), &[item_id.as_str()]);
            generation.eligibility = (0..REWARD_MAX_REQUIREMENTS)
                .map(|requirement_index| {
                    let mut req = requirement(
                        &format!("req:{index}:{requirement_index}"),
                        RewardRequirementKind::Custom("k".repeat(kind_len)),
                    );
                    req.parameters.clear();
                    req
                })
                .collect();
            generation
        })
        .collect::<Vec<_>>();
    RewardOfferDefinitionInput {
        reward_id: "reward:bulk".to_owned(),
        label: text("Bulk"),
        kind: sts2_game_mod::RewardKind::Card,
        unlock_state: sts2_game_mod::ContentUnlockState::Unlocked,
        visibility: RewardVisibility::Visible,
        selection: selection("group:bulk", 1, 1, true),
        items,
        generation: rules,
        state_policy: state_policy(),
        references: Vec::new(),
    }
}

fn bulk_actual(kind_len: usize) -> usize {
    let content = reward_manifest();
    match produce(&content, bulk_definition(kind_len)) {
        Err(RewardCatalogError::DefinitionTooLarge { actual, .. }) => actual,
        other => {
            assert!(matches!(
                other,
                Err(RewardCatalogError::DefinitionTooLarge { .. })
            ));
            0
        }
    }
}

#[test]
fn definition_byte_limit_counts_nested_custom_kind_strings() {
    let long = bulk_actual(REWARD_MAX_IDENTITY_BYTES);
    let short = bulk_actual(REWARD_MAX_IDENTITY_BYTES - 6);
    assert!(long > REWARD_MAX_DEFINITION_BYTES);
    assert_eq!(
        long - short,
        REWARD_MAX_RULES * REWARD_MAX_REQUIREMENTS * 6,
        "every nested custom requirement-kind byte must count toward the definition bound"
    );
}
