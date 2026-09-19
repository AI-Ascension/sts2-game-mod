// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    FailingSelectionSource, FixtureSelectionFailure, FixtureSelectionSource,
    SelectionCandidateListQuery, SelectionDefinitionInput, SelectionEligibilityState,
    SelectionError, SelectionFamilyState, SelectionField, SelectionListQuery,
    SelectionVisibilityScope,
};
use support::{
    ALPHA_GENERATION, EPSILON_GENERATION, action_reference, definition, fixture_definitions,
    fixture_manifest, item_definitions, manifest, manifest_of, manifest_without_selections,
    produce, produce_with, progress, selector, snapshot,
};

#[test]
fn producing_and_reading_never_mutates_or_extra_reads_the_source() {
    let manifest = fixture_manifest();
    let source = FixtureSelectionSource::new(snapshot(&manifest, fixture_definitions()));
    let catalog = produce_with(&manifest, &source).expect("catalog");
    assert_eq!(source.reads(), 1, "one owned snapshot read");
    assert_eq!(source.snapshot().definitions.len(), 5);

    let alpha = catalog
        .definition("selection.alpha")
        .expect("alpha")
        .reference
        .clone();
    let strike = catalog
        .candidate("selection.alpha", "candidate.strike")
        .expect("strike")
        .reference
        .clone();
    let candidates_before = catalog
        .definition("selection.alpha")
        .expect("alpha")
        .candidates
        .clone();

    let mut reader = catalog.reader(SelectionVisibilityScope::Public);
    reader
        .list(&SelectionListQuery {
            locale: "en-US".to_owned(),
            scope: SelectionVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("list");
    reader
        .list_candidates(&SelectionCandidateListQuery {
            selector: selector(&catalog, "selection.alpha", ALPHA_GENERATION),
            scope: SelectionVisibilityScope::Public,
            limit: 8,
            continuation: None,
            progress: None,
        })
        .expect("candidates");
    reader
        .get(&alpha, SelectionVisibilityScope::Public)
        .expect("get");
    reader
        .get_candidate(&strike, SelectionVisibilityScope::Public)
        .expect("candidate");

    let partial = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike"],
    );
    let described = reader
        .describe_progress(&partial, SelectionVisibilityScope::Public)
        .expect("progress");
    reader
        .confirmation(&partial, SelectionVisibilityScope::Public)
        .expect_err("one pick of a two-pick prompt is refused, not described as confirmable");
    let complete = progress(
        &catalog,
        "selection.alpha",
        ALPHA_GENERATION,
        &["candidate.strike", "candidate.defend"],
    );
    reader
        .confirmation(&complete, SelectionVisibilityScope::Public)
        .expect("confirmation");
    reader
        .next_selector(
            &progress(
                &catalog,
                "selection.epsilon",
                EPSILON_GENERATION,
                &["candidate.upgrade"],
            ),
            SelectionVisibilityScope::Public,
        )
        .expect("next");

    assert_eq!(described.picked, 1);
    assert_eq!(
        source.reads(),
        1,
        "no read triggered a hidden second source read"
    );
    assert_eq!(
        catalog
            .definition("selection.alpha")
            .expect("alpha")
            .candidates,
        candidates_before,
        "a read reports the candidates without changing them"
    );
    assert_eq!(
        source.snapshot().definitions.len(),
        5,
        "the retained source snapshot is untouched"
    );
    assert_eq!(
        catalog
            .candidate("selection.alpha", "candidate.strike")
            .expect("strike")
            .eligibility
            .state,
        SelectionEligibilityState::Eligible,
        "reconciling a pick never consumed or moved the candidate"
    );
}

#[test]
fn a_transient_selection_action_cannot_enter_the_static_slice() {
    let manifest = manifest(&["selection.alpha"]);
    let definition = SelectionDefinitionInput {
        action: SelectionField::available(action_reference()),
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![definition])),
        Err(SelectionError::InvalidSelectionAction {
            selection_id: "selection.alpha".to_owned(),
        })
    );
}

#[test]
fn only_sanitized_source_failures_cross_the_reader_boundary() {
    let manifest = fixture_manifest();
    for (failure, expected) in [
        (
            FixtureSelectionFailure::NoActiveSource,
            SelectionError::NoActiveSource,
        ),
        (
            FixtureSelectionFailure::AccessDenied,
            SelectionError::SourceAccessDenied,
        ),
        (
            FixtureSelectionFailure::Malformed,
            SelectionError::MalformedSource,
        ),
    ] {
        assert_eq!(
            produce_with(&manifest, &FailingSelectionSource(failure)),
            Err(expected)
        );
    }
}

#[test]
fn an_absent_selection_family_is_refused_before_any_catalog_exists() {
    let manifest = manifest_of(&item_definitions());
    assert_eq!(
        produce(&manifest, snapshot(&manifest, Vec::new())),
        Err(SelectionError::MissingFamily)
    );
}

#[test]
fn a_family_the_source_cannot_project_reports_a_capability_failure() {
    for (state, expected) in [
        (
            SelectionFamilyState::Unsupported,
            SelectionError::UnsupportedFamily,
        ),
        (
            SelectionFamilyState::Unavailable,
            SelectionError::UnavailableFamily,
        ),
    ] {
        let manifest = manifest_without_selections();
        let mut snapshot = snapshot(&manifest, Vec::new());
        snapshot.family.state = state;
        let catalog = produce(&manifest, snapshot).expect("empty catalog");
        assert!(catalog.is_empty());
        assert_eq!(catalog.family.state, state);

        let mut reader = catalog.reader(SelectionVisibilityScope::Public);
        assert_eq!(
            reader.list(&SelectionListQuery {
                locale: "en-US".to_owned(),
                scope: SelectionVisibilityScope::Public,
                limit: 8,
                continuation: None,
            }),
            Err(expected.clone())
        );
        assert_eq!(
            reader.list_candidates(&SelectionCandidateListQuery {
                selector: selector(&catalog, "selection.alpha", 3),
                scope: SelectionVisibilityScope::Public,
                limit: 8,
                continuation: None,
                progress: None,
            }),
            Err(expected.clone())
        );
        assert_eq!(
            reader.describe_progress(
                &progress(&catalog, "selection.alpha", 3, &[]),
                SelectionVisibilityScope::Public
            ),
            Err(expected)
        );
    }
}

#[test]
fn every_catalog_fence_is_enforced_before_a_prompt_is_published() {
    let manifest = fixture_manifest();
    let base = || snapshot(&manifest, fixture_definitions());

    let mut locale = base();
    locale.locale = "fr-FR".to_owned();
    assert_eq!(
        produce(&manifest, locale),
        Err(SelectionError::LocaleMismatch)
    );

    let mut producer = base();
    producer.producer_version = "other-producer".to_owned();
    assert_eq!(
        produce(&manifest, producer),
        Err(SelectionError::ProducerVersionMismatch)
    );

    let mut family = base();
    family.family.entity_kind = "shop".to_owned();
    assert_eq!(
        produce(&manifest, family),
        Err(SelectionError::FamilyIdentityMismatch)
    );

    let mut count = base();
    count.family.definition_count = 0;
    assert_eq!(
        produce(&manifest, count),
        Err(SelectionError::FamilyCountMismatch)
    );

    let mut binding = base();
    binding.manifest.catalog_generation = 999;
    assert_eq!(
        produce(&manifest, binding),
        Err(SelectionError::ManifestMismatch)
    );
}
