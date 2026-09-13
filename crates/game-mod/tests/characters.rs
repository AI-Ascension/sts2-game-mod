// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/characters.rs"]
mod fixture;

use fixture::{CatalogSource, catalog, definitions, manifest, snapshot};
use sts2_game_mod::{
    CHARACTER_PRODUCER_VERSION, CharacterCatalogError, CharacterCatalogProducer,
    CharacterCatalogSnapshot, CharacterFamilyCoverage, CharacterFamilyState, CharacterFieldStatus,
    CharacterListQuery, CharacterSourceError, CharacterText, CharacterVisibilityScope,
    ContentUnlockState,
};

fn query(locale: &str, scope: CharacterVisibilityScope, limit: usize) -> CharacterListQuery {
    CharacterListQuery {
        locale: locale.to_owned(),
        scope,
        limit,
        continuation: None,
    }
}

#[test]
fn producer_reads_manifest_bound_localized_definitions_and_starting_variants() {
    let manifest = manifest();
    let source = CatalogSource {
        snapshot: Ok(snapshot(&manifest)),
    };
    let before = source.clone();
    let catalog = CharacterCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");

    assert_eq!(catalog.family().definition_count, 3);
    assert_eq!(catalog.binding().locale, "en-US");
    assert_eq!(
        catalog.binding().producer_version,
        CHARACTER_PRODUCER_VERSION
    );
    assert_eq!(source, before);

    let mut reader = catalog.reader();
    let page = reader
        .list(&query("en-US", CharacterVisibilityScope::Public, 8))
        .expect("public page");
    assert_eq!(page.total, 1);
    assert_eq!(
        page.entries[0].reference.character_id,
        "base:character:ember"
    );
    assert_eq!(page.entries[0].loadout_count, 2);
    assert_eq!(page.entries[0].available_loadout_count, 2);
    assert_eq!(
        page.entries[0].name,
        CharacterText::Available("Émbér".to_owned())
    );

    let ember = catalog
        .get(&page.entries[0].reference, CharacterVisibilityScope::Public)
        .expect("ember");
    assert_eq!(ember.loadouts[0].starting.starting_hp, fixed(70));
    assert_eq!(
        ember.loadouts[0].starting_deck.value().expect("deck")[0].quantity,
        4
    );
    assert_eq!(
        ember.loadouts[0].starting_relics.value().expect("relics")[0].entity_kind,
        "relic"
    );
    assert_eq!(
        ember.loadouts[0].pools.value().expect("pools")[0].pool_id,
        "pool:ember"
    );
    assert_eq!(
        ember.loadouts[0].mechanics.value().expect("mechanics")[0].dependencies,
        vec!["resource:ember"]
    );
    assert_eq!(
        ember.loadouts[1]
            .prerequisites
            .value()
            .expect("prerequisites")[0]
            .progression_id,
        "mode:practice"
    );
    assert!(matches!(
        ember.loadouts[1].starting.starting_hp,
        sts2_game_mod::CharacterNumericValue::Formula(_)
    ));
    assert_eq!(
        catalog
            .reader()
            .list(&query("en-US", CharacterVisibilityScope::Reference, 8))
            .expect("reference page")
            .entries
            .iter()
            .find(|entry| entry.reference.character_id == "mod:character:locked")
            .expect("locked summary")
            .available_loadout_count,
        0
    );
}

#[test]
fn reader_lists_locked_references_but_keeps_unknown_unlock_state_hidden() {
    let catalog = catalog();
    let mut reader = catalog.reader();
    let first = reader
        .list(&query("en-US", CharacterVisibilityScope::Reference, 1))
        .expect("first page");
    assert_eq!(first.total, 2);
    assert!(!first.complete);
    assert_eq!(
        first.entries[0].reference.character_id,
        "base:character:ember"
    );
    let continuation = first.continuation.expect("continuation");
    let second = reader
        .list(&CharacterListQuery {
            continuation: Some(continuation),
            ..query("en-US", CharacterVisibilityScope::Reference, 1)
        })
        .expect("second page");
    assert!(second.complete);
    assert_eq!(second.entries[0].unlock_state, ContentUnlockState::Locked);
    assert_eq!(
        second.entries[0].unlock_availability,
        CharacterFieldStatus::Available
    );

    let locked_reference = second.entries[0].reference.clone();
    let locked = catalog
        .get(&locked_reference, CharacterVisibilityScope::Reference)
        .expect("locked detail");
    assert_eq!(
        locked
            .unlock
            .value()
            .expect("unlock")
            .requirements
            .value()
            .expect("requirements")[0]
            .progression_id,
        "progression:unlock-locked"
    );
    assert_eq!(
        locked.loadouts[0].starting_deck.status(),
        CharacterFieldStatus::Available
    );
    assert_eq!(
        locked.loadouts[0].starting_relics.status(),
        CharacterFieldStatus::Available
    );
    assert_eq!(
        locked.loadouts[0].pools.status(),
        CharacterFieldStatus::Available
    );
    assert_eq!(
        locked.loadouts[0].mechanics.status(),
        CharacterFieldStatus::Available
    );
    assert_eq!(
        locked.loadouts[0].starting.resources.status(),
        CharacterFieldStatus::Available
    );
    assert_eq!(
        locked.unlock.value().expect("unlock").requirements.status(),
        CharacterFieldStatus::Available
    );
    assert_eq!(
        catalog.get(&locked_reference, CharacterVisibilityScope::Public),
        Err(CharacterCatalogError::ExcludedByScope)
    );
}

#[test]
fn producer_fences_locale_manifest_source_and_reference_identity() {
    let manifest = manifest();
    let mut wrong_locale = snapshot(&manifest);
    wrong_locale.locale = "fr-FR".to_owned();
    assert_eq!(
        CharacterCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(wrong_locale),
            }
        ),
        Err(CharacterCatalogError::LocaleMismatch)
    );

    let mut wrong_manifest = snapshot(&manifest);
    wrong_manifest.manifest.catalog_generation += 1;
    assert_eq!(
        CharacterCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(wrong_manifest),
            }
        ),
        Err(CharacterCatalogError::ManifestMismatch)
    );

    assert_eq!(
        CharacterCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Err(CharacterSourceError::NoActiveSource),
            }
        ),
        Err(CharacterCatalogError::NoActiveSource)
    );

    let catalog = catalog();
    assert_eq!(
        catalog
            .reader()
            .list(&query("fr-FR", CharacterVisibilityScope::Public, 8)),
        Err(CharacterCatalogError::LocaleMismatch)
    );
    let public = catalog
        .reader()
        .list(&query("en-US", CharacterVisibilityScope::Public, 8))
        .expect("public");
    let mut stale = public.entries[0].reference.clone();
    stale.catalog.manifest.catalog_generation += 1;
    assert_eq!(
        catalog.get(&stale, CharacterVisibilityScope::Public),
        Err(CharacterCatalogError::StaleReference)
    );
}

#[test]
fn producer_rejects_unsupported_family_unknown_references_and_duplicates() {
    let manifest = manifest();
    let defs = definitions();
    let unsupported = CharacterCatalogProducer::new()
        .produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(CharacterCatalogSnapshot {
                    manifest: manifest.cursor_binding(),
                    locale: manifest.locale.clone(),
                    producer_version: CHARACTER_PRODUCER_VERSION.to_owned(),
                    family: CharacterFamilyCoverage {
                        entity_kind: "character".to_owned(),
                        state: CharacterFamilyState::Unsupported,
                        definition_count: defs.len(),
                    },
                    definitions: Vec::new(),
                }),
            },
        )
        .expect("unsupported catalog");
    assert_eq!(
        unsupported
            .reader()
            .list(&query("en-US", CharacterVisibilityScope::Reference, 8)),
        Err(CharacterCatalogError::UnsupportedFamily)
    );

    let mut unknown_reference = defs[0].clone();
    if let sts2_game_mod::CharacterField::Available(deck) =
        &mut unknown_reference.loadouts[0].starting_deck
    {
        deck[0].namespaced_id = "base:card:missing".to_owned();
    }
    let mut invalid = snapshot(&manifest);
    invalid.definitions = vec![unknown_reference, defs[1].clone(), defs[2].clone()];
    assert!(matches!(
        CharacterCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(invalid),
            }
        ),
        Err(CharacterCatalogError::UnknownLoadoutReference { .. })
    ));

    let mut duplicate = snapshot(&manifest);
    duplicate.definitions.push(defs[0].clone());
    assert_eq!(
        CharacterCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(duplicate),
            }
        ),
        Err(CharacterCatalogError::DuplicateDefinition(
            "base:character:ember".to_owned()
        ))
    );
}

#[test]
fn source_values_keep_uncertainty_explicit_and_reads_are_repeatable() {
    let catalog = catalog();
    let reference_page = catalog
        .reader()
        .list(&query("en-US", CharacterVisibilityScope::Reference, 8))
        .expect("reference page");
    let locked = reference_page
        .entries
        .iter()
        .find(|entry| entry.unlock_state == ContentUnlockState::Locked)
        .expect("locked")
        .reference
        .clone();
    let detail = catalog
        .get(&locked, CharacterVisibilityScope::Reference)
        .expect("locked detail");
    assert!(matches!(
        detail.loadouts[0].starting.starting_hp,
        sts2_game_mod::CharacterNumericValue::Unavailable(
            sts2_game_mod::CharacterUnavailableReason::NotObserved
        )
    ));
    assert_eq!(
        catalog
            .get(&locked, CharacterVisibilityScope::Reference)
            .expect("repeat read"),
        detail
    );
}

#[test]
fn unavailable_collections_are_distinct_from_observed_empty_collections() {
    let manifest = manifest();
    let source_snapshot = snapshot(&manifest);
    let unknown = source_snapshot
        .definitions
        .iter()
        .find(|definition| definition.character_id == "mod:character:unknown")
        .expect("unknown");
    assert_eq!(unknown.unlock.status(), CharacterFieldStatus::Unknown);
    assert_eq!(
        unknown.loadouts[0].starting_deck.status(),
        CharacterFieldStatus::NotObserved
    );
    assert_eq!(
        unknown.loadouts[0].starting_relics.status(),
        CharacterFieldStatus::NotObserved
    );
    assert_eq!(
        unknown.loadouts[0].pools.status(),
        CharacterFieldStatus::Unsupported
    );
    assert_eq!(
        unknown.loadouts[0].mechanics.status(),
        CharacterFieldStatus::Unsupported
    );
    assert_eq!(
        unknown.loadouts[0].starting.resources.status(),
        CharacterFieldStatus::Unknown
    );
    let locked = source_snapshot
        .definitions
        .iter()
        .find(|definition| definition.character_id == "mod:character:locked")
        .expect("locked");
    assert_eq!(
        locked.loadouts[0].starting_deck,
        sts2_game_mod::CharacterField::Available(Vec::new())
    );
    assert_eq!(
        locked.loadouts[0].starting_deck.status(),
        CharacterFieldStatus::Available
    );
}

#[test]
fn bounded_reader_and_text_payloads_fail_closed() {
    let catalog = catalog();
    assert_eq!(
        catalog
            .reader()
            .list(&query("en-US", CharacterVisibilityScope::Public, 0)),
        Err(CharacterCatalogError::InvalidPageSize)
    );
    assert_eq!(
        catalog
            .reader()
            .list(&query("en-US", CharacterVisibilityScope::Public, 65)),
        Err(CharacterCatalogError::InvalidPageSize)
    );

    let manifest = manifest();
    let mut oversized = snapshot(&manifest);
    oversized.definitions[0].description =
        CharacterText::Available("x".repeat(sts2_game_mod::CHARACTER_MAX_TEXT_BYTES + 1));
    assert!(matches!(
        CharacterCatalogProducer::new().produce(
            &manifest,
            &CatalogSource {
                snapshot: Ok(oversized),
            }
        ),
        Err(CharacterCatalogError::InvalidInput("description"))
    ));
}

fn fixed(value: i64) -> sts2_game_mod::CharacterNumericValue {
    sts2_game_mod::CharacterNumericValue::Fixed(value)
}
