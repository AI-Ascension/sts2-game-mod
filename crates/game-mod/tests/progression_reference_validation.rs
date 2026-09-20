// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/progression_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentManifest, ProgressionDomain, ProgressionEntryInput, ProgressionFieldValue,
    ProgressionProfileKind, ProgressionReadState, ProgressionReferenceError,
    ProgressionSensitivity, ProgressionText,
};

/// Produces one snapshot around a single entry and returns the refusal it must earn.
fn refuse(manifest: &ContentManifest, entry: ProgressionEntryInput) -> ProgressionReferenceError {
    let snapshot = snapshot(
        manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        &PROJECTED_DOMAINS,
        vec![entry],
    );
    produce(manifest, &FixturePort::new(snapshot)).expect_err("refused")
}

/// The default manifest with every family this entry could name left handled.
fn unlocked(id: &str) -> ProgressionEntryInput {
    unlocked_entry(id, ProgressionDomain::Unlocks)
}

#[test]
fn malformed_identities_and_labels_are_refused() {
    let manifest = manifest(&[]);

    let mut bad_id = unlocked("unlock:act_four");
    bad_id.entry_id = "unlock act four".to_owned();
    assert_eq!(
        refuse(&manifest, bad_id),
        ProgressionReferenceError::InvalidInput("entry_id")
    );

    let mut bad_title = unlocked("unlock:act_four");
    bad_title.title = String::new();
    assert_eq!(
        refuse(&manifest, bad_title),
        ProgressionReferenceError::InvalidInput("title")
    );

    let mut bad_description = unlocked("unlock:act_four");
    bad_description.description = Some(String::new());
    assert_eq!(
        refuse(&manifest, bad_description),
        ProgressionReferenceError::InvalidInput("description")
    );

    let mut bad_requirement = locked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    let mut stated = requirement("requirement one", ProgressionReadState::Locked);
    stated.requirement_id = "requirement one".to_owned();
    bad_requirement.requirements = ProgressionFieldValue::available(vec![stated]);
    bad_requirement = finish(bad_requirement);
    assert_eq!(
        refuse(&manifest, bad_requirement),
        ProgressionReferenceError::InvalidInput("requirement_id")
    );
}

#[test]
fn an_account_identifier_beside_game_progression_is_refused() {
    let manifest = manifest(&[]);
    let mut entry = unlocked("unlock:act_four");
    entry.account_id = Some("account:12345".to_owned());
    assert_eq!(
        refuse(&manifest, entry),
        ProgressionReferenceError::AccountIdentifierInProgression
    );
}

#[test]
fn account_scoped_data_cannot_be_published_as_progression() {
    let manifest = manifest(&[]);

    let mut by_sensitivity = unlocked("unlock:act_four");
    by_sensitivity.sensitivity = ProgressionSensitivity::AccountScoped;
    assert_eq!(
        refuse(&manifest, by_sensitivity),
        ProgressionReferenceError::AccountScopedEntryInProgression
    );

    let mut by_domain = unlocked("unlock:act_four");
    by_domain.domain = ProgressionDomain::AccountScoped;
    assert_eq!(
        refuse(&manifest, by_domain),
        ProgressionReferenceError::AccountScopedEntryInProgression
    );
}

#[test]
fn a_domain_whose_manifest_family_is_unhandled_is_refused() {
    let manifest = manifest_handling(&["achievement", "compendium_entry", "statistic"]);
    assert_eq!(
        refuse(&manifest, unlocked("unlock:act_four")),
        ProgressionReferenceError::UnhandledManifestFamily("unlock".to_owned())
    );
}

#[test]
fn a_locked_entry_must_state_what_gates_it() {
    let manifest = manifest(&[]);
    let bare = entry(
        "unlock:act_four",
        ProgressionDomain::Unlocks,
        ProgressionReadState::Locked,
    );
    assert_eq!(
        refuse(&manifest, bare),
        ProgressionReferenceError::LockedWithoutRequirement
    );
}

#[test]
fn an_unlocked_entry_may_not_carry_a_locked_requirement() {
    let manifest = manifest(&[]);
    let mut entry = unlocked("unlock:act_four");
    entry.requirements = ProgressionFieldValue::available(vec![requirement(
        "requirement:one",
        ProgressionReadState::Locked,
    )]);
    entry = finish(entry);
    assert_eq!(
        refuse(&manifest, entry),
        ProgressionReferenceError::UnlockedWithUnsatisfiedRequirement
    );
}

#[test]
fn an_undiscovered_entry_is_only_stated_inside_the_compendium() {
    let manifest = manifest(&[]);
    let undiscovered = entry(
        "achievement:hidden",
        ProgressionDomain::Achievements,
        ProgressionReadState::Undiscovered,
    );
    assert_eq!(
        refuse(&manifest, undiscovered),
        ProgressionReferenceError::UndiscoveredOutsideCompendium
    );
}

#[test]
fn a_state_that_asserts_nothing_carries_no_value() {
    let manifest = manifest(&[]);

    let mut progress = entry(
        "statistic:runs_started",
        ProgressionDomain::AggregateStatistics,
        ProgressionReadState::NotTracked,
    );
    progress.progress = ProgressionFieldValue::available(count(3));
    progress = finish(progress);
    assert_eq!(
        refuse(&manifest, progress),
        ProgressionReferenceError::StateCarriesValue("progress")
    );

    let mut best = entry(
        "statistic:runs_started",
        ProgressionDomain::AggregateStatistics,
        ProgressionReadState::Unavailable,
    );
    best.best = ProgressionFieldValue::available(seconds(1));
    best = finish(best);
    assert_eq!(
        refuse(&manifest, best),
        ProgressionReferenceError::StateCarriesValue("best_value")
    );

    let mut unclassified = entry(
        "unlock:act_four",
        ProgressionDomain::Unlocks,
        ProgressionReadState::Unclassified,
    );
    unclassified.progress = ProgressionFieldValue::available(percent(0));
    unclassified = finish(unclassified);
    assert_eq!(
        refuse(&manifest, unclassified),
        ProgressionReferenceError::StateCarriesValue("progress")
    );
}

#[test]
fn a_best_value_is_only_stated_on_a_best_record() {
    let manifest = manifest(&[]);
    let mut entry = unlocked("unlock:act_four");
    entry.best = ProgressionFieldValue::available(seconds(30));
    entry = finish(entry);
    assert_eq!(
        refuse(&manifest, entry),
        ProgressionReferenceError::BestRecordOutsideBestRecords
    );
}

#[test]
fn a_percentage_outside_its_range_is_never_clamped() {
    let manifest = manifest(&[]);

    let mut too_high = unlocked("unlock:act_four");
    too_high.progress = ProgressionFieldValue::available(percent(101));
    too_high = finish(too_high);
    assert_eq!(
        refuse(&manifest, too_high),
        ProgressionReferenceError::UnboundedPercentage
    );

    let mut negative = entry(
        "best_record:fastest_floor",
        ProgressionDomain::BestRecords,
        ProgressionReadState::Unlocked,
    );
    negative.best = ProgressionFieldValue::available(percent(-5));
    negative = finish(negative);
    assert_eq!(
        refuse(&manifest, negative),
        ProgressionReferenceError::UnboundedPercentage
    );

    let mut on_requirement = locked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    let mut stated = requirement("requirement:one", ProgressionReadState::Locked);
    stated.progress = ProgressionFieldValue::available(percent(150));
    on_requirement.requirements = ProgressionFieldValue::available(vec![stated]);
    on_requirement = finish(on_requirement);
    assert_eq!(
        refuse(&manifest, on_requirement),
        ProgressionReferenceError::UnboundedPercentage
    );
}

#[test]
fn a_collection_stated_available_may_not_be_empty() {
    let manifest = manifest(&[]);

    let mut requirements = unlocked("unlock:act_four");
    requirements.requirements = ProgressionFieldValue::available(Vec::new());
    requirements = finish(requirements);
    assert_eq!(
        refuse(&manifest, requirements),
        ProgressionReferenceError::EmptyPresentCollection("requirements")
    );

    let mut references = unlocked("unlock:act_four");
    references.content_references = ProgressionFieldValue::available(Vec::new());
    references = finish(references);
    assert_eq!(
        refuse(&manifest, references),
        ProgressionReferenceError::EmptyPresentCollection("content_references")
    );

    let mut related_entries = unlocked("unlock:act_four");
    related_entries.related = ProgressionFieldValue::available(Vec::new());
    related_entries = finish(related_entries);
    assert_eq!(
        refuse(&manifest, related_entries),
        ProgressionReferenceError::EmptyPresentCollection("related")
    );
}

#[test]
fn a_repeated_requirement_identity_is_refused() {
    let manifest = manifest(&[]);
    let mut entry = locked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    entry.requirements = ProgressionFieldValue::available(vec![
        requirement("requirement:one", ProgressionReadState::Locked),
        requirement("requirement:one", ProgressionReadState::Unlocked),
    ]);
    entry = finish(entry);
    assert_eq!(
        refuse(&manifest, entry),
        ProgressionReferenceError::DuplicateRequirement("requirement:one".to_owned())
    );
}

#[test]
fn a_reference_that_does_not_resolve_is_refused() {
    let manifest = manifest(&[]);

    let mut absent = unlocked("unlock:act_four");
    absent.content_references = ProgressionFieldValue::available(vec![
        sts2_game_mod::ProgressionContentReference::new("unlock", "base:unlock:absent")
            .expect("reference"),
    ]);
    absent = finish(absent);
    assert_eq!(
        refuse(&manifest, absent),
        ProgressionReferenceError::UnknownManifestReference {
            entity_kind: "unlock".to_owned(),
            namespaced_id: "base:unlock:absent".to_owned(),
        }
    );

    let thin = manifest_handling(&["achievement", "compendium_entry", "statistic"]);
    let mut unhandled = entry(
        "achievement:first_blood",
        ProgressionDomain::Achievements,
        ProgressionReadState::Unlocked,
    );
    unhandled.content_references = ProgressionFieldValue::available(vec![content("unlock")]);
    unhandled = finish(unhandled);
    assert_eq!(
        refuse(&thin, unhandled),
        ProgressionReferenceError::UnhandledManifestFamily("unlock".to_owned())
    );
}

#[test]
fn two_entries_may_not_share_one_progression_identity() {
    let manifest = manifest(&[]);
    let snapshot = snapshot(
        &manifest,
        unnamed_profile(ProgressionProfileKind::Active, 7),
        &PROJECTED_DOMAINS,
        vec![unlocked("unlock:act_four"), unlocked("unlock:act_four")],
    );
    assert_eq!(
        produce(&manifest, &FixturePort::new(snapshot)).expect_err("duplicate"),
        ProgressionReferenceError::DuplicateEntry("unlock:act_four".to_owned())
    );
}

#[test]
fn aggregate_collections_stay_inside_their_bounds() {
    let manifest = manifest(&[]);

    let mut many_requirements = locked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    many_requirements.requirements = ProgressionFieldValue::available(
        (0..33)
            .map(|_| requirement("requirement:one", ProgressionReadState::Locked))
            .collect(),
    );
    many_requirements = finish(many_requirements);
    assert_eq!(
        refuse(&manifest, many_requirements),
        ProgressionReferenceError::InvalidInput("requirements")
    );

    let mut many_references = unlocked("unlock:act_four");
    many_references.content_references =
        ProgressionFieldValue::available((0..65).map(|_| content("unlock")).collect());
    many_references = finish(many_references);
    assert_eq!(
        refuse(&manifest, many_references),
        ProgressionReferenceError::InvalidInput("content_references")
    );

    let mut many_related = unlocked("unlock:act_four");
    many_related.related = ProgressionFieldValue::available(
        (0..17)
            .map(|_| related(ProgressionDomain::Unlocks, "unlock:one"))
            .collect(),
    );
    many_related = finish(many_related);
    assert_eq!(
        refuse(&manifest, many_related),
        ProgressionReferenceError::InvalidInput("related")
    );
}

#[test]
fn an_entry_over_its_byte_bound_is_refused_rather_than_truncated() {
    let manifest = manifest(&[]);
    let long = "r".repeat(2_000);
    let mut bulky = locked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    bulky.title = "t".repeat(16_000);
    bulky.description = Some("d".repeat(16_000));
    bulky.requirements = ProgressionFieldValue::available(
        (0..20)
            .map(|index| {
                let mut stated = requirement(
                    &format!("requirement:{index}"),
                    ProgressionReadState::Locked,
                );
                stated.description = ProgressionText::available(&format!("{long}{index}"))
                    .expect("requirement text");
                stated
            })
            .collect(),
    );
    bulky = finish(bulky);
    let refused = refuse(&manifest, bulky);
    assert!(
        matches!(
            refused,
            ProgressionReferenceError::EntryTooLarge { limit, actual }
                if limit == 64 * 1024 && actual > limit
        ),
        "expected an entry byte refusal, got {refused:?}"
    );
}
