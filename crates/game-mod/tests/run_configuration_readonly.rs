// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_configuration.rs"]
mod support;

use sts2_game_mod::{
    FixtureRunConfigurationSource, RunConfigurationCatalog, RunConfigurationCatalogProducer,
    RunConfigurationField, RunConfigurationListQuery, RunDifficulty, RunFieldKind,
    RunModifierState, RunSeedPolicy, RunValue, RunVisibilityScope,
};
use support::{
    field_record, manifest, modifier, profile, public_seed, record, required_fields, snapshot,
};

fn record_with_seed(run_id: &str) -> sts2_game_mod::RunConfigurationRecordInput {
    let mut fields = required_fields(
        sts2_game_mod::RunMode::Standard,
        RunDifficulty::Base,
        "character:ironclad",
        &["act:1"],
    );
    fields.push(field_record(
        RunFieldKind::Modifiers,
        RunConfigurationField::available(RunValue::Identifiers(vec![
            "modifier:ascension".to_owned(),
        ])),
    ));
    fields.push(public_seed("SEED-READONLY"));
    record(
        run_id,
        3,
        RunSeedPolicy::Visible,
        fields,
        vec![modifier(
            "modifier:ascension",
            RunModifierState::Active,
            &[RunFieldKind::Difficulty],
        )],
    )
}

fn fixture() -> (
    sts2_game_mod::ContentManifest,
    FixtureRunConfigurationSource,
) {
    let manifest = manifest(&["run.alpha"]);
    let source = FixtureRunConfigurationSource::new(snapshot(
        &manifest,
        vec![record_with_seed("run.alpha")],
        profile("profile:default"),
    ));
    (manifest, source)
}

fn read_everything(catalog: RunConfigurationCatalog) {
    let reference = catalog
        .definition("run.alpha")
        .expect("definition")
        .reference
        .clone();
    let mut reader = catalog.reader();
    let _ = reader
        .list(&RunConfigurationListQuery {
            locale: "en-US".to_owned(),
            revision: None,
            mode: None,
            scope: RunVisibilityScope::Public,
            limit: 4,
            continuation: None,
        })
        .expect("page");
    let _ = reader
        .get(&reference, RunVisibilityScope::SeedBlind)
        .expect("seed-blind projection");
    let _ = reader
        .get(&reference, RunVisibilityScope::Owner)
        .expect("owner projection");
    let _ = reader
        .settled(
            "run.alpha",
            RunFieldKind::Difficulty,
            RunVisibilityScope::Public,
        )
        .expect("settled");
    let _ = reader
        .cache_key("run.alpha", RunVisibilityScope::SeedBlind)
        .expect("key");
    let _ = reader
        .cache_key("run.alpha", RunVisibilityScope::Public)
        .expect("key");
}

#[test]
fn producing_and_reading_never_mutates_the_source() {
    let (manifest, source) = fixture();
    let original = source.snapshot().clone();
    assert_eq!(source.reads(), 0);

    let catalog = RunConfigurationCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");
    assert_eq!(source.reads(), 1);
    read_everything(catalog);
    assert_eq!(source.reads(), 1);
    assert_eq!(source.snapshot(), &original);
    assert_eq!(original.records.len(), 1);
}

#[test]
fn producing_twice_from_one_source_is_deterministic_and_read_only() {
    let (manifest, source) = fixture();
    let original = source.snapshot().clone();
    let first = RunConfigurationCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("first");
    let second = RunConfigurationCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("second");
    assert_eq!(source.reads(), 2);
    assert_eq!(first, second);
    assert_eq!(source.snapshot(), &original);
    assert_eq!(first.binding(), second.binding());
    assert_eq!(first.family(), second.family());
}

#[test]
fn seed_blind_reads_do_not_alter_retained_definitions() {
    let (manifest, source) = fixture();
    let catalog = RunConfigurationCatalogProducer::new()
        .produce(&manifest, &source)
        .expect("catalog");
    let reference = catalog
        .definition("run.alpha")
        .expect("definition")
        .reference
        .clone();
    let reader = catalog.reader();

    let projected = reader
        .get(&reference, RunVisibilityScope::SeedBlind)
        .expect("seed-blind");
    assert!(projected.field(RunFieldKind::Seed).is_none());
    assert!(
        reader
            .catalog()
            .definition("run.alpha")
            .expect("retained")
            .field(RunFieldKind::Seed)
            .is_some()
    );
    assert_eq!(reader.catalog().len(), 1);
    assert_eq!(source.reads(), 1);
}
