// SPDX-License-Identifier: MIT

//! Shared selection fixture: two selectors inside the public scope, one hidden selector, one
//! selector whose candidates are withheld, and one multi-step selector.

use super::*;

/// Manifest for the shared fixture.
pub fn fixture_manifest() -> ContentManifest {
    manifest(&FIXTURE_IDS)
}

/// Two-pick card selector with an eligible pair and one visible refused candidate.
pub fn alpha() -> SelectionDefinitionInput {
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
    strike.references = vec![reference(
        SelectionReferenceKind::Effect,
        "selection-effect.upgrade",
    )];
    let defend = candidate(
        "candidate.defend",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.defend"),
    );
    let mut limit = candidate(
        "candidate.limit",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.removal"),
    );
    limit.eligibility = refused(SelectionBlockReason::LimitReached);

    SelectionDefinitionInput {
        parent: SelectionParentOperation::CombatCardPlay,
        kind: SelectionKind::Card,
        prompt: text("Choose two cards to upgrade."),
        picks: picks(true, Some(2), Some(2)),
        confirmation: SelectionConfirmation::AutomaticClose,
        observed_candidates: vec![
            "candidate.strike".to_owned(),
            "candidate.defend".to_owned(),
            "candidate.limit".to_owned(),
        ],
        candidates: vec![strike, defend, limit],
        ..definition("selection.alpha", ALPHA_GENERATION)
    }
}

/// One-pick co-op player selector that must be confirmed and may be cancelled.
pub fn beta() -> SelectionDefinitionInput {
    let one = candidate(
        "candidate.coop.one",
        SelectionCandidateKind::Player,
        reference(SelectionReferenceKind::Player, COOP_PLAYER_ID),
    );
    SelectionDefinitionInput {
        parent: SelectionParentOperation::CoopRest,
        kind: SelectionKind::Coop,
        prompt: text("Choose a player to receive the shared rest."),
        picks: picks(true, Some(1), Some(1)),
        confirmation: SelectionConfirmation::ExplicitConfirm,
        cancellation: SelectionField::available(SelectionCancellation::Cancel),
        observed_candidates: vec!["candidate.coop.one".to_owned()],
        candidates: vec![one],
        ..definition("selection.beta", BETA_GENERATION)
    }
}

/// Selector hidden from every scope, whose family and candidate are named by coverage records.
pub fn gamma() -> SelectionDefinitionInput {
    let mut sealed = candidate(
        "candidate.gamma",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.strike"),
    );
    sealed.label = withheld_text();
    sealed.visibility = SelectionVisibility::Hidden;
    sealed.eligibility = refused(SelectionBlockReason::Withheld);
    SelectionDefinitionInput {
        kind: SelectionKind::Unsupported("selection.gamma".to_owned()),
        observed_candidates: vec!["candidate.gamma".to_owned()],
        candidates: vec![sealed],
        coverage: vec![
            coverage("selection.gamma", SelectionCoverageState::Unavailable),
            coverage("candidate.gamma", SelectionCoverageState::Unsupported),
        ],
        ..hidden_definition("selection.gamma", 7)
    }
}

/// Visible selector whose entire candidate set is withheld, with an explicitly non-value label.
///
/// The definition itself may be observed; its only candidate may not. A reader therefore reports a
/// denied candidate collection rather than an observed empty one, which is what distinguishes this
/// selector from one whose candidate list is genuinely empty.
pub fn delta() -> SelectionDefinitionInput {
    let mut sealed = candidate(
        "candidate.delta",
        SelectionCandidateKind::Card,
        reference(SelectionReferenceKind::Card, "card.strike"),
    );
    sealed.label = withheld_text();
    sealed.visibility = SelectionVisibility::Hidden;
    sealed.eligibility = refused(SelectionBlockReason::Withheld);
    SelectionDefinitionInput {
        prompt: withheld_text(),
        kind: SelectionKind::Custom("selection.delta".to_owned()),
        observed_candidates: vec!["candidate.delta".to_owned()],
        candidates: vec![sealed],
        ..definition("selection.delta", 9)
    }
}

/// Multi-step upgrade selector whose next pick is the shared player selector.
pub fn epsilon() -> SelectionDefinitionInput {
    let one = candidate(
        "candidate.upgrade",
        SelectionCandidateKind::UpgradeTarget,
        reference(SelectionReferenceKind::Card, "card.removal"),
    );
    SelectionDefinitionInput {
        parent: SelectionParentOperation::CardUpgrade,
        kind: SelectionKind::Upgrade,
        prompt: text("Choose a card to upgrade."),
        steps: SelectionField::available(2),
        next: SelectionField::available(SelectionNextDomain {
            selection_id: "selection.beta".to_owned(),
            candidate_kind: SelectionCandidateKind::Player,
            selector_generation: BETA_GENERATION,
        }),
        observed_candidates: vec!["candidate.upgrade".to_owned()],
        candidates: vec![one],
        ..definition("selection.epsilon", EPSILON_GENERATION)
    }
}

pub fn fixture_definitions() -> Vec<SelectionDefinitionInput> {
    vec![alpha(), beta(), gamma(), delta(), epsilon()]
}

/// Shared fixture: manifest plus the produced immutable catalog.
pub fn fixture() -> (ContentManifest, SelectionCatalog) {
    let manifest = fixture_manifest();
    let catalog = produce(&manifest, snapshot(&manifest, fixture_definitions())).expect("catalog");
    (manifest, catalog)
}
