// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/enemies.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentUnlockState, ENEMY_MAX_PAGE_ITEMS, ENEMY_PRODUCER_VERSION, EnemyCatalogError,
    EnemyCooldownRule, EnemyFamilyState, EnemyKind, EnemyListQuery, EnemyMoveListQuery,
    EnemyNumericValue, EnemyProbability, EnemySourceError, EnemyVisibility, EnemyVisibilityScope,
};

fn list_query(locale: &str, scope: EnemyVisibilityScope, limit: usize) -> EnemyListQuery {
    EnemyListQuery {
        locale: locale.to_owned(),
        scope,
        limit,
        continuation: None,
    }
}

#[test]
fn catalog_binds_manifest_locale_and_producer_identity() {
    let content = manifest(
        &["enemy:boss", "enemy:minion"],
        &["encounter:boss"],
        &["power_status:weak"],
    );
    let catalog = catalog(
        &content,
        vec![
            boss("enemy:boss"),
            enemy(
                "enemy:minion",
                EnemyKind::Minion,
                EnemyVisibility::Visible,
                ContentUnlockState::Unlocked,
            ),
        ],
    );
    assert_eq!(catalog.binding().manifest, content.cursor_binding());
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(catalog.binding().producer_version, ENEMY_PRODUCER_VERSION);
    assert_eq!(catalog.family().entity_kind, "enemy");
    assert_eq!(catalog.family().state, EnemyFamilyState::Handled);
    assert_eq!(catalog.family().definition_count, 2);

    let reference = definition_reference(&catalog, "enemy:boss");
    assert_eq!(reference.catalog, *catalog.binding());
}

#[test]
fn deterministic_pages_expose_single_use_bound_continuations() {
    let ids = ["enemy:a", "enemy:b", "enemy:c"];
    let content = manifest(&ids, &[], &[]);
    let catalog = catalog(
        &content,
        ids.iter()
            .map(|id| {
                enemy(
                    id,
                    EnemyKind::Normal,
                    EnemyVisibility::Visible,
                    ContentUnlockState::Unlocked,
                )
            })
            .collect(),
    );
    let mut reader = catalog.reader();

    let first = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Owner, 2))
        .expect("first page");
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.enemy_id.as_str())
            .collect::<Vec<_>>(),
        ["enemy:a", "enemy:b"]
    );
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let token = first.continuation.expect("partial page continuation");

    let mut second_query = list_query("en-US", EnemyVisibilityScope::Owner, 2);
    second_query.continuation = Some(token);
    let second = reader.list(&second_query).expect("second page");
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.reference.enemy_id.as_str())
            .collect::<Vec<_>>(),
        ["enemy:c"]
    );
    assert_eq!(second.total, 3);
    assert!(second.complete);
    assert!(second.continuation.is_none());

    let stale = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Owner, 2))
        .expect("fresh page");
    let token = stale.continuation.expect("fresh continuation");
    let mut wrong_limit = list_query("en-US", EnemyVisibilityScope::Owner, 3);
    wrong_limit.continuation = Some(token);
    assert_eq!(
        reader.list(&wrong_limit),
        Err(EnemyCatalogError::InvalidContinuation)
    );

    let cross = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Owner, 2))
        .expect("cross page");
    let token = cross.continuation.expect("cross continuation");
    let mut foreign_reader = catalog.reader();
    let mut foreign_query = list_query("en-US", EnemyVisibilityScope::Owner, 2);
    foreign_query.continuation = Some(token);
    assert_eq!(
        foreign_reader.list(&foreign_query),
        Err(EnemyCatalogError::InvalidContinuation)
    );
}

#[test]
fn list_bounds_and_locale_are_enforced_without_clamping() {
    let content = manifest(&["enemy:a"], &[], &[]);
    let catalog = catalog(
        &content,
        vec![enemy(
            "enemy:a",
            EnemyKind::Normal,
            EnemyVisibility::Visible,
            ContentUnlockState::Unlocked,
        )],
    );
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&list_query("en-US", EnemyVisibilityScope::Public, 0)),
        Err(EnemyCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query(
            "en-US",
            EnemyVisibilityScope::Public,
            ENEMY_MAX_PAGE_ITEMS + 1
        )),
        Err(EnemyCatalogError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&list_query("fr-FR", EnemyVisibilityScope::Public, 8)),
        Err(EnemyCatalogError::LocaleMismatch)
    );
}

#[test]
fn exact_definition_move_phase_and_transition_lookup() {
    let content = manifest(&["enemy:boss"], &["encounter:boss"], &["power_status:weak"]);
    let catalog = catalog(&content, vec![boss("enemy:boss")]);
    let reference = definition_reference(&catalog, "enemy:boss");
    let definition = catalog
        .get(&reference, EnemyVisibilityScope::Owner)
        .expect("definition");
    assert_eq!(definition.reference, reference);
    assert_eq!(definition.kind, EnemyKind::Boss);
    assert_eq!(definition.phases.len(), 2);
    assert_eq!(definition.moves.len(), 3);
    assert_eq!(definition.transitions.len(), 1);
    assert_eq!(definition.origin_variants.len(), 1);
    assert_eq!(
        definition.stats.scaled.value().expect("scaled stats").len(),
        2
    );
    assert_eq!(definition.phases[0].phase_id, "phase:one");
    assert_eq!(
        definition.transitions[0].from_phase.as_deref(),
        Some("phase:one")
    );

    let move_ref = move_reference(&catalog, "enemy:boss", "move:slam");
    let movement = catalog
        .get_move(&move_ref, EnemyVisibilityScope::Owner)
        .expect("move");
    assert_eq!(movement.reference, move_ref);
    assert_eq!(movement.effects.len(), 2);
    assert_eq!(movement.phase_ids, vec!["phase:one"]);
    assert!(matches!(
        movement.cooldown,
        EnemyCooldownRule::Turns(EnemyNumericValue::Fixed(2))
    ));
    assert!(matches!(
        movement.probability,
        EnemyProbability::Exact {
            numerator: 1,
            denominator: 2,
            ..
        }
    ));
    assert_eq!(movement.effects[0].references[0].id, "power_status:weak");

    let missing = definition_reference(&catalog, "enemy:missing");
    assert_eq!(
        catalog.get(&missing, EnemyVisibilityScope::Owner),
        Err(EnemyCatalogError::NotFound)
    );
    let missing_move = move_reference(&catalog, "enemy:boss", "move:missing");
    assert_eq!(
        catalog.get_move(&missing_move, EnemyVisibilityScope::Owner),
        Err(EnemyCatalogError::NotFound)
    );
}

fn read_only_move_query(catalog: &sts2_game_mod::EnemyCatalog, limit: usize) -> EnemyMoveListQuery {
    EnemyMoveListQuery {
        enemy: definition_reference(catalog, "enemy:boss"),
        scope: EnemyVisibilityScope::Owner,
        limit,
        continuation: None,
    }
}

#[test]
fn move_pages_are_bounded_ordered_and_single_use() {
    let content = manifest(&["enemy:boss"], &["encounter:boss"], &["power_status:weak"]);
    let catalog = catalog(&content, vec![boss("enemy:boss")]);
    let mut reader = catalog.reader();
    let first = reader
        .list_moves(&read_only_move_query(&catalog, 2))
        .expect("first move page");
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.move_id.as_str())
            .collect::<Vec<_>>(),
        ["move:slam", "move:summon"]
    );
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let token = first.continuation.expect("move continuation");

    let mut second_query = read_only_move_query(&catalog, 2);
    second_query.continuation = Some(token);
    let second = reader.list_moves(&second_query).expect("second move page");
    assert_eq!(second.entries[0].reference.move_id, "move:enrage");
    assert!(second.complete);
    assert!(second.continuation.is_none());

    let mut wrong_enemy = read_only_move_query(&catalog, 2);
    wrong_enemy.enemy.enemy_id = "enemy:missing".to_owned();
    assert_eq!(
        reader.list_moves(&wrong_enemy),
        Err(EnemyCatalogError::NotFound)
    );
}

#[test]
fn visibility_scope_gates_definitions_and_moves() {
    let ids = [
        "enemy:public",
        "enemy:locked",
        "enemy:owner",
        "enemy:hidden",
        "enemy:moves",
    ];
    let content = manifest(&ids, &[], &[]);
    let public = enemy(
        "enemy:public",
        EnemyKind::Normal,
        EnemyVisibility::Visible,
        ContentUnlockState::Unlocked,
    );
    let locked = enemy(
        "enemy:locked",
        EnemyKind::Elite,
        EnemyVisibility::Visible,
        ContentUnlockState::Locked,
    );
    let owner = enemy(
        "enemy:owner",
        EnemyKind::Custom("rare".to_owned()),
        EnemyVisibility::OwnerOnly,
        ContentUnlockState::Unlocked,
    );
    let hidden = enemy(
        "enemy:hidden",
        EnemyKind::Minion,
        EnemyVisibility::Hidden,
        ContentUnlockState::Unlocked,
    );
    let mut moves = enemy(
        "enemy:moves",
        EnemyKind::Normal,
        EnemyVisibility::Visible,
        ContentUnlockState::Unlocked,
    );
    moves.moves[0].visibility = EnemyVisibility::OwnerOnly;
    let catalog = catalog(&content, vec![public, locked, owner, hidden, moves]);
    let mut reader = catalog.reader();

    let public_page = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Public, 8))
        .expect("public page");
    assert_eq!(public_page.total, 2);
    assert!(public_page.complete);

    let reference_page = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Reference, 8))
        .expect("reference page");
    assert_eq!(reference_page.total, 3);

    let owner_page = reader
        .list(&list_query("en-US", EnemyVisibilityScope::Owner, 8))
        .expect("owner page");
    assert_eq!(owner_page.total, 4);

    let locked_ref = definition_reference(&catalog, "enemy:locked");
    assert_eq!(
        catalog.get(&locked_ref, EnemyVisibilityScope::Public),
        Err(EnemyCatalogError::ExcludedByScope)
    );
    assert!(
        catalog
            .get(&locked_ref, EnemyVisibilityScope::Reference)
            .is_ok()
    );

    let owner_ref = definition_reference(&catalog, "enemy:owner");
    assert_eq!(
        catalog.get(&owner_ref, EnemyVisibilityScope::Reference),
        Err(EnemyCatalogError::ExcludedByScope)
    );
    assert!(catalog.get(&owner_ref, EnemyVisibilityScope::Owner).is_ok());

    let hidden_ref = definition_reference(&catalog, "enemy:hidden");
    assert_eq!(
        catalog.get(&hidden_ref, EnemyVisibilityScope::Owner),
        Err(EnemyCatalogError::ExcludedByScope)
    );

    let mut public_moves = read_only_move_query(&catalog, 8);
    public_moves.enemy.enemy_id = "enemy:moves".to_owned();
    public_moves.scope = EnemyVisibilityScope::Public;
    let page = reader.list_moves(&public_moves).expect("public moves");
    assert_eq!(page.total, 0);
    assert!(page.complete);
    let move_ref = move_reference(&catalog, "enemy:moves", "move:strike");
    assert_eq!(
        catalog.get_move(&move_ref, EnemyVisibilityScope::Public),
        Err(EnemyCatalogError::ExcludedByScope)
    );
    assert!(
        catalog
            .get_move(&move_ref, EnemyVisibilityScope::Owner)
            .is_ok()
    );
}

#[test]
fn stale_references_and_locale_bindings_are_rejected() {
    let first = manifest(&["enemy:a"], &[], &[]);
    let catalog = catalog(
        &first,
        vec![enemy(
            "enemy:a",
            EnemyKind::Normal,
            EnemyVisibility::Visible,
            ContentUnlockState::Unlocked,
        )],
    );
    let second = manifest(&["enemy:b"], &[], &[]);
    let reference = definition_reference(&catalog, "enemy:a");
    let mut stale = reference.clone();
    stale.catalog.manifest = second.cursor_binding();
    assert_eq!(
        catalog.get(&stale, EnemyVisibilityScope::Owner),
        Err(EnemyCatalogError::StaleReference)
    );
    let mut stale_locale = reference.clone();
    stale_locale.catalog.locale = "fr-FR".to_owned();
    assert_eq!(
        catalog.get(&stale_locale, EnemyVisibilityScope::Owner),
        Err(EnemyCatalogError::StaleReference)
    );

    assert_eq!(
        sts2_game_mod::EnemyCatalogProducer::new().produce(
            &first,
            &EnemySource {
                snapshot: Err(EnemySourceError::AccessDenied),
            }
        ),
        Err(EnemyCatalogError::SourceAccessDenied)
    );
}
