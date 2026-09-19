// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    RestFieldStatus, RestOptionReference, RestSiteError, RestSiteListQuery, RestVisibilityScope,
};
use support::fixture;

fn option_ids(definition: &sts2_game_mod::RestSiteDefinition) -> Vec<String> {
    definition.options.keys().cloned().collect()
}

#[test]
fn public_scope_withholds_hidden_and_owner_only_records() {
    let (_manifest, catalog) = fixture();
    let reader = catalog.reader(RestVisibilityScope::Public);
    let alpha = catalog
        .definition("rest.alpha")
        .expect("alpha")
        .reference
        .clone();
    let projected = reader
        .get(&alpha, RestVisibilityScope::Public)
        .expect("alpha");
    assert_eq!(
        option_ids(&projected),
        vec![
            "option.heal".to_owned(),
            "option.mend".to_owned(),
            "option.smith".to_owned(),
            "option.toke".to_owned(),
        ]
    );
    assert_eq!(projected.options_status, RestFieldStatus::Withheld);
    assert_eq!(
        projected.coverage.len(),
        1,
        "a coverage record for a hidden option is withheld with it"
    );
    assert_eq!(projected.coverage[0].option_id, "option.toke");
    assert_eq!(projected.coverage_status, RestFieldStatus::Withheld);
    assert_eq!(
        projected.observed_option_count, 6,
        "the host-reported identity count is not filtered by scope"
    );
}

#[test]
fn owner_scope_shows_owner_only_records_but_never_hidden_ones() {
    let (_manifest, catalog) = fixture();
    let alpha = catalog
        .definition("rest.alpha")
        .expect("alpha")
        .reference
        .clone();
    let projected = catalog
        .reader(RestVisibilityScope::Owner)
        .get(&alpha, RestVisibilityScope::Owner)
        .expect("alpha");
    assert_eq!(projected.options.len(), 5);
    assert!(projected.options.contains_key("option.owner"));
    assert!(
        !projected.options.contains_key("option.private"),
        "an owner scope still never reveals a hidden option"
    );
    assert_eq!(projected.options_status, RestFieldStatus::Withheld);
    assert_eq!(projected.coverage_status, RestFieldStatus::Withheld);
}

#[test]
fn a_fully_withheld_collection_reports_denied_rather_than_empty() {
    let (_manifest, catalog) = fixture();
    let delta = catalog
        .definition("rest.delta")
        .expect("delta")
        .reference
        .clone();
    let projected = catalog
        .reader(RestVisibilityScope::Owner)
        .get(&delta, RestVisibilityScope::Owner)
        .expect("delta");
    assert!(projected.options.is_empty());
    assert_eq!(projected.options_status, RestFieldStatus::Denied);
    assert_ne!(projected.options_status, RestFieldStatus::Available);
    assert!(projected.coverage.is_empty());
    assert_eq!(projected.coverage_status, RestFieldStatus::Denied);
}

#[test]
fn a_reference_scope_promotes_no_hidden_record() {
    let (_manifest, catalog) = fixture();
    let alpha = catalog
        .definition("rest.alpha")
        .expect("alpha")
        .reference
        .clone();
    let mut reader = catalog.reader(RestVisibilityScope::Reference);
    let listed = reader
        .list(&RestSiteListQuery {
            locale: "en-US".to_owned(),
            scope: RestVisibilityScope::Reference,
            limit: 8,
            continuation: None,
        })
        .expect("reference list");
    assert_eq!(listed.total, 3);
    let projected = reader
        .get(&alpha, RestVisibilityScope::Reference)
        .expect("alpha");
    assert_eq!(projected.options.len(), 4);
}

#[test]
fn withheld_text_stays_an_explicit_non_value_in_every_projection() {
    let (_manifest, catalog) = fixture();
    let delta = catalog
        .definition("rest.delta")
        .expect("delta")
        .reference
        .clone();
    let projected = catalog
        .reader(RestVisibilityScope::Owner)
        .get(&delta, RestVisibilityScope::Owner)
        .expect("delta");
    assert_eq!(projected.label.value(), None);
    assert_eq!(projected.label.status(), RestFieldStatus::Withheld);
    assert_eq!(projected.description.value(), None);
    let private = catalog.definition("rest.alpha").expect("alpha");
    let private = private.options.get("option.private").expect("private");
    assert_eq!(private.label.status(), RestFieldStatus::Withheld);
    assert_ne!(private.label.status(), RestFieldStatus::Available);
}

#[test]
fn a_withheld_option_is_never_resolved_by_an_exact_lookup() {
    let (_manifest, catalog) = fixture();
    let private = catalog
        .option("rest.alpha", "option.private")
        .expect("private")
        .reference
        .clone();
    assert_eq!(
        catalog
            .reader(RestVisibilityScope::Owner)
            .get_option(&private, RestVisibilityScope::Public),
        Err(RestSiteError::ExcludedByScope)
    );
    let missing = RestOptionReference {
        catalog: catalog.binding.clone(),
        site_id: "rest.alpha".to_owned(),
        option_id: "option.absent".to_owned(),
    };
    assert_eq!(
        catalog
            .reader(RestVisibilityScope::Public)
            .get_option(&missing, RestVisibilityScope::Public),
        Err(RestSiteError::NotFound)
    );
}
