// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_configuration.rs"]
mod support;

use sts2_game_mod::{
    FailingRunConfigurationSource, FixtureRunConfigurationFailure, RunConfigurationCatalogProducer,
    RunConfigurationError, RunConfigurationFamilyState, RunConfigurationField,
    RunConfigurationListQuery, RunConfigurationUnavailableReason, RunDifficulty, RunFieldKind,
    RunModifierInput, RunModifierState, RunProvenance, RunSeedPolicy, RunValue, RunVisibilityScope,
};
use support::{
    empty_family_manifest, field_record, manifest, manifest_of, modified_field, modifier, produce,
    profile, public_seed, record, required_fields, snapshot,
};

fn catalog_of(
    manifest: &sts2_game_mod::ContentManifest,
    records: Vec<sts2_game_mod::RunConfigurationRecordInput>,
) -> Result<sts2_game_mod::RunConfigurationCatalog, RunConfigurationError> {
    produce(
        manifest,
        snapshot(manifest, records, profile("profile:default")),
    )
}

fn standard_record(run_id: &str) -> sts2_game_mod::RunConfigurationRecordInput {
    record(
        run_id,
        1,
        RunSeedPolicy::Withheld,
        required_fields(
            sts2_game_mod::RunMode::Standard,
            RunDifficulty::Base,
            "character:ironclad",
            &["act:1"],
        ),
        Vec::new(),
    )
}

#[test]
fn missing_capability_maps_to_a_sanitized_error() {
    let manifest = manifest(&["run.alpha"]);
    for (failure, expected) in [
        (
            FixtureRunConfigurationFailure::NoActiveSource,
            RunConfigurationError::NoActiveSource,
        ),
        (
            FixtureRunConfigurationFailure::AccessDenied,
            RunConfigurationError::SourceAccessDenied,
        ),
        (
            FixtureRunConfigurationFailure::Malformed,
            RunConfigurationError::MalformedSource,
        ),
    ] {
        let source = FailingRunConfigurationSource(failure);
        assert_eq!(
            RunConfigurationCatalogProducer::new().produce(&manifest, &source),
            Err(expected)
        );
    }
}

#[test]
fn family_support_states_stay_explicit() {
    let manifest = empty_family_manifest();
    for (state, expected) in [
        (
            RunConfigurationFamilyState::Unsupported,
            RunConfigurationError::UnsupportedFamily,
        ),
        (
            RunConfigurationFamilyState::Unavailable,
            RunConfigurationError::UnavailableFamily,
        ),
    ] {
        let mut source = snapshot(&manifest, Vec::new(), profile("profile:default"));
        source.family.state = state;
        let catalog = produce(&manifest, source).expect("catalog");
        let mut reader = catalog.reader();
        assert_eq!(
            reader.list(&RunConfigurationListQuery {
                locale: "en-US".to_owned(),
                revision: None,
                mode: None,
                scope: RunVisibilityScope::Public,
                limit: 4,
                continuation: None,
            }),
            Err(expected)
        );
    }

    let mut unsupported = snapshot(
        &manifest,
        vec![standard_record("run.alpha")],
        profile("profile:default"),
    );
    unsupported.family.state = RunConfigurationFamilyState::Unsupported;
    unsupported.family.definition_count = 0;
    assert_eq!(
        produce(&manifest, unsupported),
        Err(RunConfigurationError::UnknownDefinition(
            "run.alpha".to_owned()
        ))
    );
}

#[test]
fn manifest_fences_reject_every_mismatched_snapshot() {
    let manifest = manifest(&["run.alpha"]);
    let good = || {
        snapshot(
            &manifest,
            vec![standard_record("run.alpha")],
            profile("profile:default"),
        )
    };

    let mut wrong_manifest = good();
    wrong_manifest.manifest = manifest_of(&[("other_kind", "other.1")]).cursor_binding();
    assert_eq!(
        produce(&manifest, wrong_manifest),
        Err(RunConfigurationError::ManifestMismatch)
    );

    let mut wrong_locale = good();
    wrong_locale.locale = "de-DE".to_owned();
    assert_eq!(
        produce(&manifest, wrong_locale),
        Err(RunConfigurationError::LocaleMismatch)
    );

    let mut wrong_producer = good();
    wrong_producer.producer_version = "run-configuration-reference-producer-v0".to_owned();
    assert_eq!(
        produce(&manifest, wrong_producer),
        Err(RunConfigurationError::ProducerVersionMismatch)
    );

    let mut wrong_family = good();
    wrong_family.family.entity_kind = "other_kind".to_owned();
    assert_eq!(
        produce(&manifest, wrong_family),
        Err(RunConfigurationError::FamilyIdentityMismatch)
    );

    let mut wrong_count = good();
    wrong_count.family.definition_count = 5;
    assert_eq!(
        produce(&manifest, wrong_count),
        Err(RunConfigurationError::FamilyCountMismatch)
    );

    let other_kind_manifest = manifest_of(&[("other_kind", "other.1")]);
    assert_eq!(
        produce(
            &other_kind_manifest,
            snapshot(&other_kind_manifest, Vec::new(), profile("profile:default"))
        ),
        Err(RunConfigurationError::MissingFamily)
    );

    let mut unknown_record = good();
    unknown_record.records = vec![standard_record("run.beta")];
    assert_eq!(
        produce(&manifest, unknown_record),
        Err(RunConfigurationError::UnknownDefinition(
            "run.beta".to_owned()
        ))
    );
}

#[test]
fn records_reject_unsafe_or_ambiguous_configuration() {
    let manifest = manifest(&["run.alpha"]);

    let mut duplicate = standard_record("run.alpha");
    duplicate.fields.push(field_record(
        RunFieldKind::Mode,
        RunConfigurationField::available(RunValue::Mode(sts2_game_mod::RunMode::Standard)),
    ));
    assert_eq!(
        catalog_of(&manifest, vec![duplicate]),
        Err(RunConfigurationError::DuplicateField {
            run_id: "run.alpha".to_owned(),
            kind: RunFieldKind::Mode,
        })
    );

    let mut missing = standard_record("run.alpha");
    missing
        .fields
        .retain(|field| field.kind != RunFieldKind::ActiveContent);
    assert_eq!(
        catalog_of(&manifest, vec![missing]),
        Err(RunConfigurationError::MissingRequiredField {
            run_id: "run.alpha".to_owned(),
            kind: RunFieldKind::ActiveContent,
        })
    );

    let mut not_applicable = standard_record("run.alpha");
    for field in &mut not_applicable.fields {
        if field.kind == RunFieldKind::Mode {
            field.settled = RunConfigurationField::unavailable(
                RunConfigurationUnavailableReason::NotApplicable,
            );
        }
    }
    assert_eq!(
        catalog_of(&manifest, vec![not_applicable]),
        Err(RunConfigurationError::NotApplicableRequiredField {
            run_id: "run.alpha".to_owned(),
            kind: RunFieldKind::Mode,
        })
    );

    let mut shaped = standard_record("run.alpha");
    for field in &mut shaped.fields {
        if field.kind == RunFieldKind::Character {
            field.settled =
                RunConfigurationField::available(RunValue::Mode(sts2_game_mod::RunMode::Standard));
            field.requested = field.settled.clone();
        }
    }
    assert_eq!(
        catalog_of(&manifest, vec![shaped]),
        Err(RunConfigurationError::FieldShapeMismatch {
            run_id: "run.alpha".to_owned(),
            kind: RunFieldKind::Character,
        })
    );

    let mut echoed = standard_record("run.alpha");
    echoed.modifiers = vec![modifier(
        "modifier:ascension",
        RunModifierState::Active,
        &[RunFieldKind::Difficulty],
    )];
    for field in &mut echoed.fields {
        if field.kind == RunFieldKind::Difficulty {
            *field = modified_field(
                field.clone(),
                RunProvenance::RequestedEcho,
                sts2_game_mod::RunMutability::FixedAtStart,
            );
        }
    }
    assert_eq!(
        catalog_of(&manifest, vec![echoed]),
        Err(RunConfigurationError::ModifiedFieldEchoesRequest {
            run_id: "run.alpha".to_owned(),
            kind: RunFieldKind::Difficulty,
        })
    );

    let mut mismatched = standard_record("run.alpha");
    mismatched.fields.push(field_record(
        RunFieldKind::Modifiers,
        RunConfigurationField::available(RunValue::Identifiers(vec!["modifier:absent".to_owned()])),
    ));
    assert_eq!(
        catalog_of(&manifest, vec![mismatched]),
        Err(RunConfigurationError::ModifierSetMismatch(
            "run.alpha".to_owned()
        ))
    );

    let mut visible_without_seed = standard_record("run.alpha");
    visible_without_seed.seed_policy = RunSeedPolicy::Visible;
    assert_eq!(
        catalog_of(&manifest, vec![visible_without_seed]),
        Err(RunConfigurationError::SeedPolicyMismatch(
            "run.alpha".to_owned()
        ))
    );

    let mut wrong_seed = standard_record("run.alpha");
    wrong_seed.seed_policy = RunSeedPolicy::Visible;
    wrong_seed.fields.push(public_seed("SEED-1"));
    wrong_seed.fields.push(public_seed("SEED-1"));
    assert_eq!(
        catalog_of(&manifest, vec![wrong_seed]),
        Err(RunConfigurationError::DuplicateField {
            run_id: "run.alpha".to_owned(),
            kind: RunFieldKind::Seed,
        })
    );

    let rng_manifest = support::manifest(&["rng_state:42"]);
    assert_eq!(
        catalog_of(&rng_manifest, vec![standard_record("rng_state:42")]),
        Err(RunConfigurationError::RngStateNotPermitted(
            "rng_state:42".to_owned()
        ))
    );
}

#[test]
fn oversized_payloads_are_rejected_inside_the_record_bound() {
    let manifest = manifest(&["run.alpha"]);
    let label = "x".repeat(512);
    let modifiers = (0..32)
        .map(|index| RunModifierInput {
            modifier_id: format!("modifier:m{index:02}"),
            label: label.clone(),
            state: RunModifierState::Unknown,
            alters: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut heavy = standard_record("run.alpha");
    heavy.modifiers = modifiers;
    let error = catalog_of(&manifest, vec![heavy]).expect_err("oversized rejection");
    assert!(
        matches!(error, RunConfigurationError::DefinitionTooLarge { limit: 16_384, actual } if actual > 16_384),
        "expected an oversized rejection at the 16384-byte limit, got {error:?}"
    );

    let mut over_long = standard_record("run.alpha");
    over_long.run_id = format!("run.{}", "x".repeat(200));
    assert_eq!(
        catalog_of(&manifest, vec![over_long]),
        Err(RunConfigurationError::InvalidInput("run_id"))
    );
}
