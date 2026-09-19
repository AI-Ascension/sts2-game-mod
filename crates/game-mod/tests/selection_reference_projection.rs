// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    SelectionCandidateKind, SelectionCatalog, SelectionDefinitionInput, SelectionError,
    SelectionFieldStatus, SelectionListQuery, SelectionVisibility, SelectionVisibilityScope,
};
use support::{candidate, definition, fixture, produce, progress, reference, snapshot};

fn produce_many(
    definitions: Vec<SelectionDefinitionInput>,
) -> Result<SelectionCatalog, SelectionError> {
    let ids: Vec<&str> = definitions
        .iter()
        .map(|definition| definition.selection_id.as_str())
        .collect();
    let manifest = support::manifest(&ids);
    produce(&manifest, snapshot(&manifest, definitions))
}

fn public_list(catalog: &SelectionCatalog, scope: SelectionVisibilityScope) -> usize {
    let mut reader = catalog.reader(scope);
    reader
        .list(&SelectionListQuery {
            locale: "en-US".to_owned(),
            scope,
            limit: 16,
            continuation: None,
        })
        .expect("list")
        .total
}

#[test]
fn public_scope_withholds_hidden_selectors_and_candidates() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(SelectionVisibilityScope::Public);
    let public = reader
        .list(&SelectionListQuery {
            locale: "en-US".to_owned(),
            scope: SelectionVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("list");
    assert_eq!(public.total, 4);
    assert!(
        public
            .entries
            .iter()
            .all(|entry| entry.reference.selection_id != "selection.gamma"),
        "a hidden selector never reaches a public page"
    );
    assert_eq!(
        public_list(&catalog, SelectionVisibilityScope::Owner),
        4,
        "an owner scope promotes no hidden selector either"
    );
    let gamma = catalog
        .definition("selection.gamma")
        .expect("gamma")
        .reference
        .clone();
    for scope in [
        SelectionVisibilityScope::Public,
        SelectionVisibilityScope::Reference,
        SelectionVisibilityScope::Owner,
    ] {
        assert_eq!(
            reader.get(&gamma, scope),
            Err(SelectionError::ExcludedByScope)
        );
    }
}

#[test]
fn a_fully_withheld_candidate_collection_reports_denied_rather_than_empty() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    let delta = catalog
        .definition("selection.delta")
        .expect("delta")
        .reference
        .clone();
    let projected = reader
        .get(&delta, SelectionVisibilityScope::Public)
        .expect("get");
    assert_eq!(projected.observed_candidate_count, 1);
    assert!(projected.candidates.is_empty());
    assert_eq!(projected.candidates_status, SelectionFieldStatus::Denied);
    assert_eq!(
        projected.prompt.status(),
        SelectionFieldStatus::Withheld,
        "withheld prompt text stays an explicit non-value"
    );
    assert!(projected.prompt.value().is_none());
    assert!(!projected.prompt.is_available());

    let empty = produce_many(vec![SelectionDefinitionInput {
        observed_candidates: Vec::new(),
        candidates: Vec::new(),
        ..definition("selection.empty", 1)
    }])
    .expect("catalog");
    let empty_definition = empty
        .definition("selection.empty")
        .expect("empty")
        .reference
        .clone();
    let observed_empty = empty
        .reader(SelectionVisibilityScope::Public)
        .get(&empty_definition, SelectionVisibilityScope::Public)
        .expect("get");
    assert!(observed_empty.candidates.is_empty());
    assert_eq!(
        observed_empty.candidates_status,
        SelectionFieldStatus::Available,
        "an observed empty candidate list is not a withheld one"
    );
}

#[test]
fn owner_scope_shows_owner_only_records_but_never_hidden_ones() {
    let open = candidate(
        "candidate.open",
        SelectionCandidateKind::Card,
        reference(sts2_game_mod::SelectionReferenceKind::Card, "card.defend"),
    );
    let mut sealed = candidate(
        "candidate.owner",
        SelectionCandidateKind::Card,
        reference(sts2_game_mod::SelectionReferenceKind::Card, "card.strike"),
    );
    sealed.visibility = SelectionVisibility::OwnerOnly;
    let catalog = produce_many(vec![
        SelectionDefinitionInput {
            observed_candidates: vec!["candidate.open".to_owned(), "candidate.owner".to_owned()],
            candidates: vec![open, sealed],
            ..definition("selection.mixed", 1)
        },
        SelectionDefinitionInput {
            visibility: SelectionVisibility::OwnerOnly,
            ..definition("selection.owner", 2)
        },
    ])
    .expect("catalog");

    assert_eq!(public_list(&catalog, SelectionVisibilityScope::Public), 1);
    assert_eq!(public_list(&catalog, SelectionVisibilityScope::Owner), 2);
    let mixed = catalog
        .definition("selection.mixed")
        .expect("mixed")
        .reference
        .clone();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    for scope in [
        SelectionVisibilityScope::Public,
        SelectionVisibilityScope::Reference,
    ] {
        let narrowed = reader
            .get(&mixed, scope)
            .expect("a visible definition stays readable");
        assert_eq!(
            narrowed.candidates.len(),
            1,
            "an owner-only record never shows in a non-owner scope"
        );
        assert_eq!(narrowed.candidates_status, SelectionFieldStatus::Withheld);
        assert!(
            narrowed
                .candidates
                .values()
                .all(|candidate| candidate.reference.candidate_id != "candidate.owner"),
            "the withheld identity is dropped rather than disclosed"
        );
    }
    let owner = reader
        .get(&mixed, SelectionVisibilityScope::Owner)
        .expect("owner scope");
    assert_eq!(owner.candidates.len(), 2);
    assert_eq!(owner.candidates_status, SelectionFieldStatus::Available);
    let owner_only = catalog
        .definition("selection.owner")
        .expect("owner")
        .reference
        .clone();
    assert!(
        reader
            .get(&owner_only, SelectionVisibilityScope::Owner)
            .is_ok()
    );
    assert_eq!(
        reader.get(&owner_only, SelectionVisibilityScope::Public),
        Err(SelectionError::ExcludedByScope)
    );
}

#[test]
fn a_withheld_candidate_is_never_resolved_by_an_exact_lookup_or_a_pick() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    let open = catalog
        .candidate("selection.alpha", "candidate.strike")
        .expect("strike")
        .reference
        .clone();
    let resolved = reader
        .get_candidate(&open, SelectionVisibilityScope::Public)
        .expect("visible candidate");
    assert_eq!(resolved.label.value(), Some("candidate.strike"));
    let withheld = catalog
        .candidate("selection.delta", "candidate.delta")
        .expect("delta candidate")
        .reference
        .clone();
    for scope in [
        SelectionVisibilityScope::Public,
        SelectionVisibilityScope::Reference,
        SelectionVisibilityScope::Owner,
    ] {
        assert_eq!(
            reader.get_candidate(&withheld, scope),
            Err(SelectionError::ExcludedByScope)
        );
    }
    assert_eq!(
        reader.describe_progress(
            &progress(&catalog, "selection.delta", 9, &["candidate.delta"]),
            SelectionVisibilityScope::Public
        ),
        Err(SelectionError::ExcludedByScope),
        "a withheld candidate is never reconciled into a legal prompt state"
    );
    let alpha = catalog
        .definition("selection.alpha")
        .expect("alpha")
        .reference
        .clone();
    let projected = reader
        .get(&alpha, SelectionVisibilityScope::Public)
        .expect("get");
    assert_eq!(projected.prompt.status(), SelectionFieldStatus::Available);
    assert_eq!(projected.candidates_status, SelectionFieldStatus::Available);
    assert_eq!(projected.coverage_status, SelectionFieldStatus::Available);
}
