// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_configuration.rs"]
mod support;

use sts2_game_mod::{
    ContentManifest, RunConfigurationCatalog, RunConfigurationDefinition, RunConfigurationError,
    RunConfigurationField, RunConfigurationFieldStatus, RunConfigurationListQuery,
    RunConfigurationUnavailableReason, RunDifficulty, RunFieldKind, RunModifierState,
    RunMutability, RunProvenance, RunSeedPolicy, RunValue, RunVisibilityScope,
};
use support::{
    field_record, ids, manifest, modified_field, modifier, produce, profile, public_seed, record,
    required_fields, snapshot, withheld_seed,
};

fn alpha() -> sts2_game_mod::RunConfigurationRecordInput {
    let mut fields = required_fields(
        sts2_game_mod::RunMode::Standard,
        RunDifficulty::Ascension(5),
        "character:ironclad",
        &["act:1", "act:2"],
    );
    fields[1] = modified_field(
        fields[1].clone(),
        RunProvenance::SettledHost,
        RunMutability::Mutable,
    );
    fields.push(field_record(
        RunFieldKind::Modifiers,
        RunConfigurationField::available(RunValue::Identifiers(ids(&["modifier:ascension"]))),
    ));
    fields.push(field_record(
        RunFieldKind::MultiplayerScaling,
        RunConfigurationField::unavailable(RunConfigurationUnavailableReason::NotApplicable),
    ));
    fields.push(public_seed("SEED-SHARED"));
    record(
        "run.alpha",
        4,
        RunSeedPolicy::Visible,
        fields,
        vec![modifier(
            "modifier:ascension",
            RunModifierState::Active,
            &[RunFieldKind::Difficulty],
        )],
    )
}

fn beta() -> sts2_game_mod::RunConfigurationRecordInput {
    let mut fields = required_fields(
        sts2_game_mod::RunMode::Custom,
        RunDifficulty::Base,
        "character:silent",
        &["act:1"],
    );
    fields[4] = field_record(
        RunFieldKind::ActiveContent,
        RunConfigurationField::available(RunValue::Identifiers(ids(&[
            "content:base",
            "content:expansion",
        ]))),
    );
    fields.push(field_record(
        RunFieldKind::Loadout,
        RunConfigurationField::available(RunValue::Identifier("loadout:default".to_owned())),
    ));
    fields.push(field_record(
        RunFieldKind::UnlockRule,
        RunConfigurationField::available(RunValue::Toggle(true)),
    ));
    fields.push(public_seed("SEED-SHARED"));
    record(
        "run.beta",
        4,
        RunSeedPolicy::Visible,
        fields,
        vec![modifier(
            "modifier:future_unknown",
            RunModifierState::Unknown,
            &[],
        )],
    )
}

fn gamma() -> sts2_game_mod::RunConfigurationRecordInput {
    let mut fields = required_fields(
        sts2_game_mod::RunMode::Cooperative,
        RunDifficulty::Base,
        "character:defect",
        &["act:1"],
    );
    fields.push(field_record(
        RunFieldKind::MultiplayerScaling,
        RunConfigurationField::available(RunValue::Count(4)),
    ));
    fields.push(withheld_seed("SEED-GAMMA"));
    record("run.gamma", 9, RunSeedPolicy::Withheld, fields, Vec::new())
}

fn fixture() -> (ContentManifest, RunConfigurationCatalog) {
    let manifest = manifest(&["run.alpha", "run.beta", "run.gamma"]);
    let catalog = produce(
        &manifest,
        snapshot(
            &manifest,
            vec![alpha(), beta(), gamma()],
            profile("profile:default"),
        ),
    )
    .expect("catalog");
    (manifest, catalog)
}

fn definition<'a>(
    catalog: &'a RunConfigurationCatalog,
    run_id: &str,
) -> &'a RunConfigurationDefinition {
    catalog.definition(run_id).expect("definition")
}

#[test]
fn every_admitted_configuration_reports_its_exact_settled_values() {
    let (_manifest, catalog) = fixture();
    assert_eq!(catalog.len(), 3);

    let alpha = definition(&catalog, "run.alpha");
    assert_eq!(alpha.live.run_id, "run.alpha");
    assert_eq!(alpha.live.instance_id, "instance:run.alpha");
    assert_eq!(alpha.live.revision, 4);
    assert_eq!(alpha.reference.revision, 4);
    let difficulty = alpha.field(RunFieldKind::Difficulty).expect("difficulty");
    assert_eq!(
        difficulty.settled.value(),
        Some(&RunValue::Difficulty(RunDifficulty::Ascension(5)))
    );
    assert_eq!(difficulty.requested.value(), difficulty.settled.value());
    assert_eq!(difficulty.provenance, RunProvenance::SettledHost);
    assert_eq!(difficulty.mutability, RunMutability::Mutable);
    assert_eq!(
        alpha
            .field(RunFieldKind::MultiplayerScaling)
            .expect("scaling")
            .settled
            .reason(),
        Some(RunConfigurationUnavailableReason::NotApplicable)
    );

    let beta = definition(&catalog, "run.beta");
    assert_eq!(
        beta.field(RunFieldKind::Mode)
            .expect("mode")
            .settled
            .value(),
        Some(&RunValue::Mode(sts2_game_mod::RunMode::Custom))
    );
    assert_eq!(
        beta.field(RunFieldKind::Loadout)
            .expect("loadout")
            .settled
            .value(),
        Some(&RunValue::Identifier("loadout:default".to_owned()))
    );
    assert_eq!(
        beta.field(RunFieldKind::UnlockRule)
            .expect("unlock")
            .settled
            .value(),
        Some(&RunValue::Toggle(true))
    );
    assert_eq!(beta.modifiers.len(), 1);
    assert_eq!(beta.modifiers[0].state, RunModifierState::Unknown);

    let gamma = definition(&catalog, "run.gamma");
    assert_eq!(
        gamma
            .field(RunFieldKind::MultiplayerScaling)
            .expect("scaling")
            .settled
            .value(),
        Some(&RunValue::Count(4))
    );
    let seed = gamma.field(RunFieldKind::Seed).expect("seed");
    assert_eq!(seed.settled.value(), None);
    assert_eq!(seed.settled.status(), RunConfigurationFieldStatus::Withheld);
    assert!(seed.settled.reason().is_some());
    for run_id in ["run.alpha", "run.beta", "run.gamma"] {
        assert!(definition(&catalog, run_id).completeness.is_complete());
    }
}

#[test]
fn same_seed_runs_with_other_settings_cannot_share_cache_entries() {
    let (_manifest, catalog) = fixture();
    let alpha = definition(&catalog, "run.alpha");
    let beta = definition(&catalog, "run.beta");
    let gamma = definition(&catalog, "run.gamma");

    assert_eq!(alpha.cache.revision, beta.cache.revision);
    assert!(!alpha.cache.is_compatible_with(&beta.cache));
    assert!(
        !alpha
            .seed_blind_cache
            .is_compatible_with(&beta.seed_blind_cache)
    );
    assert!(!alpha.cache.is_compatible_with(&gamma.cache));
    assert!(alpha.cache.is_compatible_with(&alpha.cache));
    assert!(!alpha.cache.seed_blind);
    assert!(alpha.seed_blind_cache.seed_blind);
    assert!(!alpha.cache.is_compatible_with(&alpha.seed_blind_cache));
    assert_eq!(alpha.cache.fingerprint, alpha.cache.fingerprint.clone());

    let mut reader = catalog.reader();
    let page = reader
        .list(&RunConfigurationListQuery {
            locale: "en-US".to_owned(),
            revision: None,
            mode: None,
            scope: RunVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.total, 3);
    assert!(page.complete);
    for (index, entry) in page.entries.iter().enumerate() {
        for other in page.entries.iter().skip(index + 1) {
            assert!(!entry.cache.is_compatible_with(&other.cache));
        }
    }
}

#[test]
fn seed_blind_scope_and_profile_isolation_survive_expansion() {
    let (manifest, catalog) = fixture();
    let alpha_reference = definition(&catalog, "run.alpha").reference.clone();
    let alpha_cache = definition(&catalog, "run.alpha").cache.clone();
    let alpha_seed_blind_cache = definition(&catalog, "run.alpha").seed_blind_cache.clone();
    let binding = catalog.binding().clone();
    let reader = catalog.reader();

    assert_eq!(
        reader.settled(
            "run.alpha",
            RunFieldKind::Seed,
            RunVisibilityScope::SeedBlind
        ),
        Err(RunConfigurationError::SeedBlindScopeViolation)
    );
    let public_seed = reader
        .settled("run.alpha", RunFieldKind::Seed, RunVisibilityScope::Public)
        .expect("seed");
    assert_eq!(
        public_seed.settled.value(),
        Some(&RunValue::Seed("SEED-SHARED".to_owned()))
    );
    let projected = reader
        .get(&alpha_reference, RunVisibilityScope::SeedBlind)
        .expect("projection");
    assert!(projected.field(RunFieldKind::Seed).is_none());
    assert!(
        reader
            .get(&alpha_reference, RunVisibilityScope::Public)
            .expect("public")
            .field(RunFieldKind::Seed)
            .is_some()
    );
    assert_eq!(
        reader
            .cache_key("run.alpha", RunVisibilityScope::SeedBlind)
            .expect("blind key"),
        alpha_seed_blind_cache
    );
    assert_eq!(
        reader
            .cache_key("run.alpha", RunVisibilityScope::Public)
            .expect("key"),
        alpha_cache
    );
    assert_eq!(
        reader.cache_key("run.missing", RunVisibilityScope::Public),
        Err(RunConfigurationError::UnknownRun("run.missing".to_owned()))
    );

    let other = produce(
        &manifest,
        snapshot(
            &manifest,
            vec![alpha(), beta(), gamma()],
            profile("profile:other"),
        ),
    )
    .expect("other catalog");
    assert_ne!(other.binding(), &binding);
    let other_reader = other.reader();
    assert_eq!(
        other_reader.get(&alpha_reference, RunVisibilityScope::Public),
        Err(RunConfigurationError::StaleReference)
    );
    assert_eq!(
        other_reader
            .settled("run.alpha", RunFieldKind::Seed, RunVisibilityScope::Public)
            .expect("other seed")
            .settled
            .value(),
        Some(&RunValue::Seed("SEED-SHARED".to_owned()))
    );
}

#[test]
fn pages_are_bounded_and_continuations_are_single_use() {
    let (_manifest, catalog) = fixture();
    let mut reader = catalog.clone().reader();
    let query = |limit: usize, continuation| RunConfigurationListQuery {
        locale: "en-US".to_owned(),
        revision: None,
        mode: None,
        scope: RunVisibilityScope::Public,
        limit,
        continuation,
    };
    let first = reader.list(&query(1, None)).expect("first");
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let continuation = first.continuation.clone().expect("continuation");
    let second = reader
        .list(&query(1, Some(continuation.clone())))
        .expect("second");
    assert_eq!(second.entries.len(), 1);
    assert_eq!(second.entries[0].live.run_id, "run.beta");
    assert_eq!(
        reader.list(&query(1, Some(continuation))),
        Err(RunConfigurationError::InvalidContinuation)
    );
    assert_eq!(
        reader.list(&query(0, None)),
        Err(RunConfigurationError::InvalidPageSize)
    );
    assert_eq!(
        reader.list(&RunConfigurationListQuery {
            locale: "de-DE".to_owned(),
            ..query(1, None)
        }),
        Err(RunConfigurationError::LocaleMismatch)
    );

    let mut other_reader = catalog.reader();
    let other_page = other_reader.list(&query(1, None)).expect("other page");
    let foreign = other_page.continuation.expect("foreign continuation");
    assert_eq!(
        reader.list(&query(1, Some(foreign))),
        Err(RunConfigurationError::InvalidContinuation)
    );

    let filtered = reader.list(&query(8, None)).expect("filtered");
    assert_eq!(filtered.total, 3);
    let stale = reader
        .list(&RunConfigurationListQuery {
            revision: Some(9),
            ..query(8, None)
        })
        .expect("revision filter");
    assert_eq!(stale.total, 1);
    assert_eq!(stale.entries[0].live.run_id, "run.gamma");
    let custom = reader
        .list(&RunConfigurationListQuery {
            mode: Some(sts2_game_mod::RunMode::Custom),
            ..query(8, None)
        })
        .expect("mode filter");
    assert_eq!(custom.total, 1);
    assert_eq!(custom.entries[0].live.run_id, "run.beta");
}

#[test]
fn every_required_kind_is_settled_for_each_admitted_mode() {
    let (_manifest, catalog) = fixture();
    for run_id in ["run.alpha", "run.beta", "run.gamma"] {
        let definition = definition(&catalog, run_id);
        for kind in [
            RunFieldKind::Mode,
            RunFieldKind::Difficulty,
            RunFieldKind::Character,
            RunFieldKind::ActSequence,
            RunFieldKind::ActiveContent,
        ] {
            assert!(
                definition
                    .field(kind)
                    .expect("required kind")
                    .settled
                    .is_available(),
                "{run_id} {kind:?}"
            );
        }
    }
}
