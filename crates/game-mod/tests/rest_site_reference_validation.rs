// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    RestCoverageState, RestEffect, RestEffectKind, RestField, RestHealModifierKind,
    RestOptionInput, RestOptionKind, RestOptionState, RestProspectiveChange,
    RestProspectiveComparison, RestReferenceKind, RestSelectionDomain, RestSelectionRequirement,
    RestSiteCatalog, RestSiteDefinitionInput, RestSiteError, RestUnavailableReason,
};
use support::{
    available_now, candidate, coverage, definition, gold_cost, heal_amount, heal_effect,
    heal_modifier, manifest, option, per_run_limit, produce, reference, requirement, selection,
    snapshot, text, untyped_option,
};

fn produce_with_sites(
    sites: &[&str],
    definitions: Vec<RestSiteDefinitionInput>,
) -> Result<RestSiteCatalog, RestSiteError> {
    let manifest = manifest(sites);
    produce(&manifest, snapshot(&manifest, definitions))
}

fn produce_one(definition: RestSiteDefinitionInput) -> Result<RestSiteCatalog, RestSiteError> {
    produce_with_sites(&["rest.alpha"], vec![definition])
}

/// Definition whose single option carries the given input.
fn one_option(option: RestOptionInput) -> RestSiteDefinitionInput {
    RestSiteDefinitionInput {
        observed_options: vec![option.option_id.clone()],
        options: vec![option],
        ..definition("rest.alpha", 3)
    }
}

fn with_effects(effects: Vec<RestEffect>) -> RestSiteDefinitionInput {
    let mut heal = option("option.heal", RestOptionKind::Heal);
    heal.effects = effects;
    one_option(heal)
}

fn with_selection(the_selection: RestSelectionRequirement) -> RestSiteDefinitionInput {
    let mut smith = option("option.smith", RestOptionKind::Smith);
    smith.definition = reference(RestReferenceKind::Effect, "rest-effect.smith");
    smith.selection = the_selection;
    one_option(smith)
}

#[test]
fn a_host_option_that_is_neither_described_nor_named_is_refused() {
    let mut definition = definition("rest.alpha", 3);
    definition.observed_options = vec!["option.absent".to_owned()];
    assert_eq!(
        produce_one(definition),
        Err(RestSiteError::UncoveredOption {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.absent".to_owned(),
        })
    );
}

#[test]
fn a_supported_option_cannot_report_itself_unknown() {
    let mut heal = option("option.heal", RestOptionKind::Unknown);
    heal.definition = reference(RestReferenceKind::Effect, "rest-effect.heal");
    assert_eq!(
        produce_one(one_option(heal)),
        Err(RestSiteError::KindDisagreesWithDefinition {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.heal".to_owned(),
            reported: RestOptionKind::Unknown,
            family: "rest-effect".to_owned(),
        })
    );
}

#[test]
fn an_untyped_option_needs_a_named_coverage_record() {
    let untyped = |option_id: &str| {
        untyped_option(
            option_id,
            RestOptionKind::Unsupported(option_id.to_owned()),
            available_now(),
        )
    };
    let mut definition = one_option(untyped("option.toke"));
    assert_eq!(
        produce_one(definition.clone()),
        Err(RestSiteError::UncoveredOption {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.toke".to_owned(),
        })
    );
    definition.coverage = vec![coverage("option.toke", RestCoverageState::Unsupported)];
    assert!(produce_one(definition).is_ok());
}

#[test]
fn duplicate_site_and_option_identities_are_refused() {
    let manifest = manifest(&["rest.alpha"]);
    let mut duplicated = snapshot(
        &manifest,
        vec![definition("rest.alpha", 3), definition("rest.alpha", 4)],
    );
    duplicated.family.definition_count = 1;
    assert_eq!(
        produce(&manifest, duplicated),
        Err(RestSiteError::DuplicateDefinition("rest.alpha".to_owned()))
    );
    let duplicate = RestSiteDefinitionInput {
        observed_options: vec!["option.heal".to_owned()],
        options: vec![
            option("option.heal", RestOptionKind::Heal),
            option("option.heal", RestOptionKind::Heal),
        ],
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_one(duplicate),
        Err(RestSiteError::DuplicateOption {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.heal".to_owned(),
        })
    );
}

#[test]
fn duplicate_sub_records_are_refused() {
    let mut heal = option("option.heal", RestOptionKind::Heal);
    heal.requirements = vec![
        requirement("req.one", sts2_game_mod::RestRequirementState::Satisfied),
        requirement("req.one", sts2_game_mod::RestRequirementState::Satisfied),
    ];
    assert_eq!(
        produce_one(one_option(heal)),
        Err(RestSiteError::DuplicateRequirement {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.heal".to_owned(),
            requirement_id: "req.one".to_owned(),
        })
    );

    let mut costed = option("option.heal", RestOptionKind::Heal);
    costed.costs = vec![gold_cost("cost.one", 1), gold_cost("cost.one", 2)];
    assert_eq!(
        produce_one(one_option(costed)),
        Err(RestSiteError::DuplicateCost {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.heal".to_owned(),
            cost_id: "cost.one".to_owned(),
        })
    );

    let mut limited = option("option.heal", RestOptionKind::Heal);
    limited.limits = vec![per_run_limit("limit.one", 1), per_run_limit("limit.one", 2)];
    assert_eq!(
        produce_one(one_option(limited)),
        Err(RestSiteError::DuplicateCost {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.heal".to_owned(),
            cost_id: "limit.one".to_owned(),
        })
    );

    let mut effected = option("option.heal", RestOptionKind::Heal);
    effected.effects = vec![
        heal_effect("effect.one", heal_amount(30, Vec::new(), 30)),
        heal_effect("effect.one", heal_amount(30, Vec::new(), 30)),
    ];
    assert_eq!(
        produce_one(one_option(effected)),
        Err(RestSiteError::DuplicateEffect {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.heal".to_owned(),
            effect_id: "effect.one".to_owned(),
        })
    );
}

#[test]
fn duplicate_coverage_and_candidate_identities_are_refused() {
    let mut definition = one_option(untyped_option(
        "option.toke",
        RestOptionKind::Unsupported("option.toke".to_owned()),
        available_now(),
    ));
    definition.coverage = vec![
        coverage("option.toke", RestCoverageState::Unsupported),
        coverage("option.toke", RestCoverageState::Unavailable),
    ];
    assert_eq!(
        produce_one(definition),
        Err(RestSiteError::InvalidInput("coverage"))
    );

    let repeated = selection(
        RestSelectionDomain::UpgradeCandidates,
        vec![
            candidate(
                "candidate.one",
                reference(RestReferenceKind::Card, "card.strike"),
            ),
            candidate(
                "candidate.one",
                reference(RestReferenceKind::Card, "card.strike"),
            ),
        ],
    );
    assert_eq!(
        produce_one(with_selection(repeated)),
        Err(RestSiteError::DuplicateCandidate {
            site_id: "rest.alpha".to_owned(),
            option_id: "option.smith".to_owned(),
            candidate_id: "candidate.one".to_owned(),
        })
    );
}

#[test]
fn an_impossible_healing_declaration_is_refused() {
    let heal_effect_without_amount = RestEffect {
        effect_id: "effect.heal".to_owned(),
        label: text("effect.heal"),
        kind: RestEffectKind::Heal,
        target: RestField::unavailable(RestUnavailableReason::NotApplicable),
        healing: None,
        references: Vec::new(),
    };
    let upgraded_with_amount = RestEffect {
        healing: Some(heal_amount(30, Vec::new(), 30)),
        ..heal_effect_without_amount.clone()
    };
    for (effects, expected) in [
        (vec![heal_effect_without_amount], "no healing amount"),
        (
            vec![RestEffect {
                kind: RestEffectKind::UpgradeCard,
                ..upgraded_with_amount
            }],
            "a non-healing effect carrying a healing amount",
        ),
        (
            vec![heal_effect(
                "effect.heal",
                heal_amount(120, Vec::new(), 120),
            )],
            "a base fraction above 100 percent",
        ),
        (
            vec![heal_effect("effect.heal", heal_amount(30, Vec::new(), 45))],
            "a folded total with no contributor to justify it",
        ),
    ] {
        assert_eq!(
            produce_one(with_effects(effects)),
            Err(RestSiteError::InvalidEffect {
                site_id: "rest.alpha".to_owned(),
                option_id: "option.heal".to_owned(),
                effect_id: "effect.heal".to_owned(),
            }),
            "{expected} is refused"
        );
    }
    let justified = heal_effect(
        "effect.heal",
        heal_amount(
            30,
            vec![heal_modifier(
                "modifier.relic",
                RestHealModifierKind::RelicBonus,
                15,
            )],
            45,
        ),
    );
    assert!(
        produce_one(with_effects(vec![justified])).is_ok(),
        "a named contributor justifies the resolved total"
    );
}

#[test]
fn an_impossible_availability_declaration_is_refused() {
    let mut refused_silently = option("option.heal", RestOptionKind::Heal);
    refused_silently.availability = sts2_game_mod::RestOptionAvailability {
        state: RestOptionState::Disabled,
        reason: RestField::unavailable(RestUnavailableReason::NotObserved),
        references: Vec::new(),
    };
    let mut offered_with_reason = option("option.heal", RestOptionKind::Heal);
    offered_with_reason.availability = sts2_game_mod::RestOptionAvailability {
        state: RestOptionState::Available,
        reason: RestField::available(sts2_game_mod::RestBlockReason::LimitReached),
        references: Vec::new(),
    };
    for input in [refused_silently, offered_with_reason] {
        assert_eq!(
            produce_one(one_option(input)),
            Err(RestSiteError::InvalidAvailability {
                site_id: "rest.alpha".to_owned(),
                option_id: "option.heal".to_owned(),
            })
        );
    }
}

#[test]
fn an_impossible_selection_declaration_is_refused() {
    let empty_but_required = RestSelectionRequirement {
        domain: RestSelectionDomain::None,
        required: true,
        ..support::no_selection()
    };
    let impossible_bounds = RestSelectionRequirement {
        minimum: RestField::available(3),
        maximum: RestField::available(1),
        ..selection(RestSelectionDomain::UpgradeCandidates, Vec::new())
    };
    let zero_maximum = RestSelectionRequirement {
        minimum: RestField::unavailable(RestUnavailableReason::NotObserved),
        maximum: RestField::available(0),
        ..selection(RestSelectionDomain::UpgradeCandidates, Vec::new())
    };
    for the_selection in [empty_but_required, impossible_bounds, zero_maximum] {
        assert_eq!(
            produce_one(with_selection(the_selection)),
            Err(RestSiteError::InvalidSelection {
                site_id: "rest.alpha".to_owned(),
                option_id: "option.smith".to_owned(),
            })
        );
    }
}

#[test]
fn a_comparison_must_derive_its_own_completeness() {
    let partial = RestProspectiveChange {
        before: RestField::unavailable(RestUnavailableReason::NotObserved),
        ..support::upgrade_change()
    };
    let claiming = RestProspectiveComparison {
        complete: true,
        changes: vec![partial],
        ..support::comparison("option.smith")
    };
    let mut smith = option("option.smith", RestOptionKind::Smith);
    smith.comparison = Some(claiming);
    assert_eq!(
        produce_one(one_option(smith)),
        Err(RestSiteError::InvalidInput("comparison_complete"))
    );

    let mismatched = support::comparison("option.other");
    let mut smith = option("option.smith", RestOptionKind::Smith);
    smith.comparison = Some(mismatched);
    assert_eq!(
        produce_one(one_option(smith)),
        Err(RestSiteError::InvalidInput("comparison_option"))
    );
}
