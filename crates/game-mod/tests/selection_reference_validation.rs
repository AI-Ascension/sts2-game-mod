// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    SEL_MAX_CANDIDATES, SEL_MAX_TEXT_BYTES, SelectionBlockReason, SelectionCancellation,
    SelectionCandidateKind, SelectionCatalog, SelectionConfirmation, SelectionCoverageState,
    SelectionDefinitionInput, SelectionEligibility, SelectionEligibilityState, SelectionError,
    SelectionField, SelectionKind, SelectionNextDomain, SelectionProspectiveEffect,
    SelectionProspectiveEffectKind, SelectionReferenceKind, SelectionUnavailableReason,
};
use support::{
    candidate, coverage, definition, effect, eligible, picks, produce, reference, refused,
    snapshot, text,
};

/// Produces a snapshot of explicit selection identities and definitions.
fn produce_with_selections(
    ids: &[&str],
    definitions: Vec<SelectionDefinitionInput>,
) -> Result<SelectionCatalog, SelectionError> {
    let manifest = support::manifest(ids);
    produce(&manifest, snapshot(&manifest, definitions))
}

/// Produces a single-selector catalog so one declaration can be exercised in isolation.
fn produce_one(definition: SelectionDefinitionInput) -> Result<SelectionCatalog, SelectionError> {
    let id = definition.selection_id.clone();
    produce_with_selections(&[id.as_str()], vec![definition])
}

/// Definition with one observed card candidate and no other decoration.
fn one_candidate() -> SelectionDefinitionInput {
    SelectionDefinitionInput {
        observed_candidates: vec!["candidate.strike".to_owned()],
        candidates: vec![candidate(
            "candidate.strike",
            SelectionCandidateKind::Card,
            reference(SelectionReferenceKind::Card, "card.strike"),
        )],
        ..definition("selection.alpha", 3)
    }
}

#[test]
fn a_host_candidate_that_is_neither_described_nor_named_is_refused() {
    let definition = SelectionDefinitionInput {
        observed_candidates: vec!["candidate.absent".to_owned()],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(definition),
        Err(SelectionError::UncoveredCandidate {
            selection_id: "selection.alpha".to_owned(),
            candidate_id: "candidate.absent".to_owned(),
        })
    );
}

#[test]
fn an_untyped_selector_must_cover_itself_and_every_candidate_it_presents() {
    let untyped = || SelectionDefinitionInput {
        kind: SelectionKind::Unsupported("selection.alpha".to_owned()),
        ..one_candidate()
    };
    assert_eq!(
        produce_one(untyped()),
        Err(SelectionError::UncoveredSelection {
            selection_id: "selection.alpha".to_owned(),
        })
    );
    let partially_covered = SelectionDefinitionInput {
        coverage: vec![coverage(
            "selection.alpha",
            SelectionCoverageState::Unavailable,
        )],
        ..untyped()
    };
    assert_eq!(
        produce_one(partially_covered),
        Err(SelectionError::UncoveredCandidate {
            selection_id: "selection.alpha".to_owned(),
            candidate_id: "candidate.strike".to_owned(),
        })
    );
    let covered = SelectionDefinitionInput {
        coverage: vec![
            coverage("selection.alpha", SelectionCoverageState::Unavailable),
            coverage("candidate.strike", SelectionCoverageState::Unsupported),
        ],
        ..untyped()
    };
    let catalog = produce_one(covered).expect("a fully named untyped prompt is accepted");
    assert_eq!(
        catalog
            .definition("selection.alpha")
            .expect("alpha")
            .coverage
            .len(),
        2
    );
}

#[test]
fn a_selector_that_contradicts_the_candidate_family_it_presents_is_refused() {
    let contradicting = SelectionDefinitionInput {
        kind: SelectionKind::Unknown,
        coverage: vec![
            coverage("selection.alpha", SelectionCoverageState::Unavailable),
            coverage("candidate.strike", SelectionCoverageState::Unsupported),
        ],
        ..one_candidate()
    };
    assert_eq!(
        produce_one(contradicting),
        Err(SelectionError::KindDisagreesWithDefinition {
            selection_id: "selection.alpha".to_owned(),
            reported: SelectionKind::Unknown,
            family: "card".to_owned(),
        }),
        "an unknown selector never stays unknown while a typed candidate names a known family"
    );
}

#[test]
fn a_repeated_candidate_or_coverage_identity_is_refused() {
    let strike = candidate(
        "candidate.strike",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.strike"),
    );
    let repeated = SelectionDefinitionInput {
        observed_candidates: vec!["candidate.strike".to_owned()],
        candidates: vec![strike.clone(), strike],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(repeated),
        Err(SelectionError::DuplicateCandidate {
            selection_id: "selection.alpha".to_owned(),
            candidate_id: "candidate.strike".to_owned(),
        })
    );

    let twice_covered = SelectionDefinitionInput {
        coverage: vec![
            coverage("selection.alpha", SelectionCoverageState::Unavailable),
            coverage("selection.alpha", SelectionCoverageState::Unsupported),
        ],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(twice_covered),
        Err(SelectionError::InvalidInput("coverage"))
    );
}

#[test]
fn a_coverage_record_that_names_nothing_observed_is_refused() {
    let phantom = SelectionDefinitionInput {
        coverage: vec![coverage(
            "candidate.phantom",
            SelectionCoverageState::Unavailable,
        )],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(phantom),
        Err(SelectionError::InvalidCoverageRecord {
            selection_id: "selection.alpha".to_owned(),
            target_id: "candidate.phantom".to_owned(),
        })
    );

    let self_named = SelectionDefinitionInput {
        coverage: vec![coverage(
            "selection.alpha",
            SelectionCoverageState::Unavailable,
        )],
        ..definition("selection.alpha", 3)
    };
    let catalog = produce_one(self_named).expect("a selector may name its own coverage");
    assert_eq!(
        catalog
            .definition("selection.alpha")
            .expect("alpha")
            .coverage
            .len(),
        1
    );
}

#[test]
fn an_impossible_pick_rule_is_refused() {
    for impossible in [
        picks(true, Some(2), Some(1)),
        picks(true, Some(0), None),
        picks(false, None, Some(0)),
        picks(true, None, Some(33)),
    ] {
        assert_eq!(
            produce_one(SelectionDefinitionInput {
                picks: impossible,
                ..definition("selection.alpha", 3)
            }),
            Err(SelectionError::InvalidPickRule {
                selection_id: "selection.alpha".to_owned(),
            })
        );
    }
    let satisfiable = produce_one(SelectionDefinitionInput {
        picks: picks(true, Some(1), Some(3)),
        ..definition("selection.alpha", 3)
    });
    assert!(satisfiable.is_ok());
}

#[test]
fn confirmation_and_cancellation_may_not_contradict_each_other() {
    let abandoning = SelectionDefinitionInput {
        confirmation: SelectionConfirmation::AutomaticClose,
        cancellation: SelectionField::available(SelectionCancellation::Cancel),
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(abandoning),
        Err(SelectionError::InvalidConfirmation {
            selection_id: "selection.alpha".to_owned(),
        })
    );
    let explicit = produce_one(SelectionDefinitionInput {
        confirmation: SelectionConfirmation::ExplicitConfirm,
        cancellation: SelectionField::available(SelectionCancellation::Cancel),
        ..definition("selection.alpha", 3)
    });
    assert!(
        explicit.is_ok(),
        "a confirmable prompt may also be cancelable"
    );
}

#[test]
fn a_multi_step_declaration_must_name_a_consistent_next_domain() {
    for steps in [0_u32, 1, 33] {
        let declared = SelectionDefinitionInput {
            steps: SelectionField::available(steps),
            next: if steps == 1 {
                SelectionField::available(SelectionNextDomain {
                    selection_id: "selection.alpha".to_owned(),
                    candidate_kind: SelectionCandidateKind::Card,
                    selector_generation: 3,
                })
            } else {
                SelectionField::unavailable(SelectionUnavailableReason::NotApplicable)
            },
            ..definition("selection.alpha", 3)
        };
        assert_eq!(
            produce_one(declared),
            Err(SelectionError::InvalidStepDeclaration {
                selection_id: "selection.alpha".to_owned(),
            }),
            "steps {steps} never describe a consistent sequence"
        );
    }
}

#[test]
fn eligibility_state_and_reason_must_agree() {
    let cases = [
        SelectionEligibility {
            reason: SelectionField::available(SelectionBlockReason::LimitReached),
            ..eligible()
        },
        SelectionEligibility {
            state: SelectionEligibilityState::Ineligible,
            reason: SelectionField::unavailable(SelectionUnavailableReason::NotObserved),
            references: Vec::new(),
        },
        refused(SelectionBlockReason::Withheld),
    ];
    for eligibility in cases {
        let mut strike = candidate(
            "candidate.strike",
            SelectionCandidateKind::Card,
            reference(SelectionReferenceKind::Card, "card.strike"),
        );
        strike.eligibility = eligibility;
        assert_eq!(
            produce_one(SelectionDefinitionInput {
                observed_candidates: vec!["candidate.strike".to_owned()],
                candidates: vec![strike],
                ..definition("selection.alpha", 3)
            }),
            Err(SelectionError::InvalidEligibility {
                selection_id: "selection.alpha".to_owned(),
                candidate_id: "candidate.strike".to_owned(),
            }),
            "a visible candidate never reports a withheld refusal and a refusal always states why"
        );
    }
}

#[test]
fn a_prospective_effect_that_would_change_nothing_is_refused() {
    for broken in [
        effect(
            "effect.flat",
            SelectionProspectiveEffectKind::CardUpgraded,
            "6",
            "6",
        ),
        SelectionProspectiveEffect {
            before: SelectionField::unavailable(SelectionUnavailableReason::NotObserved),
            after: SelectionField::unavailable(SelectionUnavailableReason::NotObserved),
            ..effect(
                "effect.unstated",
                SelectionProspectiveEffectKind::CardUpgraded,
                "6",
                "9",
            )
        },
    ] {
        let mut strike = candidate(
            "candidate.strike",
            SelectionCandidateKind::Card,
            reference(SelectionReferenceKind::Card, "card.strike"),
        );
        strike.prospective = vec![broken];
        assert!(matches!(
            produce_one(SelectionDefinitionInput {
                observed_candidates: vec!["candidate.strike".to_owned()],
                candidates: vec![strike],
                ..definition("selection.alpha", 3)
            }),
            Err(SelectionError::InvalidProspectiveEffect { .. })
        ));
    }
    let mut strike = candidate(
        "candidate.strike",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.strike"),
    );
    strike.prospective = vec![effect(
        "effect.upgrade",
        SelectionProspectiveEffectKind::CardUpgraded,
        "6",
        "9",
    )];
    let real_change = SelectionDefinitionInput {
        observed_candidates: vec!["candidate.strike".to_owned()],
        candidates: vec![strike],
        ..definition("selection.alpha", 3)
    };
    assert!(
        produce_one(real_change).is_ok(),
        "a documented change is published"
    );
}

#[test]
fn a_definition_exceeding_its_local_bounds_or_carrying_an_invalid_identity_is_refused() {
    let mut too_many = definition("selection.alpha", 3);
    too_many.candidates = (0..=SEL_MAX_CANDIDATES)
        .map(|index| {
            candidate(
                &format!("candidate.{index}"),
                SelectionCandidateKind::Card,
                reference(SelectionReferenceKind::Card, "card.strike"),
            )
        })
        .collect();
    assert_eq!(
        produce_one(too_many),
        Err(SelectionError::InvalidInput("candidates"))
    );
    assert_eq!(
        produce_with_selections(&["selection.alpha"], vec![definition("selection alpha", 3)]),
        Err(SelectionError::InvalidInput("selection_id"))
    );
    let oversized_prompt = SelectionDefinitionInput {
        prompt: text(&"p".repeat(SEL_MAX_TEXT_BYTES + 1)),
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(oversized_prompt),
        Err(SelectionError::InvalidInput("prompt"))
    );
    let invalid_candidate = SelectionDefinitionInput {
        candidates: vec![candidate(
            "candidate strike",
            SelectionCandidateKind::Card,
            reference(SelectionReferenceKind::Card, "card.strike"),
        )],
        ..definition("selection.alpha", 3)
    };
    assert_eq!(
        produce_one(invalid_candidate),
        Err(SelectionError::InvalidInput("candidate_id"))
    );
}

#[test]
fn the_shared_fixture_passes_every_declaration_check() {
    let (_, catalog) = support::fixture();
    assert_eq!(catalog.len(), 5, "no shared fixture declaration is refused");
    assert_eq!(
        catalog
            .definition("selection.alpha")
            .expect("alpha")
            .observed_candidate_count,
        3
    );
}
