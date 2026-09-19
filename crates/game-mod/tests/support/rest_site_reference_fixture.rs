// SPDX-License-Identifier: MIT

//! Shared rest-site fixture: three visible sites and one hidden site, plus their builders.

use sts2_game_mod::{
    RestBlockReason, RestComparisonBasis, RestProspectiveChange, RestProspectiveChangeKind,
    RestProspectiveComparison, RestUpgradePreview,
};

use super::*;

/// Manifest for the shared fixture: three visible sites and one hidden site.
pub fn fixture_manifest() -> ContentManifest {
    manifest(&["rest.alpha", "rest.beta", "rest.gamma", "rest.delta"])
}

fn upgrade_preview() -> RestUpgradePreview {
    RestUpgradePreview {
        candidate: reference(RestReferenceKind::Card, "card.strike"),
        before: RestField::available("6".to_owned()),
        after: RestField::available("9".to_owned()),
        references: Vec::new(),
    }
}

/// One resolved documented upgrade change for the shared comparison fixture.
pub fn upgrade_change() -> RestProspectiveChange {
    RestProspectiveChange {
        change_id: "change.upgrade".to_owned(),
        label: text("Strike 6 -> 9"),
        kind: RestProspectiveChangeKind::CardUpgrade,
        target: RestField::available(reference(RestReferenceKind::Card, "card.strike")),
        before: RestField::available("6".to_owned()),
        after: RestField::available("9".to_owned()),
        basis: RestComparisonBasis::Definition,
        references: Vec::new(),
    }
}

/// Documented comparison for one option, carrying a single resolved upgrade change.
pub fn comparison(option_id: &str) -> RestProspectiveComparison {
    RestProspectiveComparison::from_changes(
        option_id.to_owned(),
        RestComparisonBasis::Definition,
        vec![upgrade_change()],
        RestField::available(upgrade_preview()),
    )
}

/// Owner-declared site whose options cover healing, card work, and every audited extra button.
pub fn alpha() -> RestSiteDefinitionInput {
    let mut heal = option("option.heal", RestOptionKind::Heal);
    heal.description = text("Rest to recover a fraction of maximum health.");
    heal.requirements = vec![requirement("req.health", RestRequirementState::Satisfied)];
    heal.limits = vec![per_run_limit("limit.heal", 1)];
    heal.effects = vec![heal_effect(
        "effect.heal",
        heal_amount(
            30,
            vec![heal_modifier(
                "modifier.relic",
                RestHealModifierKind::RelicBonus,
                5,
            )],
            30,
        ),
    )];
    heal.references = vec![reference(RestReferenceKind::Relic, "relic.burning_blood")];

    let mut smith = option("option.smith", RestOptionKind::Smith);
    smith.costs = vec![gold_cost("cost.smith", 0)];
    smith.definition = reference(RestReferenceKind::Effect, "rest-effect.smith");
    smith.effects = vec![card_effect("effect.upgrade", RestEffectKind::UpgradeCard)];
    smith.selection = selection(
        RestSelectionDomain::UpgradeCandidates,
        vec![candidate(
            "candidate.strike",
            reference(RestReferenceKind::Card, "card.strike"),
        )],
    );
    smith.comparison = Some(comparison("option.smith"));

    let mut mend = option("option.mend", RestOptionKind::Mend);
    mend.definition = reference(RestReferenceKind::Effect, "rest-effect.mend");
    mend.availability = disabled_by(RestBlockReason::RequirementUnsatisfied);

    let mut toke = untyped_option(
        "option.toke",
        RestOptionKind::Unsupported("option.toke".to_owned()),
        available_now(),
    );
    toke.description = text("Audited host button this producer cannot type yet.");

    let mut private = untyped_option(
        "option.private",
        RestOptionKind::Unsupported("option.private".to_owned()),
        available_now(),
    );
    private.label = withheld_text();
    private.description = withheld_text();
    private.visibility = RestVisibility::Hidden;

    let mut owner = option(
        "option.owner",
        RestOptionKind::Custom("option.owner".to_owned()),
    );
    owner.visibility = RestVisibility::OwnerOnly;

    RestSiteDefinitionInput {
        observed_options: vec![
            "option.heal".to_owned(),
            "option.smith".to_owned(),
            "option.mend".to_owned(),
            "option.toke".to_owned(),
            "option.private".to_owned(),
            "option.owner".to_owned(),
        ],
        options: vec![heal, smith, mend, toke, private, owner],
        coverage: vec![
            coverage("option.toke", RestCoverageState::Unsupported),
            coverage("option.private", RestCoverageState::Unavailable),
        ],
        references: vec![
            reference(RestReferenceKind::Site, "rest.beta"),
            reference(RestReferenceKind::Card, "card.strike"),
            reference(RestReferenceKind::Effect, "rest-effect.heal"),
            reference(RestReferenceKind::Potion, "potion.fire"),
        ],
        ..definition("rest.alpha", 3)
    }
}

/// Second visible site carrying the co-op player selector.
pub fn beta() -> RestSiteDefinitionInput {
    let mut lift = option("option.lift", RestOptionKind::Lift);
    lift.definition = reference(RestReferenceKind::Effect, "rest-effect.lift");
    lift.selection = selection(
        RestSelectionDomain::CoopPlayers,
        vec![candidate(
            "candidate.coop.one",
            reference(RestReferenceKind::Player, COOP_PLAYER_ID),
        )],
    );
    RestSiteDefinitionInput {
        observed_options: vec!["option.lift".to_owned()],
        options: vec![lift],
        ..definition("rest.beta", 1)
    }
}

/// Site hidden from every scope, with a withheld typed option.
pub fn gamma() -> RestSiteDefinitionInput {
    let mut option = option(
        "option.gamma",
        RestOptionKind::Custom("option.gamma".to_owned()),
    );
    option.label = withheld_text();
    option.description = withheld_text();
    option.visibility = RestVisibility::Hidden;
    RestSiteDefinitionInput {
        observed_options: vec!["option.gamma".to_owned()],
        options: vec![option],
        ..hidden_definition("rest.gamma", 5)
    }
}

/// Visible site whose entire option set is withheld, with explicitly non-value text.
///
/// The definition itself may be observed; every option inside it may not. A reader therefore
/// reports a denied option collection rather than an observed empty one, which is what
/// distinguishes this site from a site whose option list is genuinely empty.
pub fn delta() -> RestSiteDefinitionInput {
    let mut sealed = option(
        "option.sealed",
        RestOptionKind::Custom("option.sealed".to_owned()),
    );
    sealed.label = withheld_text();
    sealed.description = withheld_text();
    sealed.visibility = RestVisibility::Hidden;
    RestSiteDefinitionInput {
        label: withheld_text(),
        description: withheld_text(),
        observed_options: vec!["option.sealed".to_owned()],
        options: vec![sealed],
        coverage: vec![coverage("option.sealed", RestCoverageState::Unavailable)],
        ..definition("rest.delta", 7)
    }
}

pub fn fixture_definitions() -> Vec<RestSiteDefinitionInput> {
    vec![alpha(), beta(), gamma(), delta()]
}

/// Shared fixture: manifest plus the produced immutable catalog.
pub fn fixture() -> (ContentManifest, RestSiteCatalog) {
    let manifest = fixture_manifest();
    let catalog = produce(&manifest, snapshot(&manifest, fixture_definitions())).expect("catalog");
    (manifest, catalog)
}
