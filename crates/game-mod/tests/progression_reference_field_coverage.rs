// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/progression_reference.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentManifest, ProgressionDomain, ProgressionEntryInput, ProgressionField,
    ProgressionFieldAvailability, ProgressionListQuery, ProgressionProfileKind,
    ProgressionProfileQuery, ProgressionReferenceError, ProgressionVisibilityScope,
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

#[test]
fn every_field_row_is_required_and_a_gap_is_never_silent() {
    let manifest = manifest(&[]);
    for field in ProgressionField::all() {
        let mut entry = unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks);
        entry.fields.retain(|row| row.field != field);
        assert_eq!(
            refuse(&manifest, entry),
            ProgressionReferenceError::MissingFieldCoverage(field.name()),
            "field {} must be required",
            field.name()
        );
    }
}

#[test]
fn every_field_row_is_stated_exactly_once() {
    let manifest = manifest(&[]);
    for field in ProgressionField::all() {
        let mut entry = unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks);
        let row = *entry
            .fields
            .iter()
            .find(|row| row.field == field)
            .expect("row");
        entry.fields.push(row);
        assert_eq!(
            refuse(&manifest, entry),
            ProgressionReferenceError::DuplicateFieldCoverage(field.name()),
            "field {} must be stated once",
            field.name()
        );
    }
}

#[test]
fn a_field_row_must_agree_with_the_value_the_entry_carries() {
    let manifest = manifest(&[]);
    for field in ProgressionField::all() {
        let mut entry = unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks);
        break_row(&mut entry, field);
        assert_eq!(
            refuse(&manifest, entry),
            ProgressionReferenceError::InconsistentField(field.name()),
            "field {} must agree with its carrier",
            field.name()
        );
    }
}

#[test]
fn a_field_row_may_not_be_added_past_the_row_bound() {
    let manifest = manifest(&[]);
    let mut entry = unlocked_entry("unlock:act_four", ProgressionDomain::Unlocks);
    let rows: Vec<ProgressionFieldAvailability> = entry.fields.clone();
    entry.fields.extend(rows.iter().copied());
    entry.fields.extend(rows.iter().copied().take(1));
    assert_eq!(entry.fields.len(), 17);
    assert_eq!(
        refuse(&manifest, entry),
        ProgressionReferenceError::InvalidInput("fields")
    );
}

#[test]
fn an_accepted_catalog_states_one_row_per_field_and_nothing_else() {
    let manifest = manifest(&[]);
    let snapshot = canonical_snapshot(&manifest, 7);
    let catalog = produce(&manifest, &FixturePort::new(snapshot)).expect("catalog");
    let page = catalog
        .list(&ProgressionListQuery {
            locale: "en-US".to_owned(),
            profile: ProgressionProfileQuery::Active,
            scope: ProgressionVisibilityScope::Reference,
            revision: None,
            domain: None,
            state: None,
            limit: 64,
            continuation: None,
        })
        .expect("page");
    assert_eq!(page.entries.len(), canonical_entries().len());
    assert_eq!(ProgressionField::all().len(), 8);
    for summary in &page.entries {
        let entry = catalog
            .get(
                &summary.reference,
                ProgressionVisibilityScope::Reference,
                None,
            )
            .expect("entry");
        assert!(
            entry.fields_are_declared(),
            "{} states every row",
            entry.entry_id
        );
        assert_eq!(
            entry.fields.len(),
            ProgressionField::all().len(),
            "{} states no row twice",
            entry.entry_id
        );
        assert_eq!(
            entry
                .field_status(ProgressionField::Description)
                .expect("row"),
            status_for(entry.description.value().is_some()),
        );
        assert_eq!(
            entry.field_status(ProgressionField::Progress).expect("row"),
            status_for(entry.progress.is_available()),
        );
        assert_eq!(
            entry
                .field_status(ProgressionField::BestValue)
                .expect("row"),
            status_for(entry.best.is_available()),
        );
        assert_eq!(
            entry
                .field_status(ProgressionField::Requirements)
                .expect("row"),
            status_for(entry.requirements.is_available()),
        );
        assert_eq!(
            entry
                .field_status(ProgressionField::ContentReferences)
                .expect("row"),
            status_for(entry.content_references.is_available()),
        );
        assert_eq!(
            entry
                .field_status(ProgressionField::RelatedEntries)
                .expect("row"),
            status_for(entry.related.is_available()),
        );
    }
}

/// The declared row an accepted entry states for a carrier that holds a value or states none.
fn status_for(available: bool) -> sts2_game_mod::ProgressionFieldStatus {
    if available {
        sts2_game_mod::ProgressionFieldStatus::Available
    } else {
        sts2_game_mod::ProgressionFieldStatus::NotObserved
    }
}
