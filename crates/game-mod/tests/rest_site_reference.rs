// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    RestFamilyState, RestFieldStatus, RestHealModifierKind, RestOptionListQuery, RestOptionState,
    RestReferenceKind, RestRoundingMode, RestSelectionDomain, RestSiteDefinitionReference,
    RestSiteError, RestSiteListQuery, RestVisibilityScope,
};
use support::{fixture, manifest_with_extra, produce, snapshot};

fn site_reference(
    catalog: &sts2_game_mod::RestSiteCatalog,
    site_id: &str,
) -> RestSiteDefinitionReference {
    catalog.definition(site_id).expect("site").reference.clone()
}

#[test]
fn a_produced_catalog_publishes_every_site_and_audited_option() {
    let (manifest, catalog) = fixture();
    assert_eq!(catalog.locale(), "en-US");
    assert_eq!(catalog.len(), 4);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.binding.manifest, manifest.cursor_binding());
    assert_eq!(catalog.family.state, RestFamilyState::Handled);
    assert_eq!(catalog.family.definition_count, 4);

    let alpha = catalog.definition("rest.alpha").expect("alpha");
    assert_eq!(alpha.option_set_generation, 3);
    assert_eq!(alpha.option_set_generation, 3);
    assert_eq!(alpha.options.len(), 6);
    assert_eq!(alpha.observed_option_count, 6);
    assert_eq!(alpha.coverage.len(), 2);
    assert_eq!(alpha.options_status, RestFieldStatus::Available);
    assert_eq!(
        alpha.coverage_status,
        RestFieldStatus::Available,
        "the stored lists are the producer's own complete copy"
    );
    assert_eq!(alpha.references.len(), 4);
}

#[test]
fn documented_healing_keeps_its_contributors_separate() {
    let (_manifest, catalog) = fixture();
    let heal = catalog.option("rest.alpha", "option.heal").expect("heal");
    let healing = heal.effects[0].healing.as_ref().expect("healing");
    assert_eq!(healing.base_percent.value(), Some(&30));
    assert_eq!(healing.modifiers.len(), 1);
    assert_eq!(healing.modifiers[0].kind, RestHealModifierKind::RelicBonus);
    assert_eq!(healing.modifiers[0].percent.value(), Some(&5));
    assert_eq!(healing.rounding, RestRoundingMode::Down);
    assert_eq!(
        healing.resolved_percent.value(),
        Some(&30),
        "the resolved total stays the source's value, not a folded one"
    );
    assert_eq!(heal.limits[0].remaining.value(), Some(&1));
}

#[test]
fn documented_upgrade_comparison_carries_both_sides() {
    let (_manifest, catalog) = fixture();
    let smith = catalog.option("rest.alpha", "option.smith").expect("smith");
    let comparison = smith.comparison.as_ref().expect("comparison");
    assert_eq!(comparison.option_id, "option.smith");
    assert!(comparison.complete);
    assert_eq!(comparison.changes[0].before.value(), Some(&"6".to_owned()));
    assert_eq!(comparison.changes[0].after.value(), Some(&"9".to_owned()));
    let upgrade = comparison.upgrade.value().expect("upgrade preview");
    assert_eq!(upgrade.candidate.id, "card.strike");
    assert_eq!(
        smith.selection.candidates[0].reference.id, "card.strike",
        "the selection domain resolves the same candidate"
    );
}

#[test]
fn a_coop_player_selector_resolves_a_player_reference() {
    let (_manifest, catalog) = fixture();
    let lift = catalog.option("rest.beta", "option.lift").expect("lift");
    assert_eq!(lift.selection.domain, RestSelectionDomain::CoopPlayers);
    assert!(lift.selection.required);
    let candidate = &lift.selection.candidates[0];
    assert_eq!(candidate.reference.kind, RestReferenceKind::Player);
    assert_eq!(candidate.reference.id, support::COOP_PLAYER_ID);
}

#[test]
fn site_list_pages_are_bounded_and_deterministic() {
    let (_manifest, catalog) = fixture();
    let mut reader = catalog.reader(RestVisibilityScope::Public);
    let first = reader
        .list(&RestSiteListQuery {
            locale: "en-US".to_owned(),
            scope: RestVisibilityScope::Public,
            limit: 1,
            continuation: None,
        })
        .expect("first page");
    assert_eq!(
        first.total, 3,
        "the hidden site is never counted as visible"
    );
    assert_eq!(first.entries.len(), 1);
    assert_eq!(first.entries[0].reference.site_id, "rest.alpha");
    assert_eq!(first.entries[0].option_count, 4);
    assert_eq!(first.entries[0].options_status, RestFieldStatus::Withheld);
    assert!(!first.complete);

    let second = reader
        .list(&RestSiteListQuery {
            locale: "en-US".to_owned(),
            scope: RestVisibilityScope::Public,
            limit: 1,
            continuation: first.continuation,
        })
        .expect("second page");
    assert_eq!(second.entries[0].reference.site_id, "rest.beta");
    assert!(!second.complete);

    let third = reader
        .list(&RestSiteListQuery {
            locale: "en-US".to_owned(),
            scope: RestVisibilityScope::Public,
            limit: 1,
            continuation: second.continuation,
        })
        .expect("third page");
    assert_eq!(third.entries[0].reference.site_id, "rest.delta");
    assert_eq!(
        third.entries[0].options_status,
        RestFieldStatus::Denied,
        "a visible site whose option set is withheld is denied, not observed as empty"
    );
    assert!(third.complete);
    assert!(third.continuation.is_none());
}

#[test]
fn option_lists_scope_to_one_site_and_withhold_restricted_options() {
    let (_manifest, catalog) = fixture();
    let alpha = site_reference(&catalog, "rest.alpha");
    let mut reader = catalog.reader(RestVisibilityScope::Public);
    let public = reader
        .list_options(&RestOptionListQuery {
            site: alpha.clone(),
            scope: RestVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("public options");
    assert_eq!(public.total, 4);
    assert_eq!(public.options_status, RestFieldStatus::Withheld);
    assert!(public.complete);
    assert!(
        public
            .entries
            .iter()
            .all(|entry| entry.reference.option_id != "option.owner")
    );

    let owner = reader
        .list_options(&RestOptionListQuery {
            site: alpha,
            scope: RestVisibilityScope::Owner,
            limit: 8,
            continuation: None,
        })
        .expect("owner options");
    assert_eq!(owner.total, 5);
}

#[test]
fn exact_lookups_enforce_scope_and_identity() {
    let (_manifest, catalog) = fixture();
    let reader = catalog.reader(RestVisibilityScope::Public);
    let alpha = site_reference(&catalog, "rest.alpha");
    assert_eq!(
        reader
            .get(&alpha, RestVisibilityScope::Public)
            .expect("alpha")
            .reference
            .site_id,
        "rest.alpha"
    );
    let gamma = site_reference(&catalog, "rest.gamma");
    assert_eq!(
        reader.get(&gamma, RestVisibilityScope::Public),
        Err(RestSiteError::ExcludedByScope)
    );
    assert_eq!(
        reader.get(&gamma, RestVisibilityScope::Owner),
        Err(RestSiteError::ExcludedByScope),
        "a hidden site stays hidden even in an explicit owner scope"
    );

    let owner_option = catalog
        .option("rest.alpha", "option.owner")
        .expect("owner option")
        .reference
        .clone();
    assert_eq!(
        reader.get_option(&owner_option, RestVisibilityScope::Public),
        Err(RestSiteError::ExcludedByScope),
        "an owner-only option is withheld from a public lookup"
    );
    assert!(
        reader
            .get_option(&owner_option, RestVisibilityScope::Owner)
            .is_ok(),
        "an explicit owner scope resolves an owner-only option"
    );
}

#[test]
fn disabled_options_publish_their_refusal_state() {
    let (_manifest, catalog) = fixture();
    let mend = catalog.option("rest.alpha", "option.mend").expect("mend");
    assert_eq!(mend.availability.state, RestOptionState::Disabled);
    assert!(mend.availability.reason.is_available());
}

#[test]
fn a_reference_from_another_catalog_or_identity_is_refused() {
    let (_manifest, catalog) = fixture();
    let other_manifest = manifest_with_extra(&[("currency", "currency.gems")]);
    let other = produce(
        &other_manifest,
        snapshot(&other_manifest, support::fixture_definitions()),
    )
    .expect("other catalog");
    assert_eq!(
        other.reader(RestVisibilityScope::Public).get(
            &site_reference(&catalog, "rest.alpha"),
            RestVisibilityScope::Public
        ),
        Err(RestSiteError::StaleReference)
    );
    let missing = RestSiteDefinitionReference {
        catalog: catalog.binding.clone(),
        site_id: "rest.missing".to_owned(),
    };
    assert_eq!(
        catalog
            .reader(RestVisibilityScope::Public)
            .get(&missing, RestVisibilityScope::Public),
        Err(RestSiteError::NotFound)
    );
}

#[test]
fn locale_and_page_size_fences_are_enforced() {
    let (_manifest, catalog) = fixture();
    let mut reader = catalog.reader(RestVisibilityScope::Public);
    assert_eq!(
        reader.list(&RestSiteListQuery {
            locale: "fr-FR".to_owned(),
            scope: RestVisibilityScope::Public,
            limit: 8,
            continuation: None,
        }),
        Err(RestSiteError::LocaleMismatch)
    );
    for limit in [0, sts2_game_mod::REST_MAX_PAGE_ITEMS + 1] {
        assert_eq!(
            reader.list(&RestSiteListQuery {
                locale: "en-US".to_owned(),
                scope: RestVisibilityScope::Public,
                limit,
                continuation: None,
            }),
            Err(RestSiteError::InvalidPageSize)
        );
    }
}
