// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    SELECTION_REFERENCE_CARD_KIND, SelectionConfirmation, SelectionError, SelectionProgressInput,
    SelectionVisibilityScope,
};
use support::{ALPHA_GENERATION, BETA_GENERATION, EPSILON_GENERATION, fixture, progress, selector};

#[test]
fn a_stale_or_foreign_selector_reference_is_refused() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    let stale = progress(&catalog, "selection.alpha", ALPHA_GENERATION + 1, &[]);
    assert_eq!(
        reader.describe_progress(&stale, SelectionVisibilityScope::Public),
        Err(SelectionError::StaleSelectorReference {
            selection_id: "selection.alpha".to_owned(),
            referenced: ALPHA_GENERATION + 1,
            current: ALPHA_GENERATION,
        })
    );
    let mut foreign = selector(&catalog, "selection.alpha", ALPHA_GENERATION);
    foreign.selection.catalog.manifest =
        support::manifest_with_extra(&[(SELECTION_REFERENCE_CARD_KIND, "card.extra")])
            .cursor_binding();
    assert_eq!(
        reader.describe_progress(
            &SelectionProgressInput {
                selector: foreign,
                selected: Vec::new(),
            },
            SelectionVisibilityScope::Public
        ),
        Err(SelectionError::StaleReference)
    );
}

#[test]
fn next_selector_resolves_the_next_domain_and_refuses_an_incomplete_choice() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    let empty = progress(&catalog, "selection.epsilon", EPSILON_GENERATION, &[]);
    assert_eq!(
        reader.next_selector(&empty, SelectionVisibilityScope::Public),
        Err(SelectionError::IncompleteSelection {
            selection_id: "selection.epsilon".to_owned(),
            required: 1,
            observed: 0,
        })
    );
    let chosen = progress(
        &catalog,
        "selection.epsilon",
        EPSILON_GENERATION,
        &["candidate.upgrade"],
    );
    let next = reader
        .next_selector(&chosen, SelectionVisibilityScope::Public)
        .expect("next")
        .expect("a multi-step selector names its next domain");
    assert_eq!(next.selection.selection_id, "selection.beta");
    assert_eq!(next.selector_generation, BETA_GENERATION);
    let beta_pick = progress(
        &catalog,
        "selection.beta",
        BETA_GENERATION,
        &["candidate.coop.one"],
    );
    assert_eq!(
        reader.next_selector(&beta_pick, SelectionVisibilityScope::Public),
        Ok(None),
        "the last step of a sequence ends there"
    );
    assert_eq!(
        catalog
            .definition("selection.beta")
            .expect("beta")
            .confirmation,
        SelectionConfirmation::ExplicitConfirm
    );
}
