// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    SEL_MAX_DEFINITION_BYTES, SelectionCandidateKind, SelectionCatalog, SelectionDefinitionInput,
    SelectionError, SelectionField, SelectionNextDomain, SelectionProspectiveEffectKind,
    SelectionReferenceKind,
};
use support::{BETA_GENERATION, candidate, definition, effect, produce, reference, snapshot};

/// Produces a snapshot of explicit selection identities and definitions.
fn produce_with_selections(
    ids: &[&str],
    definitions: Vec<SelectionDefinitionInput>,
) -> Result<SelectionCatalog, SelectionError> {
    let manifest = support::manifest(ids);
    produce(&manifest, snapshot(&manifest, definitions))
}

/// Definition whose single observed card candidate carries the given input.
fn one_candidate(candidate: sts2_game_mod::SelectionCandidateInput) -> SelectionDefinitionInput {
    SelectionDefinitionInput {
        observed_candidates: vec![candidate.candidate_id.clone()],
        candidates: vec![candidate],
        ..definition("selection.alpha", 3)
    }
}

/// Card candidate bound to the shared card fixture.
fn strike() -> sts2_game_mod::SelectionCandidateInput {
    candidate(
        "candidate.strike",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.strike"),
    )
}

/// Multi-step declaration naming an explicit next domain.
fn with_next(next: SelectionNextDomain) -> SelectionDefinitionInput {
    SelectionDefinitionInput {
        steps: SelectionField::available(2),
        next: SelectionField::available(next),
        ..definition("selection.alpha", 3)
    }
}

fn next_domain(
    selection_id: &str,
    candidate_kind: SelectionCandidateKind,
    selector_generation: u64,
) -> SelectionNextDomain {
    SelectionNextDomain {
        selection_id: selection_id.to_owned(),
        candidate_kind,
        selector_generation,
    }
}

#[test]
fn a_typed_reference_absent_from_the_snapshot_is_refused() {
    let dangling_selection = SelectionDefinitionInput {
        references: vec![reference(
            SelectionReferenceKind::Selection,
            "selection.absent",
        )],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_with_selections(&["selection.alpha"], vec![dangling_selection]),
        Err(SelectionError::DanglingReference {
            selection_id: "selection.alpha".to_owned(),
            reference_kind: SelectionReferenceKind::Selection,
            id: "selection.absent".to_owned(),
        })
    );

    let mut dangling_candidate = strike();
    dangling_candidate.references = vec![reference(
        SelectionReferenceKind::Candidate,
        "candidate.absent",
    )];
    assert_eq!(
        produce_with_selections(
            &["selection.alpha"],
            vec![one_candidate(dangling_candidate)]
        ),
        Err(SelectionError::DanglingReference {
            selection_id: "selection.alpha".to_owned(),
            reference_kind: SelectionReferenceKind::Candidate,
            id: "candidate.absent".to_owned(),
        })
    );

    let resolved = SelectionDefinitionInput {
        references: vec![reference(
            SelectionReferenceKind::Selection,
            "selection.beta",
        )],
        ..definition("selection.alpha", 3)
    };
    assert!(
        produce_with_selections(
            &["selection.alpha", "selection.beta"],
            vec![resolved, support::beta()]
        )
        .is_ok(),
        "a reference that names a present same-snapshot selector is accepted"
    );
}

#[test]
fn a_record_more_visible_than_its_target_is_refused() {
    let leaking = SelectionDefinitionInput {
        references: vec![reference(
            SelectionReferenceKind::Selection,
            "selection.gamma",
        )],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_with_selections(
            &["selection.alpha", "selection.gamma"],
            vec![leaking, support::gamma()]
        ),
        Err(SelectionError::HiddenReferenceLeak {
            selection_id: "selection.alpha".to_owned(),
            reference_kind: SelectionReferenceKind::Selection,
        }),
        "a visible record never discloses a restricted identity, and the rejection omits it"
    );
}

#[test]
fn a_reference_absent_from_the_content_manifest_is_refused() {
    let unknown = SelectionDefinitionInput {
        references: vec![reference(SelectionReferenceKind::Card, "card.absent")],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_with_selections(&["selection.alpha"], vec![unknown]),
        Err(SelectionError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card.absent".to_owned(),
        })
    );

    let mut targeted = strike();
    let mut documented = effect(
        "effect.upgrade",
        SelectionProspectiveEffectKind::CardUpgraded,
        "6",
        "9",
    );
    documented.target =
        SelectionField::available(reference(SelectionReferenceKind::Relic, "relic.absent"));
    targeted.prospective = vec![documented];
    assert_eq!(
        produce_with_selections(&["selection.alpha"], vec![one_candidate(targeted)]),
        Err(SelectionError::UnknownManifestReference {
            entity_kind: "relic".to_owned(),
            namespaced_id: "relic.absent".to_owned(),
        }),
        "a nested effect target is fenced by the same manifest"
    );
}

#[test]
fn a_definition_exceeding_the_aggregate_byte_bound_is_refused() {
    let mut bulky = strike();
    bulky.detail = SelectionField::available("d".repeat(SEL_MAX_DEFINITION_BYTES));
    let refused = produce_with_selections(&["selection.alpha"], vec![one_candidate(bulky)]);
    assert!(matches!(
        refused,
        Err(SelectionError::DefinitionTooLarge { .. })
    ));
    let Err(SelectionError::DefinitionTooLarge { limit, actual }) = refused else {
        return;
    };
    assert_eq!(limit, SEL_MAX_DEFINITION_BYTES);
    assert!(
        actual > limit,
        "the reported size is the retained estimate that exceeded the bound"
    );
}

#[test]
fn the_next_domain_must_belong_to_the_same_snapshot_generation() {
    assert_eq!(
        produce_with_selections(
            &["selection.alpha", "selection.beta"],
            vec![
                with_next(next_domain(
                    "selection.beta",
                    SelectionCandidateKind::Player,
                    BETA_GENERATION + 1
                )),
                support::beta(),
            ]
        ),
        Err(SelectionError::StaleSelectorReference {
            selection_id: "selection.alpha".to_owned(),
            referenced: BETA_GENERATION + 1,
            current: BETA_GENERATION,
        })
    );
    assert_eq!(
        produce_with_selections(
            &["selection.alpha"],
            vec![with_next(next_domain(
                "selection.absent",
                SelectionCandidateKind::Player,
                BETA_GENERATION
            ))]
        ),
        Err(SelectionError::DanglingReference {
            selection_id: "selection.alpha".to_owned(),
            reference_kind: SelectionReferenceKind::Selection,
            id: "selection.absent".to_owned(),
        })
    );
    assert_eq!(
        produce_with_selections(
            &["selection.alpha"],
            vec![with_next(next_domain(
                "selection.alpha",
                SelectionCandidateKind::Unknown,
                3
            ))]
        ),
        Err(SelectionError::InvalidStepDeclaration {
            selection_id: "selection.alpha".to_owned(),
        }),
        "an unclassified next candidate domain names no real domain"
    );
    assert!(
        produce_with_selections(
            &["selection.alpha", "selection.beta"],
            vec![
                with_next(next_domain(
                    "selection.beta",
                    SelectionCandidateKind::Player,
                    BETA_GENERATION
                )),
                support::beta(),
            ]
        )
        .is_ok(),
        "a next domain naming the current generation is accepted"
    );
}
