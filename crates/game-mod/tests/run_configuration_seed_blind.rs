// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_configuration.rs"]
mod support;

use sts2_game_mod::{
    RunConfigurationCatalog, RunConfigurationDefinition, RunConfigurationListQuery, RunDifficulty,
    RunFieldRecord, RunMode, RunSeedPolicy, RunVisibilityScope,
};
use support::{manifest, produce, profile, public_seed, record, required_fields, snapshot};

fn definition<'a>(
    catalog: &'a RunConfigurationCatalog,
    run_id: &str,
) -> &'a RunConfigurationDefinition {
    catalog.definition(run_id).expect("definition")
}

fn seeded(seed: &str) -> Vec<RunFieldRecord> {
    let mut fields = required_fields(
        RunMode::Standard,
        RunDifficulty::Base,
        "character:ironclad",
        &["act:1"],
    );
    fields.push(public_seed(seed));
    fields
}

fn catalog_of_seeded_runs() -> RunConfigurationCatalog {
    let manifest = manifest(&["run.seeded-one", "run.seeded-two"]);
    produce(
        &manifest,
        snapshot(
            &manifest,
            vec![
                record(
                    "run.seeded-one",
                    4,
                    RunSeedPolicy::Visible,
                    seeded("SEED-ONE"),
                    Vec::new(),
                ),
                record(
                    "run.seeded-two",
                    4,
                    RunSeedPolicy::Visible,
                    seeded("SEED-TWO"),
                    Vec::new(),
                ),
            ],
            profile("profile:default"),
        ),
    )
    .expect("catalog")
}

#[test]
fn seed_blind_keys_and_projections_never_carry_seed_material() {
    let catalog = catalog_of_seeded_runs();
    let one_cache = definition(&catalog, "run.seeded-one").cache.clone();
    let one_seed_blind = definition(&catalog, "run.seeded-one")
        .seed_blind_cache
        .clone();
    let two_cache = definition(&catalog, "run.seeded-two").cache.clone();
    let two_seed_blind = definition(&catalog, "run.seeded-two")
        .seed_blind_cache
        .clone();
    assert_ne!(
        one_cache, two_cache,
        "the seed-aware key must still separate two runs with different visible seeds"
    );
    assert_eq!(
        one_seed_blind, two_seed_blind,
        "the seed-blind key must not incorporate seed material"
    );

    let reference = definition(&catalog, "run.seeded-one").reference.clone();
    let mut reader = catalog.reader();
    let page = reader
        .list(&RunConfigurationListQuery {
            locale: "en-US".to_owned(),
            revision: None,
            mode: None,
            scope: RunVisibilityScope::SeedBlind,
            limit: 8,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.entries.len(), 2);
    assert_eq!(
        page.entries[0].cache, page.entries[1].cache,
        "a seed-blind page must not expose a seed-derived cache key"
    );
    assert_eq!(
        page.entries[0].seed_blind_cache,
        page.entries[1].seed_blind_cache
    );

    let projected = reader
        .get(&reference, RunVisibilityScope::SeedBlind)
        .expect("seed-blind projection");
    assert_eq!(
        projected.cache, projected.seed_blind_cache,
        "a seed-blind projection must not keep the seed-aware key"
    );
}
