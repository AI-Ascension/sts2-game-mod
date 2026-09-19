// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    RestOptionListQuery, RestOptionSetReference, RestOptionState, RestSiteError, RestSiteListQuery,
    RestVisibilityScope,
};
use support::{fixture, fixture_definitions, manifest_with_extra, produce, snapshot};

fn site_scope() -> RestSiteListQuery {
    RestSiteListQuery {
        locale: "en-US".to_owned(),
        scope: RestVisibilityScope::Public,
        limit: 1,
        continuation: None,
    }
}

#[test]
fn availability_is_fenced_to_the_option_set_generation_that_produced_it() {
    let (_manifest, catalog) = fixture();
    let reader = catalog.reader(RestVisibilityScope::Public);
    let heal = catalog
        .option("rest.alpha", "option.heal")
        .expect("heal")
        .reference
        .clone();
    let referenced = |generation: u64| RestOptionSetReference {
        option: heal.clone(),
        option_set_generation: generation,
    };
    assert_eq!(
        reader.option_availability(&referenced(3), RestVisibilityScope::Public),
        Ok(RestOptionState::Available)
    );
    for generation in [2, 4] {
        assert_eq!(
            reader.option_availability(&referenced(generation), RestVisibilityScope::Public),
            Err(RestSiteError::StaleOptionSetReference {
                option_id: "option.heal".to_owned(),
                referenced: generation,
                current: 3,
            }),
            "an earlier or later option set is refused rather than answered"
        );
    }
}

#[test]
fn an_option_set_reference_resolves_only_inside_its_own_catalog_and_scope() {
    let (_manifest, catalog) = fixture();
    let other_manifest = manifest_with_extra(&[("currency", "currency.gems")]);
    let other = produce(
        &other_manifest,
        snapshot(&other_manifest, fixture_definitions()),
    )
    .expect("other catalog");
    let heal = catalog
        .option("rest.alpha", "option.heal")
        .expect("heal")
        .reference
        .clone();
    let reference = RestOptionSetReference {
        option: heal.clone(),
        option_set_generation: 3,
    };
    assert_eq!(
        other
            .reader(RestVisibilityScope::Public)
            .option_availability(&reference, RestVisibilityScope::Public),
        Err(RestSiteError::StaleReference)
    );
    assert_eq!(
        catalog
            .reader(RestVisibilityScope::Public)
            .option_availability(&reference, RestVisibilityScope::Owner),
        Ok(RestOptionState::Available),
        "an owner-scope query still resolves a public option"
    );
    let hidden = catalog
        .option("rest.alpha", "option.private")
        .expect("private")
        .reference
        .clone();
    assert_eq!(
        catalog
            .reader(RestVisibilityScope::Owner)
            .option_availability(
                &RestOptionSetReference {
                    option: hidden,
                    option_set_generation: 3,
                },
                RestVisibilityScope::Owner,
            ),
        Err(RestSiteError::ExcludedByScope)
    );
}

#[test]
fn a_site_continuation_is_single_use() {
    let (_manifest, catalog) = fixture();
    let mut reader = catalog.reader(RestVisibilityScope::Public);
    let first = reader.list(&site_scope()).expect("first page");
    let continuation = first.continuation.expect("continuation");
    let mut second = site_scope();
    second.continuation = Some(continuation.clone());
    assert_eq!(reader.list(&second).expect("second page").entries.len(), 1);
    let mut reused = site_scope();
    reused.continuation = Some(continuation);
    assert_eq!(
        reader.list(&reused),
        Err(RestSiteError::InvalidContinuation),
        "a reused clone is rejected"
    );
}

#[test]
fn a_continuation_is_bound_to_its_reader_query_and_site() {
    let (_manifest, catalog) = fixture();
    let mut reader = catalog.reader(RestVisibilityScope::Public);
    let continuation = reader
        .list(&site_scope())
        .expect("first page")
        .continuation
        .expect("continuation");

    let mut widened = site_scope();
    widened.limit = 2;
    widened.continuation = Some(continuation.clone());
    assert_eq!(
        reader.list(&widened),
        Err(RestSiteError::InvalidContinuation),
        "a continuation bound to another page size is refused"
    );

    let mut rescoped = site_scope();
    rescoped.scope = RestVisibilityScope::Owner;
    rescoped.continuation = Some(continuation.clone());
    assert_eq!(
        reader.list(&rescoped),
        Err(RestSiteError::InvalidContinuation),
        "a continuation bound to another visibility scope is refused"
    );

    let mut other_reader = catalog.reader(RestVisibilityScope::Public);
    let mut reused = site_scope();
    reused.continuation = Some(continuation);
    assert_eq!(
        other_reader.list(&reused),
        Err(RestSiteError::InvalidContinuation),
        "another reader's continuation is refused"
    );
}

#[test]
fn an_option_continuation_is_bound_to_its_own_site() {
    let (_manifest, catalog) = fixture();
    let alpha = catalog
        .definition("rest.alpha")
        .expect("alpha")
        .reference
        .clone();
    let beta = catalog
        .definition("rest.beta")
        .expect("beta")
        .reference
        .clone();
    let mut reader = catalog.reader(RestVisibilityScope::Public);
    let first = reader
        .list_options(&RestOptionListQuery {
            site: alpha,
            scope: RestVisibilityScope::Public,
            limit: 1,
            continuation: None,
        })
        .expect("alpha options");
    assert_eq!(first.total, 4);
    assert!(!first.complete);
    let continuation = first.continuation.expect("continuation");
    assert_eq!(
        reader.list_options(&RestOptionListQuery {
            site: beta,
            scope: RestVisibilityScope::Public,
            limit: 1,
            continuation: Some(continuation),
        }),
        Err(RestSiteError::InvalidContinuation),
        "a continuation bound to another site is refused"
    );
}
