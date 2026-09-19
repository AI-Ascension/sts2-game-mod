// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/settings_reference.rs"]
mod support;

use sts2_game_mod::{
    SETTINGS_REFERENCE_ENTITY_KIND, SETTINGS_REFERENCE_PRODUCER_VERSION, SettingsCategory,
    SettingsFamilyState, SettingsLevel, SettingsListQuery, SettingsProfileKind,
    SettingsReferenceError, SettingsSemanticReference, SettingsSemanticReferenceKind, SettingsText,
    SettingsValueType, SettingsVisibilityScope,
};
use support::{
    FailingSource, Failure, SettingsFixtureSource, empty_family_manifest, manifest, manifest_of,
    produce, profile, setting, snapshot,
};

fn fixture(ids: &[&str]) -> (sts2_game_mod::ContentManifest, SettingsFixtureSource) {
    let manifest = manifest(&[], ids);
    let definitions = ids
        .iter()
        .map(|id| {
            setting(
                id,
                SettingsCategory::Display,
                SettingsLevel::Global,
                SettingsValueType::Boolean,
            )
        })
        .collect();
    let snapshot = snapshot(
        &manifest,
        definitions,
        profile("profile:default", SettingsProfileKind::Default),
    );
    (manifest, SettingsFixtureSource::new(snapshot))
}

#[test]
fn a_manifest_without_the_settings_family_is_refused() {
    let manifest = manifest_of(&[("card", "card:strike")]);
    let snapshot = snapshot(
        &manifest,
        Vec::new(),
        profile("profile:default", SettingsProfileKind::Default),
    );
    let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
    assert_eq!(error, SettingsReferenceError::MissingFamily);
}

#[test]
fn a_family_count_disagreement_is_refused() {
    let (manifest, source) = fixture(&["setting.a", "setting.b"]);
    let mut snapshot = source.snapshot.clone();
    snapshot.family.definition_count = 3;
    let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
    assert_eq!(error, SettingsReferenceError::FamilyCountMismatch);
}

#[test]
fn every_catalog_fence_is_enforced() {
    let (manifest, source) = fixture(&["setting.a"]);
    let mut cases = Vec::new();
    let mut wrong_manifest = source.snapshot.clone();
    wrong_manifest.manifest.adapter_compatibility = "adapter-v2".to_owned();
    cases.push((wrong_manifest, SettingsReferenceError::ManifestMismatch));
    let mut wrong_locale = source.snapshot.clone();
    wrong_locale.locale = "fr-FR".to_owned();
    cases.push((wrong_locale, SettingsReferenceError::LocaleMismatch));
    let mut wrong_producer = source.snapshot.clone();
    wrong_producer.producer_version = "other-producer".to_owned();
    cases.push((
        wrong_producer,
        SettingsReferenceError::ProducerVersionMismatch,
    ));
    let mut wrong_family = source.snapshot.clone();
    wrong_family.family.entity_kind = "other".to_owned();
    cases.push((wrong_family, SettingsReferenceError::FamilyIdentityMismatch));
    for (snapshot, expected) in cases {
        let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
        assert_eq!(error, expected);
    }
    assert_eq!(
        SETTINGS_REFERENCE_PRODUCER_VERSION,
        source.snapshot.producer_version
    );
}

#[test]
fn a_source_definition_outside_the_manifest_is_refused() {
    let manifest = manifest(&[], &["setting.a", "setting.b"]);
    let definitions = vec![
        setting(
            "setting.a",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
        setting(
            "setting.c",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
    ];
    let snapshot = snapshot(
        &manifest,
        definitions,
        profile("profile:default", SettingsProfileKind::Default),
    );
    let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
    assert_eq!(
        error,
        SettingsReferenceError::UnknownDefinition("setting.c".to_owned())
    );
}

#[test]
fn a_duplicate_source_definition_is_refused() {
    let manifest = manifest(&[], &["setting.a"]);
    let definitions = vec![
        setting(
            "setting.a",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
        setting(
            "setting.a",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Boolean,
        ),
    ];
    let mut snapshot = snapshot(
        &manifest,
        definitions,
        profile("profile:default", SettingsProfileKind::Default),
    );
    snapshot.family.definition_count = 1;
    let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
    assert_eq!(
        error,
        SettingsReferenceError::DuplicateDefinition("setting.a".to_owned())
    );
}

#[test]
fn source_failures_are_mapped_without_host_detail() {
    let (manifest, _) = fixture(&["setting.a"]);
    for (failure, expected) in [
        (
            Failure::NoActiveSource,
            SettingsReferenceError::NoActiveSource,
        ),
        (
            Failure::AccessDenied,
            SettingsReferenceError::SourceAccessDenied,
        ),
        (Failure::Malformed, SettingsReferenceError::MalformedSource),
    ] {
        let error = produce(&manifest, &FailingSource(failure)).expect_err("refused");
        assert_eq!(error, expected);
    }
    assert_eq!(SETTINGS_REFERENCE_ENTITY_KIND, "setting");
}

#[test]
fn an_unsupported_family_reports_its_state_and_projects_nothing() {
    let manifest = empty_family_manifest();
    let mut snapshot = snapshot(
        &manifest,
        Vec::new(),
        profile("profile:default", SettingsProfileKind::Default),
    );
    snapshot.family.state = SettingsFamilyState::Unsupported;
    let catalog =
        produce(&manifest, &SettingsFixtureSource::new(snapshot.clone())).expect("catalog");
    assert_eq!(catalog.family().state, SettingsFamilyState::Unsupported);
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(SettingsReferenceError::UnsupportedFamily)
    );
    snapshot.family.state = SettingsFamilyState::Unavailable;
    let catalog = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect("catalog");
    let mut reader = catalog.reader();
    assert_eq!(
        reader.list(&SettingsListQuery {
            locale: "en-US".to_owned(),
            category: None,
            level: None,
            scope: SettingsVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(SettingsReferenceError::UnavailableFamily)
    );
}

#[test]
fn an_unresolved_content_reference_is_refused() {
    let manifest = manifest(&[("card", "card:strike")], &["setting.a"]);
    let mut definition = setting(
        "setting.a",
        SettingsCategory::GameplayInteraction,
        SettingsLevel::Global,
        SettingsValueType::Boolean,
    );
    definition.references.push(SettingsSemanticReference {
        kind: SettingsSemanticReferenceKind::Content {
            entity_kind: "card".to_owned(),
        },
        id: "card:missing".to_owned(),
        label: SettingsText::available("card:missing").expect("label"),
    });
    let snapshot = snapshot(
        &manifest,
        vec![definition],
        profile("profile:default", SettingsProfileKind::Default),
    );
    let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
    assert_eq!(
        error,
        SettingsReferenceError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card:missing".to_owned(),
        }
    );
}

#[test]
fn a_run_configuration_link_must_resolve_in_the_manifest() {
    let manifest = manifest(&[], &["setting.a"]);
    let definition = support::with_run_link(
        setting(
            "setting.a",
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
    let error = produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused");
    assert_eq!(
        error,
        SettingsReferenceError::UnknownManifestReference {
            entity_kind: support::run_kind().to_owned(),
            namespaced_id: "run_configuration:standard".to_owned(),
        }
    );
}
