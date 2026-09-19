// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/settings_reference.rs"]
mod support;

use sts2_game_mod::{
    SettingValue, SettingsCategory, SettingsDefinitionReference, SettingsFieldStatus,
    SettingsLevel, SettingsListQuery, SettingsProfileKind, SettingsReferenceError,
    SettingsRestartState, SettingsValueType, SettingsVisibility, SettingsVisibilityScope,
};
use support::{
    SettingsFixtureSource, manifest, produce, profile, setting, snapshot, with_range, with_values,
};

fn fixture() -> (sts2_game_mod::ContentManifest, SettingsFixtureSource) {
    let ids = ["setting.display.scale", "setting.input.confirm_keys"];
    let manifest = manifest(&[], &ids);
    let mut scale = setting(
        "setting.display.scale",
        SettingsCategory::Display,
        SettingsLevel::Profile,
        SettingsValueType::Integer,
    );
    scale = with_range(scale, 50, 200, Some(10));
    scale = with_values(
        scale,
        SettingValue::Integer(100),
        SettingValue::Integer(100),
    );
    let mut keys = setting(
        "setting.input.confirm_keys",
        SettingsCategory::Input,
        SettingsLevel::Profile,
        SettingsValueType::KeyBinding,
    );
    keys = with_values(
        keys,
        SettingValue::Keys(vec!["key:a".to_owned()]),
        SettingValue::Keys(vec!["key:b".to_owned()]),
    );
    keys.restart = SettingsRestartState::Required;
    let definitions = vec![scale, keys];
    let snapshot = snapshot(
        &manifest,
        definitions,
        profile("profile:one", SettingsProfileKind::Named),
    );
    (manifest, SettingsFixtureSource::new(snapshot))
}

#[test]
fn produces_a_fenced_catalog_and_reads_every_setting_once() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    assert_eq!(source.reads(), 1);
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(catalog.profile().profile_id, "profile:one");
    assert_eq!(catalog.binding().manifest, manifest.cursor_binding());
    assert_eq!(catalog.family().definition_count, 2);

    let mut reader = catalog.reader();
    let page = reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.total, 2);
    assert!(page.complete);
    assert_eq!(
        page.entries[0].reference.setting_id,
        "setting.display.scale"
    );
    assert_eq!(
        page.entries[1].reference.setting_id,
        "setting.input.confirm_keys"
    );
    assert_eq!(page.entries[0].stored_value, SettingsFieldStatus::Available);
    assert_eq!(
        page.entries[0].effective_value,
        SettingsFieldStatus::Available
    );
}

#[test]
fn distinguishes_stored_from_effective_and_restart_requirement() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let reference = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.input.confirm_keys".to_owned(),
    };
    let definition = catalog
        .get(&reference, SettingsVisibilityScope::Public)
        .expect("definition");
    assert_eq!(
        definition.stored_value.get(),
        Some(&SettingValue::Keys(vec!["key:a".to_owned()]))
    );
    assert_eq!(
        definition.effective_value.get(),
        Some(&SettingValue::Keys(vec!["key:b".to_owned()]))
    );
    assert_ne!(definition.stored_value, definition.effective_value);
    assert_eq!(definition.restart, SettingsRestartState::Required);
    assert_eq!(definition.level, SettingsLevel::Profile);
}

#[test]
fn rejects_an_unknown_setting_identity() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let reference = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.display.missing".to_owned(),
    };
    assert_eq!(
        catalog.get(&reference, SettingsVisibilityScope::Public),
        Err(SettingsReferenceError::NotFound)
    );
}

#[test]
fn a_catalog_from_another_profile_is_a_stale_reference() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let other = sts2_game_mod::SettingsCatalogBinding {
        profile: profile("profile:two", SettingsProfileKind::Named),
        ..catalog.binding().clone()
    };
    let reference = SettingsDefinitionReference {
        catalog: other,
        setting_id: "setting.display.scale".to_owned(),
    };
    assert_eq!(
        catalog.get(&reference, SettingsVisibilityScope::Public),
        Err(SettingsReferenceError::StaleReference)
    );
}

#[test]
fn a_locale_that_does_not_match_the_catalog_is_refused() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&SettingsListQuery {
            locale: "fr-FR".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(SettingsReferenceError::LocaleMismatch)
    );
}

#[test]
fn pages_are_bounded_and_continuations_are_single_use() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let mut reader = catalog.reader();
    let first = reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 1,
            continuation: None,
        })
        .expect("first page");
    assert!(!first.complete);
    let continuation = first.continuation.expect("continuation");
    let replay = continuation.clone();
    let second = reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 1,
            continuation: Some(continuation),
        })
        .expect("second page");
    assert!(second.complete);
    assert_eq!(
        second.entries[0].reference.setting_id,
        "setting.input.confirm_keys"
    );
    assert_eq!(
        reader.list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 1,
            continuation: Some(replay),
        }),
        Err(SettingsReferenceError::InvalidContinuation)
    );
}

#[test]
fn a_zero_or_oversized_page_is_refused() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let mut reader = catalog.reader();
    for limit in [0, 65] {
        assert_eq!(
            reader.list(&SettingsListQuery {
                locale: "en-US".to_owned(),
                category: None,
                level: None,
                scope: SettingsVisibilityScope::Public,
                limit,
                continuation: None,
            }),
            Err(SettingsReferenceError::InvalidPageSize)
        );
    }
}

#[test]
fn category_and_level_filters_are_stable_and_do_not_widen_scope() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let mut reader = catalog.reader();
    let page = reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: Some(SettingsCategory::Input),
            level: Some(SettingsLevel::Profile),
            scope: SettingsVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.total, 1);
    assert_eq!(
        page.entries[0].reference.setting_id,
        "setting.input.confirm_keys"
    );
    let empty = reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: Some(SettingsCategory::Audio),
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("page");
    assert_eq!(empty.total, 0);
    assert_eq!(empty.entries.len(), 0);
}

#[test]
fn repeated_reads_return_identical_values_and_never_write() {
    let (manifest, source) = fixture();
    let catalog = produce(&manifest, &source).expect("catalog");
    let first = catalog
        .summaries("en-US", SettingsVisibilityScope::Public)
        .expect("first");
    let second = catalog
        .summaries("en-US", SettingsVisibilityScope::Public)
        .expect("second");
    assert_eq!(first, second);
    assert_eq!(source.reads(), 1);
    let reference = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.display.scale".to_owned(),
    };
    let definition = catalog
        .get(&reference, SettingsVisibilityScope::Public)
        .expect("definition");
    assert_eq!(
        definition.stored_value.get(),
        Some(&SettingValue::Integer(100))
    );
    assert_eq!(
        definition
            .constraints
            .iter()
            .map(|constraint| constraint.kind())
            .collect::<Vec<_>>(),
        vec!["range"]
    );
}

#[test]
fn a_private_setting_is_discoverable_only_in_the_owner_scope_and_never_readable() {
    let ids = ["setting.gameplay.telemetry_endpoint"];
    let manifest = manifest(&[], &ids);
    let private = support::withheld(
        setting(
            "setting.gameplay.telemetry_endpoint",
            SettingsCategory::GameplayInteraction,
            SettingsLevel::Global,
            SettingsValueType::Text,
        ),
        true,
    );
    let snapshot = snapshot(
        &manifest,
        vec![private],
        profile("profile:default", SettingsProfileKind::Default),
    );
    let source = SettingsFixtureSource::new(snapshot);
    let catalog = produce(&manifest, &source).expect("catalog");
    let reference = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.gameplay.telemetry_endpoint".to_owned(),
    };
    assert_eq!(
        catalog.get(&reference, SettingsVisibilityScope::Public),
        Err(SettingsReferenceError::ExcludedByScope)
    );
    let definition = catalog
        .get(&reference, SettingsVisibilityScope::Owner)
        .expect("owner definition");
    assert_eq!(definition.visibility, SettingsVisibility::OwnerOnly);
    assert_eq!(
        definition.stored_value.status(),
        SettingsFieldStatus::Withheld
    );
    assert_eq!(definition.effective_value.get(), None);
    assert_eq!(definition.default_value.get(), None);
}
