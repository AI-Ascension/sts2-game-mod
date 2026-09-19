// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/selection_reference.rs"]
mod support;

use sts2_game_mod::{
    SEL_MAX_PAGE_ITEMS, SelectionBlockReason, SelectionCandidateKind, SelectionConfirmation,
    SelectionDefinitionInput, SelectionDuplicateRule, SelectionEligibilityState, SelectionError,
    SelectionFieldStatus, SelectionKind, SelectionListQuery, SelectionOrderingRule,
    SelectionParentOperation, SelectionProspectiveEffectKind, SelectionReferenceKind,
    SelectionVisibilityScope,
};
use support::{
    COOP_PLAYER_ID, EPSILON_GENERATION, candidate, definition, fixture, fixture_manifest,
    manifest_with_extra, produce, reference, snapshot, text,
};

#[test]
fn a_produced_catalog_publishes_every_selector_and_audited_candidate() {
    let (manifest, catalog) = fixture();
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(catalog.len(), 5);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.binding.manifest, manifest.cursor_binding());
    assert_eq!(catalog.family.entity_kind, "selection");
    assert_eq!(catalog.family.definition_count, 5);

    let alpha = catalog.definition("selection.alpha").expect("alpha");
    assert_eq!(alpha.parent, SelectionParentOperation::CombatCardPlay);
    assert_eq!(alpha.kind, SelectionKind::Card);
    assert_eq!(
        alpha.prompt.value(),
        Some("Choose two cards to upgrade."),
        "a selector publishes its localized prompt"
    );
    assert_eq!(alpha.selector_generation, 3);
    assert_eq!(alpha.picks.minimum_picks(), Some(2));
    assert_eq!(alpha.picks.maximum_picks(), Some(2));
    assert!(alpha.picks.required);
    assert_eq!(alpha.ordering, SelectionOrderingRule::Stable);
    assert_eq!(alpha.duplicate, SelectionDuplicateRule::Distinct);
    assert_eq!(alpha.confirmation, SelectionConfirmation::AutomaticClose);
    assert_eq!(alpha.observed_candidate_count, 3);
    assert_eq!(alpha.candidates.len(), 3);
    assert_eq!(alpha.candidates_status, SelectionFieldStatus::Available);
    assert_eq!(alpha.coverage_status, SelectionFieldStatus::Available);
    assert!(alpha.coverage.is_empty());
    assert!(alpha.is_single_step());
    assert_eq!(alpha.candidate_kind(), Some(&SelectionCandidateKind::Card));

    let strike = catalog
        .candidate("selection.alpha", "candidate.strike")
        .expect("strike");
    assert_eq!(
        strike.eligibility.state,
        SelectionEligibilityState::Eligible
    );
    assert!(!strike.eligibility.reason.is_available());
    assert!(strike.is_eligible());
    assert_eq!(
        strike.detail.value().map(String::as_str),
        Some("candidate.strike")
    );
    assert_eq!(strike.prospective.len(), 1);

    let limit = catalog
        .candidate("selection.alpha", "candidate.limit")
        .expect("limit");
    assert_eq!(
        limit.eligibility.state,
        SelectionEligibilityState::Ineligible
    );
    assert_eq!(
        limit.eligibility.reason.value(),
        Some(&SelectionBlockReason::LimitReached),
        "a refused candidate always states why it is refused"
    );

    assert!(catalog.definition("selection.absent").is_none());
    assert!(
        catalog
            .candidate("selection.alpha", "candidate.absent")
            .is_none()
    );
}

#[test]
fn documented_prospective_effects_stay_on_their_candidate() {
    let (_, catalog) = fixture();
    let effect = &catalog
        .candidate("selection.alpha", "candidate.strike")
        .expect("strike")
        .prospective[0];
    assert_eq!(effect.kind, SelectionProspectiveEffectKind::CardUpgraded);
    assert_eq!(
        effect.before.value(),
        Some(&"6".to_owned()),
        "the documented value before the pick is kept as stated"
    );
    assert_eq!(effect.after.value(), Some(&"9".to_owned()));
    assert_eq!(
        catalog
            .candidate("selection.alpha", "candidate.strike")
            .expect("strike")
            .references[0]
            .id,
        "selection-effect.upgrade"
    );
    assert!(
        catalog
            .candidate("selection.alpha", "candidate.defend")
            .expect("defend")
            .prospective
            .is_empty(),
        "an unrelated candidate carries no effect of its neighbour"
    );
}

#[test]
fn a_coop_player_selector_resolves_a_player_reference() {
    let (_, catalog) = fixture();
    let beta = catalog.definition("selection.beta").expect("beta");
    assert_eq!(beta.parent, SelectionParentOperation::CoopRest);
    assert_eq!(beta.kind, SelectionKind::Coop);
    assert_eq!(beta.confirmation, SelectionConfirmation::ExplicitConfirm);
    assert!(beta.cancellation.is_available());
    let candidate = catalog
        .candidate("selection.beta", "candidate.coop.one")
        .expect("coop");
    assert_eq!(candidate.kind, SelectionCandidateKind::Player);
    assert_eq!(candidate.definition.kind, SelectionReferenceKind::Player);
    assert_eq!(candidate.definition.id, COOP_PLAYER_ID);
}

#[test]
fn every_named_selector_family_publishes_a_readable_prompt_and_candidate() {
    let families: [(
        &str,
        SelectionKind,
        SelectionParentOperation,
        SelectionCandidateKind,
        &str,
    ); 7] = [
        (
            "selection.combat",
            SelectionKind::Card,
            SelectionParentOperation::CombatCardPlay,
            SelectionCandidateKind::Card,
            "card.strike",
        ),
        (
            "selection.reward",
            SelectionKind::Reward,
            SelectionParentOperation::RewardClaim,
            SelectionCandidateKind::RewardItem,
            "selection-effect.reward",
        ),
        (
            "selection.shop",
            SelectionKind::ShopRemoval,
            SelectionParentOperation::ShopRemoval,
            SelectionCandidateKind::Card,
            "card.removal",
        ),
        (
            "selection.upgrade",
            SelectionKind::Upgrade,
            SelectionParentOperation::CardUpgrade,
            SelectionCandidateKind::UpgradeTarget,
            "card.strike",
        ),
        (
            "selection.rest",
            SelectionKind::RestOption,
            SelectionParentOperation::RestOption,
            SelectionCandidateKind::RestOption,
            "selection-effect.reward",
        ),
        (
            "selection.coop",
            SelectionKind::Coop,
            SelectionParentOperation::CoopRest,
            SelectionCandidateKind::Player,
            COOP_PLAYER_ID,
        ),
        (
            "selection.player",
            SelectionKind::Player,
            SelectionParentOperation::EventChoice,
            SelectionCandidateKind::Player,
            COOP_PLAYER_ID,
        ),
    ];
    let ids: Vec<&str> = families.iter().map(|family| family.0).collect();
    let manifest = support::manifest(&ids);
    let definitions: Vec<SelectionDefinitionInput> = families
        .iter()
        .map(
            |(id, kind, parent, candidate_kind, item)| SelectionDefinitionInput {
                parent: parent.clone(),
                kind: kind.clone(),
                prompt: text(&format!("choose for {id}")),
                observed_candidates: vec![format!("candidate.{id}")],
                candidates: vec![candidate(
                    &format!("candidate.{id}"),
                    candidate_kind.clone(),
                    reference(reference_kind_of(candidate_kind), item),
                )],
                ..definition(id, 1)
            },
        )
        .collect();
    let catalog = produce(&manifest, snapshot(&manifest, definitions)).expect("catalog");
    assert_eq!(catalog.len(), 7);
    for (id, kind, parent, candidate_kind, _) in families {
        let definition = catalog.definition(id).expect("selector");
        assert_eq!(definition.kind, kind);
        assert_eq!(definition.parent, parent);
        assert_eq!(
            definition.prompt.value(),
            Some(format!("choose for {id}").as_str())
        );
        assert_eq!(definition.candidates.len(), 1);
        assert_eq!(definition.candidate_kind(), Some(&candidate_kind));
    }
}

/// Returns the manifest family one candidate kind resolves to in the family fixture.
fn reference_kind_of(kind: &SelectionCandidateKind) -> SelectionReferenceKind {
    match kind {
        SelectionCandidateKind::Card | SelectionCandidateKind::UpgradeTarget => {
            SelectionReferenceKind::Card
        }
        SelectionCandidateKind::Player => SelectionReferenceKind::Player,
        SelectionCandidateKind::RewardItem | SelectionCandidateKind::RestOption => {
            SelectionReferenceKind::Effect
        }
        _ => SelectionReferenceKind::Unknown,
    }
}

#[test]
fn a_multi_step_selector_names_its_next_domain() {
    let (_, catalog) = fixture();
    let epsilon = catalog.definition("selection.epsilon").expect("epsilon");
    assert_eq!(epsilon.selector_generation, EPSILON_GENERATION);
    assert!(!epsilon.is_single_step());
    assert_eq!(epsilon.steps.value(), Some(&2));
    let next = epsilon.next.value().expect("next domain");
    assert_eq!(next.selection_id, "selection.beta");
    assert_eq!(next.candidate_kind, SelectionCandidateKind::Player);
    assert_eq!(next.selector_generation, 5);
}

#[test]
fn selector_lists_page_deterministically_and_hide_restricted_selectors() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(SelectionVisibilityScope::Public);
    let query = |limit: usize| SelectionListQuery {
        locale: "en-US".to_owned(),
        scope: SelectionVisibilityScope::Public,
        limit,
        continuation: None,
    };
    let first = reader.list(&query(2)).expect("first page");
    assert_eq!(first.total, 4);
    assert!(!first.complete);
    assert_eq!(
        first
            .entries
            .iter()
            .map(|entry| entry.reference.selection_id.clone())
            .collect::<Vec<_>>(),
        vec!["selection.alpha", "selection.beta"]
    );
    assert_eq!(first.entries[0].candidate_count, 3);
    assert_eq!(first.entries[0].coverage_count, 0);
    let continuation = first.continuation.expect("continuation");
    let second = reader
        .list(&SelectionListQuery {
            continuation: Some(continuation),
            ..query(2)
        })
        .expect("second page");
    assert_eq!(
        second
            .entries
            .iter()
            .map(|entry| entry.reference.selection_id.clone())
            .collect::<Vec<_>>(),
        vec!["selection.delta", "selection.epsilon"],
        "a hidden selector is never paged into a public list"
    );
    assert!(second.complete);
    assert!(second.continuation.is_none());
}

#[test]
fn exact_lookups_enforce_identity_scope_and_catalog() {
    let (_, catalog) = fixture();
    let reader = catalog.reader(SelectionVisibilityScope::Public);
    let alpha = catalog
        .definition("selection.alpha")
        .expect("alpha")
        .reference
        .clone();
    let lookup = reader
        .get(&alpha, SelectionVisibilityScope::Public)
        .expect("get");
    assert_eq!(lookup.prompt.value(), Some("Choose two cards to upgrade."));

    let absent = support::selector(&catalog, "selection.absent", 3).selection;
    assert_eq!(
        reader.get(&absent, SelectionVisibilityScope::Public),
        Err(SelectionError::NotFound)
    );

    let mut foreign = alpha.clone();
    foreign.catalog.locale = "fr-FR".to_owned();
    assert_eq!(
        reader.get(&foreign, SelectionVisibilityScope::Public),
        Err(SelectionError::StaleReference)
    );
    let hidden = catalog
        .definition("selection.gamma")
        .expect("gamma")
        .reference
        .clone();
    assert_eq!(
        reader.get(&hidden, SelectionVisibilityScope::Public),
        Err(SelectionError::ExcludedByScope)
    );
}

#[test]
fn locale_and_page_size_fences_are_enforced() {
    let (_, catalog) = fixture();
    let mut reader = catalog.reader(SelectionVisibilityScope::Public);
    assert_eq!(
        reader.list(&SelectionListQuery {
            locale: "fr-FR".to_owned(),
            scope: SelectionVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(SelectionError::LocaleMismatch)
    );
    for limit in [0, SEL_MAX_PAGE_ITEMS + 1] {
        assert_eq!(
            reader.list(&SelectionListQuery {
                locale: "en-US".to_owned(),
                scope: SelectionVisibilityScope::Public,
                limit,
                continuation: None,
            }),
            Err(SelectionError::InvalidPageSize)
        );
    }
    let extra = manifest_with_extra(&[(
        sts2_game_mod::SELECTION_REFERENCE_ENTITY_KIND,
        "selection.extra",
    )]);
    assert_eq!(
        produce(&extra, snapshot(&extra, support::fixture_definitions())),
        Err(SelectionError::FamilyCountMismatch),
        "a manifest that inventories a selector the source does not describe is refused"
    );
    assert_eq!(extra.locale, fixture_manifest().locale);
}
