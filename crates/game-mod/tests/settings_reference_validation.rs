// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/settings_reference.rs"]
mod support;

use sts2_game_mod::{
    RunConfigurationLink, SettingConstraint, SettingDefinitionInput, SettingOption, SettingValue,
    SettingValueField, SettingsCategory, SettingsEvidence, SettingsLevel, SettingsProfileKind,
    SettingsReadSeam, SettingsReferenceError, SettingsSemanticReference,
    SettingsSemanticReferenceKind, SettingsSensitivity, SettingsText, SettingsUnavailableReason,
    SettingsValueType, SettingsVisibility,
};
use support::{
    SettingsFixtureSource, manifest, produce, profile, seam, setting, snapshot, with_options,
    with_range, with_values,
};

fn bool_setting(setting_id: &str) -> SettingDefinitionInput {
    setting(
        setting_id,
        SettingsCategory::Display,
        SettingsLevel::Global,
        SettingsValueType::Boolean,
    )
}

fn reject(definition: SettingDefinitionInput) -> SettingsReferenceError {
    let manifest = manifest(&[], &["setting.a"]);
    let snapshot = snapshot(
        &manifest,
        vec![definition],
        profile("profile:default", SettingsProfileKind::Default),
    );
    produce(&manifest, &SettingsFixtureSource::new(snapshot)).expect_err("refused")
}

#[test]
fn malformed_identities_labels_and_constraints_are_refused() {
    let mut bad_id = bool_setting("setting.a");
    bad_id.setting_id = "bad id!".to_owned();
    assert_eq!(
        reject(bad_id),
        SettingsReferenceError::InvalidInput("setting_id")
    );

    let mut bad_label = bool_setting("setting.a");
    bad_label.label = String::new();
    assert_eq!(
        reject(bad_label),
        SettingsReferenceError::InvalidInput("label")
    );

    let mut duplicate = setting(
        "setting.a",
        SettingsCategory::Display,
        SettingsLevel::Global,
        SettingsValueType::Integer,
    );
    duplicate.constraints = vec![
        SettingConstraint::Range {
            min: 0,
            max: 1,
            step: None,
        },
        SettingConstraint::Range {
            min: 0,
            max: 1,
            step: None,
        },
    ];
    assert_eq!(
        reject(duplicate),
        SettingsReferenceError::InvalidInput("duplicate_constraint")
    );

    let mut range_on_bool = bool_setting("setting.a");
    range_on_bool.constraints = vec![SettingConstraint::Range {
        min: 0,
        max: 1,
        step: None,
    }];
    assert_eq!(
        reject(range_on_bool),
        SettingsReferenceError::ConstraintValueTypeMismatch("setting.a".to_owned())
    );
}

#[test]
fn integer_bounds_are_enforced() {
    let integer = |min: i64, max: i64, step: Option<i64>| {
        with_range(
            setting(
                "setting.a",
                SettingsCategory::Display,
                SettingsLevel::Global,
                SettingsValueType::Integer,
            ),
            min,
            max,
            step,
        )
    };
    assert_eq!(
        reject(integer(10, 1, None)),
        SettingsReferenceError::InvalidInput("range")
    );
    assert_eq!(
        reject(integer(0, 10, Some(0))),
        SettingsReferenceError::InvalidInput("range")
    );
    let out_of_range = with_values(
        integer(0, 10, None),
        SettingValue::Integer(99),
        SettingValue::Integer(99),
    );
    assert_eq!(
        reject(out_of_range),
        SettingsReferenceError::ValueOutOfRange {
            setting_id: "setting.a".to_owned()
        }
    );
    let accepted = with_values(
        integer(0, 10, Some(2)),
        SettingValue::Integer(4),
        SettingValue::Integer(4),
    );
    let manifest = manifest(&[], &["setting.a"]);
    let snapshot = snapshot(
        &manifest,
        vec![accepted],
        profile("profile:default", SettingsProfileKind::Default),
    );
    assert!(produce(&manifest, &SettingsFixtureSource::new(snapshot)).is_ok());
}

#[test]
fn enumeration_options_are_closed_and_unique() {
    let enumeration = || {
        setting(
            "setting.a",
            SettingsCategory::Language,
            SettingsLevel::Global,
            SettingsValueType::Enumeration,
        )
    };
    let mut empty = enumeration();
    empty.constraints = vec![SettingConstraint::Options(Vec::new())];
    assert_eq!(
        reject(empty),
        SettingsReferenceError::InvalidInput("options")
    );
    assert_eq!(
        reject(with_options(enumeration(), &["opt:a", "opt:a"])),
        SettingsReferenceError::InvalidInput("duplicate_option")
    );
    let unknown = with_values(
        with_options(enumeration(), &["opt:a", "opt:b"]),
        SettingValue::Enumeration("opt:c".to_owned()),
        SettingValue::Enumeration("opt:a".to_owned()),
    );
    assert_eq!(
        reject(unknown),
        SettingsReferenceError::UnknownOption {
            setting_id: "setting.a".to_owned(),
            option_id: "opt:c".to_owned()
        }
    );
}

#[test]
fn value_types_and_read_seams_must_match_the_declaration() {
    let mismatch = with_values(
        setting(
            "setting.a",
            SettingsCategory::Display,
            SettingsLevel::Global,
            SettingsValueType::Integer,
        ),
        SettingValue::Boolean(true),
        SettingValue::Integer(1),
    );
    assert_eq!(
        reject(mismatch),
        SettingsReferenceError::ValueTypeMismatch("setting.a".to_owned())
    );

    let mut untyped = bool_setting("setting.a");
    untyped.value_type = SettingsValueType::Unknown;
    untyped.stored_value = SettingValueField::available(
        SettingValue::Boolean(true),
        SettingsEvidence::SourceDerived,
        SettingsReadSeam::OwnerSettingsApi,
    );
    assert_eq!(
        reject(untyped),
        SettingsReferenceError::ValueTypeMismatch("setting.a".to_owned())
    );

    let mut wrong_seam = setting(
        "setting.a",
        SettingsCategory::Display,
        SettingsLevel::Profile,
        SettingsValueType::Boolean,
    );
    wrong_seam.stored_value = SettingValueField::available(
        SettingValue::Boolean(true),
        SettingsEvidence::SourceDerived,
        SettingsReadSeam::OwnerSettingsApi,
    );
    assert_eq!(
        reject(wrong_seam),
        SettingsReferenceError::InvalidInput("value_seam")
    );
    assert_eq!(
        seam(SettingsLevel::Profile),
        SettingsReadSeam::ProfilePreferenceApi
    );
}

#[test]
fn key_bindings_are_bounded_and_typed() {
    let mut keys = setting(
        "setting.a",
        SettingsCategory::Input,
        SettingsLevel::Global,
        SettingsValueType::KeyBinding,
    );
    let many: Vec<String> = (0..17).map(|index| format!("key:{index}")).collect();
    keys.stored_value = SettingValueField::available(
        SettingValue::Keys(many),
        SettingsEvidence::SourceDerived,
        SettingsReadSeam::OwnerSettingsApi,
    );
    assert_eq!(reject(keys), SettingsReferenceError::InvalidInput("keys"));
}

#[test]
fn private_and_hidden_settings_must_withhold_every_value() {
    let mut private_visible = bool_setting("setting.a");
    private_visible.sensitivity = SettingsSensitivity::Private;
    assert_eq!(
        reject(private_visible),
        SettingsReferenceError::ValueMustBeWithheld("setting.a".to_owned())
    );

    let mut private_value = with_values(
        bool_setting("setting.a"),
        SettingValue::Boolean(true),
        SettingValue::Boolean(true),
    );
    private_value.sensitivity = SettingsSensitivity::Private;
    private_value.visibility = SettingsVisibility::OwnerOnly;
    assert_eq!(
        reject(private_value),
        SettingsReferenceError::ValueMustBeWithheld("setting.a".to_owned())
    );

    let mut hidden_value = with_values(
        bool_setting("setting.a"),
        SettingValue::Boolean(true),
        SettingValue::Boolean(true),
    );
    hidden_value.visibility = SettingsVisibility::Hidden;
    assert_eq!(
        reject(hidden_value),
        SettingsReferenceError::ValueMustBeWithheld("setting.a".to_owned())
    );
}

#[test]
fn the_run_configuration_link_and_its_reference_must_agree() {
    let mut linked_without_reference = bool_setting("setting.a");
    linked_without_reference.run_link = RunConfigurationLink::RunAffecting {
        configuration_id: "run_configuration:standard".to_owned(),
    };
    assert_eq!(
        reject(linked_without_reference),
        SettingsReferenceError::MissingRunReference("setting.a".to_owned())
    );

    let mut reference_without_link = bool_setting("setting.a");
    reference_without_link
        .references
        .push(SettingsSemanticReference {
            kind: SettingsSemanticReferenceKind::RunConfiguration,
            id: "run_configuration:standard".to_owned(),
            label: SettingsText::available("standard").expect("label"),
        });
    assert_eq!(
        reject(reference_without_link),
        SettingsReferenceError::UnexpectedRunReference("setting.a".to_owned())
    );

    let mut withheld_link = bool_setting("setting.a");
    withheld_link.run_link =
        RunConfigurationLink::Unavailable(SettingsUnavailableReason::NotIntegrated);
    let manifest = manifest(&[], &["setting.a"]);
    let snapshot = snapshot(
        &manifest,
        vec![withheld_link],
        profile("profile:default", SettingsProfileKind::Default),
    );
    assert!(produce(&manifest, &SettingsFixtureSource::new(snapshot)).is_ok());
}

#[test]
fn an_oversized_definition_is_refused() {
    let mut text = setting(
        "setting.a",
        SettingsCategory::Accessibility,
        SettingsLevel::Global,
        SettingsValueType::Enumeration,
    );
    let long_label = "l".repeat(300);
    let options = (0..256)
        .map(|index| {
            SettingOption::new(&format!("opt:{index:03}-{}", "i".repeat(240)), &long_label)
                .expect("option")
        })
        .collect();
    text.constraints = vec![SettingConstraint::Options(options)];
    let error = reject(text);
    assert!(
        matches!(error, SettingsReferenceError::DefinitionTooLarge { .. }),
        "unexpected error: {error:?}"
    );
}
