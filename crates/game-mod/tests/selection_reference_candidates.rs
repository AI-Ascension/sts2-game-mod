// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    SelectionCandidateKind, SelectionCandidateListQuery, SelectionCatalog,
    SelectionDefinitionInput, SelectionDuplicateRule, SelectionError, SelectionField,
    SelectionFieldStatus, SelectionProgressInput, SelectionVisibilityScope,
};
use support::{
    ALPHA_GENERATION, candidate, definition, fixture, picks, produce, progress, reference,
    selector, snapshot,
};

/// Produces a single-selector catalog so one declaration can be exercised in isolation.
fn produce_one(definition: SelectionDefinitionInput) -> Result<SelectionCatalog, SelectionError> {
    let manifest = support::manifest(&[definition.selection_id.as_str()]);
    produce(&manifest, snapshot(&manifest, vec![definition]))
}

fn candidates(
    catalog: &SelectionCatalog,
    selection_id: &str,
    generation: u64,
    progress: Option<SelectionProgressInput>,
    limit: usize,
) -> Result<sts2_game_mod::SelectionCandidatePage, SelectionError> {
    let mut reader = catalog.reader(SelectionVisibilityScope::Public);
    reader.list_candidates(&SelectionCandidateListQuery {
        selector: selector(catalog, selection_id, generation),
        scope: SelectionVisibilityScope::Public,
        limit,
        continuation: None,
        progress,
    })
}

#[test]
fn a_candidate_page_reports_eligibility_and_the_reported_constraints() {
    let (_, catalog) = fixture();
    let page = candidates(&catalog, "selection.alpha", ALPHA_GENERATION, None, 8).expect("page");
    assert_eq!(page.total, 3);
    assert!(page.complete);
    assert_eq!(page.picked, 0);
    assert_eq!(page.remaining, SelectionField::available(2));
    assert_eq!(page.candidates_status, SelectionFieldStatus::Available);
    assert_eq!(
        page.entries
            .iter()
            .map(|entry| entry.reference.candidate_id.clone())
            .collect::<Vec<_>>(),
        vec!["candidate.defend", "candidate.limit", "candidate.strike"],
        "candidates stay in stable identity order"
    );
    let strike = page
        .entries
        .iter()
        .find(|entry| entry.reference.candidate_id == "candidate.strike")
        .expect("strike");
    assert_eq!(strike.kind, SelectionCandidateKind::Card);
    assert_eq!(strike.definition.id, "card.strike");
    assert_eq!(strike.effect_count, 1);
    let limit = page
        .entries
        .iter()
        .find(|entry| entry.reference.candidate_id == "candidate.limit")
        .expect("limit");
    assert!(limit.eligibility.reason.is_available());
}

#[test]
fn a_two_pick_sequence_updates_remaining_and_available_candidates() {
    let (_, catalog) = fixture();
    let first = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike"],
    );
    let after_first = catalog
        .reader(SelectionVisibilityScope::Public)
        .describe_progress(&first, SelectionVisibilityScope::Public)
        .expect("progress");
    assert_eq!(after_first.picked, 1);
    assert_eq!(after_first.remaining, SelectionField::available(1));
    assert_eq!(
        after_first.available, 2,
        "the picked candidate leaves the selectable set"
    );
    assert!(!after_first.complete);
    assert!(!after_first.confirmation_state.may_confirm);
    assert_eq!(
        after_first.confirmation_state.unmet,
        SelectionField::available(1)
    );

    let second_page = candidates(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        Some(first.clone()),
        8,
    )
    .expect("page");
    assert_eq!(second_page.total, 2);
    assert_eq!(second_page.picked, 1);
    assert_eq!(second_page.remaining, SelectionField::available(1));
    assert!(
        second_page
            .entries
            .iter()
            .all(|entry| entry.reference.candidate_id != "candidate.strike"),
        "a no-repeat selector no longer offers a candidate already picked"
    );

    let both = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike", "candidate.defend"],
    );
    let complete = catalog
        .reader(SelectionVisibilityScope::Public)
        .describe_progress(&both, SelectionVisibilityScope::Public)
        .expect("progress");
    assert_eq!(complete.picked, 2);
    assert_eq!(complete.remaining, SelectionField::available(0));
    assert_eq!(complete.available, 1);
    assert!(complete.complete);
    assert!(complete.confirmation_state.may_confirm);
    assert_eq!(
        complete.confirmation_state.unmet,
        SelectionField::available(0)
    );
    assert_eq!(
        complete.selected.len(),
        2,
        "the reconciled progress carries the picks the caller reported"
    );
}

#[test]
fn an_early_confirmation_is_refused_with_the_exact_shortfall() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    for (selected, observed) in [(vec![], 0_u32), (vec!["candidate.strike"], 1_u32)] {
        let short = progress(&catalog, "selection.alpha", ALPHA_GENERATION, &selected);
        assert_eq!(
            reader.confirmation(&short, SelectionVisibilityScope::Public),
            Err(SelectionError::PrematureConfirmation {
                selection_id: "selection.alpha".to_owned(),
                required: 2,
                observed,
            })
        );
    }
    let complete = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike", "candidate.defend"],
    );
    let state = reader
        .confirmation(&complete, SelectionVisibilityScope::Public)
        .expect("confirmation");
    assert!(state.may_confirm);
    assert_eq!(state.unmet, SelectionField::available(0));
    assert!(
        !state.reason.is_available(),
        "a confirmable prompt carries no refusal"
    );
}

#[test]
fn a_duplicate_choice_is_refused_unless_the_selector_allows_repeats() {
    let (_, catalog) = fixture();
    let repeated = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike", "candidate.strike"],
    );
    assert_eq!(
        catalog
            .reader(SelectionVisibilityScope::Public)
            .describe_progress(&repeated, SelectionVisibilityScope::Public),
        Err(SelectionError::DuplicateChoice {
            selection_id: "selection.alpha".to_owned(),
            candidate_id: "candidate.strike".to_owned(),
        })
    );

    let repeatable = produce_one(SelectionDefinitionInput {
        duplicate: SelectionDuplicateRule::RepeatsAllowed,
        picks: picks(true, Some(1), Some(2)),
        observed_candidates: vec!["candidate.one".to_owned()],
        candidates: vec![candidate(
            "candidate.one",
            SelectionCandidateKind::Card,
            reference(sts2_game_mod::SelectionReferenceKind::Card, "card.strike"),
        )],
        ..definition("selection.repeat", 1)
    })
    .expect("catalog");
    let repeated = progress(
        &repeatable,
        "selection.repeat",
        1,
        &["candidate.one", "candidate.one"],
    );
    let described = repeatable
        .reader(SelectionVisibilityScope::Public)
        .describe_progress(&repeated, SelectionVisibilityScope::Public)
        .expect("a repeating selector accepts the same choice twice");
    assert_eq!(described.picked, 2);
    assert_eq!(described.remaining, SelectionField::available(0));
    assert!(described.complete);
    let mut reader = repeatable.reader(SelectionVisibilityScope::Public);
    let page = reader
        .list_candidates(&SelectionCandidateListQuery {
            selector: selector(&repeatable, "selection.repeat", 1),
            scope: SelectionVisibilityScope::Public,
            limit: 4,
            continuation: None,
            progress: Some(repeated),
        })
        .expect("page");
    assert_eq!(
        page.total, 1,
        "a repeating selector keeps offering its choice"
    );
}

#[test]
fn an_unknown_candidate_and_an_excess_pick_count_are_refused() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    let unknown = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike", "candidate.absent"],
    );
    assert_eq!(
        reader.describe_progress(&unknown, SelectionVisibilityScope::Public),
        Err(SelectionError::UnknownCandidate {
            selection_id: "selection.alpha".to_owned(),
            candidate_id: "candidate.absent".to_owned(),
        })
    );
    let excess = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike", "candidate.defend", "candidate.limit"],
    );
    assert_eq!(
        reader.describe_progress(&excess, SelectionVisibilityScope::Public),
        Err(SelectionError::ExcessPicks {
            selection_id: "selection.alpha".to_owned(),
            maximum: 2,
            observed: 3,
        })
    );
}

#[test]
fn candidate_continuations_are_single_use_and_bound_to_one_pick_state() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(SelectionVisibilityScope::Public);
    let query = |limit: usize| SelectionCandidateListQuery {
        selector: selector(&catalog, "selection.alpha", ALPHA_GENERATION),
        scope: SelectionVisibilityScope::Public,
        limit,
        continuation: None,
        progress: None,
    };
    let first = reader.list_candidates(&query(2)).expect("page");
    assert_eq!(first.total, 3);
    assert!(!first.complete);
    let continuation = first.continuation.expect("continuation");
    let second = reader
        .list_candidates(&SelectionCandidateListQuery {
            continuation: Some(continuation.clone()),
            ..query(2)
        })
        .expect("second page");
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.reference.candidate_id.clone())
            .collect::<Vec<_>>(),
        vec!["candidate.strike"]
    );
    assert!(second.complete);
    assert_eq!(
        reader.list_candidates(&SelectionCandidateListQuery {
            continuation: Some(continuation),
            ..query(2)
        }),
        Err(SelectionError::InvalidContinuation),
        "a candidate continuation is consumed on first use"
    );

    let picked = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike"],
    );
    let mut other = catalog.reader(SelectionVisibilityScope::Public);
    let page = other
        .list_candidates(&SelectionCandidateListQuery {
            progress: Some(picked.clone()),
            ..query(8)
        })
        .expect("page");
    let continuation = page.continuation;
    assert!(
        continuation.is_none(),
        "a complete page retains no continuation at all"
    );
    let mut third = catalog.reader(SelectionVisibilityScope::Public);
    let partial = third
        .list_candidates(&SelectionCandidateListQuery {
            progress: Some(picked),
            ..query(1)
        })
        .expect("page");
    let token = partial.continuation.expect("continuation");
    assert_eq!(
        third.list_candidates(&SelectionCandidateListQuery {
            progress: Some(progress(&catalog, "selection.alpha", ALPHA_GENERATION, &[])),
            continuation: Some(token),
            ..query(1)
        }),
        Err(SelectionError::InvalidContinuation),
        "a continuation bound to one pick state cannot resume another"
    );
    let mut fourth = catalog.reader(SelectionVisibilityScope::Public);
    let page = fourth
        .list_candidates(&SelectionCandidateListQuery {
            progress: Some(progress(&catalog, "selection.alpha", ALPHA_GENERATION, &[])),
            ..query(1)
        })
        .expect("page");
    let token = page.continuation.expect("continuation");
    assert_eq!(
        reader.list_candidates(&SelectionCandidateListQuery {
            continuation: Some(token),
            ..query(1)
        }),
        Err(SelectionError::InvalidContinuation),
        "a continuation never crosses readers"
    );
}
