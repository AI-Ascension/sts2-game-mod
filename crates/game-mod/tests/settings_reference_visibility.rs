// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/settings_reference.rs"]
mod support;

use sts2_game_mod::{
    SettingValue, SettingsCategory, SettingsDefinitionReference, SettingsFieldStatus,
    SettingsLevel, SettingsListQuery, SettingsProfileKind, SettingsValueType, SettingsVisibility,
    SettingsVisibilityScope,
};
use support::{
    SettingsFixtureSource, manifest, produce, profile, setting, snapshot, with_run_link,
    with_values, withheld,
};

fn scope_matrix() -> (sts2_game_mod::ContentManifest, SettingsFixtureSource) {
    let ids = [
        "setting.display.public",
        "setting.display.owner_only",
        "setting.gameplay.secret_endpoint",
        "setting.display.hidden",
    ];
    let manifest = manifest(&[], &ids);
    let public = with_values(
        setting(
            "setting.display.public",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
        SettingValue::Boolean(true),
        SettingValue::Boolean(true),
    );
    let mut owner_only = with_values(
        setting(
            "setting.display.owner_only",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
        SettingValue::Boolean(false),
        SettingValue::Boolean(false),
    );
    owner_only.visibility = SettingsVisibility::OwnerOnly;
    let private = withheld(
        setting(
            "setting.gameplay.secret_endpoint",
            SettingsCategory::GameplayInteraction,
            SettingsLevel::Global,
            SettingsValueType::Text,
        ),
        true,
    );
    let hidden = withheld(
        setting(
            "setting.display.hidden",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
        false,
    );
    let definitions = vec![public, owner_only, private, hidden];
    let snapshot = snapshot(
        &manifest,
        definitions,
        profile("profile:default", SettingsProfileKind::Default),
    );
    (manifest, SettingsFixtureSource::new(snapshot))
}

fn list_ids(
    catalog: &sts2_game_mod::SettingsCatalog,
    scope: SettingsVisibilityScope,
) -> Vec<String> {
    let mut reader = catalog.reader();
    reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope,
            limit: 64,
            continuation: None,
        })
        .expect("page")
        .entries
        .into_iter()
        .map(|entry| entry.reference.setting_id)
        .collect()
}

#[test]
fn visibility_scopes_widen_without_exposing_hidden_or_private_values() {
    let (manifest, source) = scope_matrix();
    let catalog = produce(&manifest, &source).expect("catalog");
    assert_eq!(
        list_ids(&catalog, SettingsVisibilityScope::Public),
        vec!["setting.display.public".to_owned()]
    );
    assert_eq!(
        list_ids(&catalog, SettingsVisibilityScope::Reference),
        vec![
            "setting.display.owner_only".to_owned(),
            "setting.display.public".to_owned()
        ]
    );
    assert_eq!(
        list_ids(&catalog, SettingsVisibilityScope::Owner),
        vec![
            "setting.display.owner_only".to_owned(),
            "setting.display.public".to_owned(),
            "setting.gameplay.secret_endpoint".to_owned()
        ]
    );
    let hidden = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.display.hidden".to_owned(),
    };
    assert!(
        catalog
            .get(&hidden, SettingsVisibilityScope::Owner)
            .is_err()
    );
    let private = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.gameplay.secret_endpoint".to_owned(),
    };
    let definition = catalog
        .get(&private, SettingsVisibilityScope::Owner)
        .expect("owner definition");
    assert_eq!(
        definition.stored_value.status(),
        SettingsFieldStatus::Withheld
    );
    assert_eq!(
        definition.effective_value.status(),
        SettingsFieldStatus::Withheld
    );
}

#[test]
fn a_resolved_run_configuration_link_is_reported_as_available() {
    let manifest = manifest(
        &[("run_configuration", "run_configuration:standard")],
        &["setting.gameplay.difficulty_override"],
    );
    let definition = with_run_link(
        setting(
            "setting.gameplay.difficulty_override",
            SettingsCategory::GameplayInteraction,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
        "run_configuration:standard",
    );
    let snapshot = snapshot(
        &manifest,
        vec![definition],
        profile("profile:default", SettingsProfileKind::Default),
    );
    let catalog = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect("catalog");
    let reference = SettingsDefinitionReference {
        catalog: catalog.binding().clone(),
        setting_id: "setting.gameplay.difficulty_override".to_owned(),
    };
    let resolved = catalog
        .get(&reference, SettingsVisibilityScope::Public)
        .expect("definition");
    assert_eq!(resolved.run_link.status(), SettingsFieldStatus::Available);
    assert!(matches!(
        resolved.run_link,
        sts2_game_mod::RunConfigurationLink::RunAffecting { ref configuration_id }
            if configuration_id == "run_configuration:standard"
    ));
}

#[test]
fn global_and_per_profile_settings_are_both_represented() {
    let ids = ["setting.audio.master", "setting.audio.profile_volume"];
    let manifest = manifest(&[], &ids);
    let global = with_values(
        setting(
            "setting.audio.master",
            SettingsCategory::Audio,
            SettingsLevel::Global,
            SettingsValueType::Integer,
        ),
        SettingValue::Integer(80),
        SettingValue::Integer(80),
    );
    let per_profile = with_values(
        setting(
            "setting.audio.profile_volume",
            SettingsCategory::Audio,
            SettingsLevel::Profile,
            SettingsValueType::Integer,
        ),
        SettingValue::Integer(30),
        SettingValue::Integer(20),
    );
    let alpha_snapshot = snapshot(
        &manifest,
        vec![global, per_profile],
        profile("profile:alpha", SettingsProfileKind::Named),
    );
    let catalog = produce(&manifest, &SettingsFixtureSource::new(alpha_snapshot)).expect("catalog");
    let mut reader = catalog.reader();
    let page = reader
        .list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: Some(SettingsCategory::Audio),
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 64,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.total, 2);
    assert_eq!(page.entries[0].level, SettingsLevel::Global);
    assert_eq!(page.entries[1].level, SettingsLevel::Profile);
    assert_eq!(catalog.profile().profile_id, "profile:alpha");

    let other_profile = snapshot(
        &manifest,
        vec![
            with_values(
                setting(
                    "setting.audio.master",
                    SettingsCategory::Audio,
                    SettingsLevel::Global,
                    SettingsValueType::Integer,
                ),
                SettingValue::Integer(80),
                SettingValue::Integer(80),
            ),
            with_values(
                setting(
                    "setting.audio.profile_volume",
                    SettingsCategory::Audio,
                    SettingsLevel::Profile,
                    SettingsValueType::Integer,
                ),
                SettingValue::Integer(30),
                SettingValue::Integer(30),
            ),
        ],
        profile("profile:beta", SettingsProfileKind::Named),
    );
    let other = produce(&manifest, &SettingsFixtureSource::new(other_profile)).expect("catalog");
    assert_ne!(catalog.binding(), other.binding());
    assert_eq!(
        catalog.get(
            &SettingsDefinitionReference {
                catalog: other.binding().clone(),
                setting_id: "setting.audio.master".to_owned(),
            },
            SettingsVisibilityScope::Public
        ),
        Err(sts2_game_mod::SettingsReferenceError::StaleReference)
    );
}

#[test]
fn producing_twice_from_one_source_is_deterministic_and_read_only() {
    let (manifest, source) = scope_matrix();
    let first = produce(&manifest, &source).expect("catalog");
    let second = produce(&manifest, &source).expect("catalog");
    assert_eq!(first, second);
    assert_eq!(source.reads(), 2);
    let reference = SettingsDefinitionReference {
        catalog: first.binding().clone(),
        setting_id: "setting.display.public".to_owned(),
    };
    let definition = first
        .get(&reference, SettingsVisibilityScope::Public)
        .expect("definition");
    assert_eq!(
        definition.stored_value.get(),
        Some(&SettingValue::Boolean(true))
    );
    assert_eq!(definition.label.value(), Some("setting.display.public"));
    let _ = &reference;
}
