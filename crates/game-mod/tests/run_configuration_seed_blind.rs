// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/run_configuration.rs"]
mod support;

use sts2_game_mod::{
    RunConfigurationCacheKey, RunConfigurationCatalog, RunConfigurationListQuery,
    RunConfigurationPage, RunConfigurationRecordInput, RunConfigurationSummary, RunDifficulty,
    RunFieldRecord, RunMode, RunSeedPolicy, RunVisibilityScope,
};
use support::{
    manifest, produce, profile, public_seed, record, required_fields, snapshot, withheld_seed,
};

fn seeded(
    run_id: &str,
    seed: Option<RunFieldRecord>,
    policy: RunSeedPolicy,
) -> RunConfigurationRecordInput {
    let mut fields = required_fields(
        RunMode::Standard,
        RunDifficulty::Base,
        "character:ironclad",
        &["act:1"],
    );
    if let Some(seed) = seed {
        fields.push(seed);
    }
    record(run_id, 4, policy, fields, vec![])
}

fn catalog_of(records: Vec<RunConfigurationRecordInput>) -> RunConfigurationCatalog {
    let run_ids: Vec<String> = records.iter().map(|r| r.run_id.clone()).collect();
    let run_ids: Vec<&str> = run_ids.iter().map(String::as_str).collect();
    let manifest = manifest(&run_ids);
    let snap = snapshot(&manifest, records, profile("profile:steam"));
    produce(&manifest, snap).expect("produce")
}

fn seed_blind_page(reader: &mut sts2_game_mod::RunConfigurationReader) -> RunConfigurationPage {
    reader
        .list(&RunConfigurationListQuery {
            locale: "en-US".to_owned(),
            revision: None,
            mode: None,
            scope: RunVisibilityScope::SeedBlind,
            limit: 8,
            continuation: None,
        })
        .expect("seed-blind page")
}

fn entry<'a>(page: &'a RunConfigurationPage, run_id: &str) -> &'a RunConfigurationSummary {
    page.entries
        .iter()
        .find(|entry| entry.reference.run_id == run_id)
        .expect("entry")
}

fn visible_runs() -> RunConfigurationCatalog {
    catalog_of(vec![
        seeded("run.a", Some(public_seed("SEED-A")), RunSeedPolicy::Visible),
        seeded("run.b", Some(public_seed("SEED-B")), RunSeedPolicy::Visible),
    ])
}

#[test]
fn seed_blind_cache_does_not_vary_with_seed_material() {
    let mut reader = visible_runs().reader();
    let page = seed_blind_page(&mut reader);
    let a = entry(&page, "run.a");
    let b = entry(&page, "run.b");
    assert_eq!(a.seed_blind_cache, b.seed_blind_cache);
    assert_ne!(a.cache, b.cache);
}

#[test]
fn seed_blind_accessor_is_seed_invariant_where_the_public_accessor_is_not() {
    let reader = visible_runs().reader();
    let a = reader
        .cache_key("run.a", RunVisibilityScope::SeedBlind)
        .expect("seed-blind key a");
    let b = reader
        .cache_key("run.b", RunVisibilityScope::SeedBlind)
        .expect("seed-blind key b");
    assert_eq!(a, b);
    let public_a = reader
        .cache_key("run.a", RunVisibilityScope::Public)
        .expect("public key a");
    let public_b = reader
        .cache_key("run.b", RunVisibilityScope::Public)
        .expect("public key b");
    assert_ne!(public_a, public_b);
}

#[test]
fn a_seed_blind_key_is_never_reusable_for_a_seed_aware_query() {
    let mut reader = visible_runs().reader();
    let page = seed_blind_page(&mut reader);
    for summary in &page.entries {
        let seed_blind: RunConfigurationCacheKey = summary.seed_blind_cache.clone();
        assert!(seed_blind.seed_blind);
        assert!(!summary.cache.seed_blind);
        assert_ne!(seed_blind, summary.cache);
        assert_ne!(seed_blind, entry(&page, "run.a").cache);
        assert_ne!(seed_blind, entry(&page, "run.b").cache);
    }
}

#[test]
fn an_absent_seed_field_leaves_the_seed_blind_key_unchanged() {
    let absent = catalog_of(vec![seeded("run.a", None, RunSeedPolicy::Withheld)]);
    let present = catalog_of(vec![seeded(
        "run.a",
        Some(withheld_seed("SEED-A")),
        RunSeedPolicy::Withheld,
    )]);
    let absent_reader = absent.reader();
    let present_reader = present.reader();
    let absent_key = absent_reader
        .cache_key("run.a", RunVisibilityScope::SeedBlind)
        .expect("absent key");
    let present_key = present_reader
        .cache_key("run.a", RunVisibilityScope::SeedBlind)
        .expect("present key");
    assert_eq!(absent_key, present_key);
}
