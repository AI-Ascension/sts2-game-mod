// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    FailingRestSiteSource, FixtureRestFailure, FixtureRestSiteSource, RestFamilyState, RestField,
    RestOptionKind, RestOptionListQuery, RestOptionSetReference, RestOptionState,
    RestSiteDefinitionInput, RestSiteError, RestSiteListQuery, RestVisibilityScope,
};
use support::{
    action_reference, definition, fixture_definitions, fixture_manifest, item_definitions,
    manifest, manifest_of, manifest_without_sites, option, produce, produce_with, snapshot,
};

#[test]
fn producing_and_reading_never_mutates_or_extra_reads_the_source() {
    let manifest = fixture_manifest();
    let source = FixtureRestSiteSource::new(snapshot(&manifest, fixture_definitions()));
    let catalog = produce_with(&manifest, &source).expect("catalog");
    assert_eq!(source.reads(), 1, "one owned snapshot read");
    assert_eq!(source.snapshot().definitions.len(), 4);

    let alpha = catalog
        .definition("rest.alpha")
        .expect("alpha")
        .reference
        .clone();
    let heal = catalog
        .option("rest.alpha", "option.heal")
        .expect("heal")
        .reference
        .clone();
    let effects_before = catalog
        .option("rest.alpha", "option.heal")
        .expect("heal")
        .effects
        .clone();

    let mut reader = catalog.reader(RestVisibilityScope::Public);
    reader
        .list(&RestSiteListQuery {
            locale: "en-US".to_owned(),
            scope: RestVisibilityScope::Public,
            limit: 8,
            continuation: None,
        })
        .expect("list");
    reader
        .list_options(&RestOptionListQuery {
            site: alpha.clone(),
            scope: RestVisibilityScope::Owner,
            limit: 8,
            continuation: None,
        })
        .expect("options");
    reader
        .get(&alpha, RestVisibilityScope::Public)
        .expect("get");
    reader
        .get_option(&heal, RestVisibilityScope::Public)
        .expect("option");
    let availability = reader
        .option_availability(
            &RestOptionSetReference {
                option: heal.clone(),
                option_set_generation: 3,
            },
            RestVisibilityScope::Public,
        )
        .expect("availability");

    assert_eq!(availability, RestOptionState::Available);
    assert_eq!(
        source.reads(),
        1,
        "no read triggered a hidden second source read"
    );
    assert_eq!(
        catalog
            .option("rest.alpha", "option.heal")
            .expect("heal")
            .effects,
        effects_before,
        "a read reports the option without changing it"
    );
    assert_eq!(
        source.snapshot().definitions[0].options.len(),
        6,
        "the retained source snapshot is untouched"
    );
    assert_eq!(
        catalog
            .definition("rest.alpha")
            .expect("alpha")
            .options
            .len(),
        6,
        "no read removed or added an option"
    );
}

#[test]
fn a_transient_rest_action_cannot_enter_the_static_slice() {
    let manifest = manifest(&["rest.alpha"]);
    let mut heal = option("option.heal", RestOptionKind::Heal);
    heal.action = RestField::available(action_reference());
    let definition = RestSiteDefinitionInput {
        observed_options: vec!["option.heal".to_owned()],
        options: vec![heal],
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce(&manifest, snapshot(&manifest, vec![definition])),
        Err(RestSiteError::InvalidInput("rest_action"))
    );
}

#[test]
fn only_sanitized_source_failures_cross_the_reader_boundary() {
    let manifest = fixture_manifest();
    for (failure, expected) in [
        (
            FixtureRestFailure::NoActiveSource,
            RestSiteError::NoActiveSource,
        ),
        (
            FixtureRestFailure::AccessDenied,
            RestSiteError::SourceAccessDenied,
        ),
        (
            FixtureRestFailure::Malformed,
            RestSiteError::MalformedSource,
        ),
    ] {
        assert_eq!(
            produce_with(&manifest, &FailingRestSiteSource(failure)),
            Err(expected)
        );
    }
}

#[test]
fn an_absent_rest_family_is_refused_before_any_catalog_exists() {
    let manifest = manifest_of(&item_definitions());
    assert_eq!(
        produce(&manifest, snapshot(&manifest, Vec::new())),
        Err(RestSiteError::MissingFamily)
    );
}

#[test]
fn a_family_the_source_cannot_project_reports_a_capability_failure() {
    for (state, expected) in [
        (
            RestFamilyState::Unsupported,
            RestSiteError::UnsupportedFamily,
        ),
        (
            RestFamilyState::Unavailable,
            RestSiteError::UnavailableFamily,
        ),
    ] {
        let manifest = manifest_without_sites();
        let mut snapshot = snapshot(&manifest, Vec::new());
        snapshot.family.state = state;
        let catalog = produce(&manifest, snapshot).expect("empty catalog");
        assert!(catalog.is_empty());
        assert_eq!(catalog.family.state, state);
        let mut reader = catalog.reader(RestVisibilityScope::Public);
        assert_eq!(
            reader.list(&RestSiteListQuery {
                locale: "en-US".to_owned(),
                scope: RestVisibilityScope::Public,
                limit: 8,
                continuation: None,
            }),
            Err(expected)
        );
    }
}

#[test]
fn every_catalog_fence_is_enforced_before_an_option_is_published() {
    let manifest = fixture_manifest();
    let base = || snapshot(&manifest, fixture_definitions());

    let mut locale = base();
    locale.locale = "fr-FR".to_owned();
    assert_eq!(
        produce(&manifest, locale),
        Err(RestSiteError::LocaleMismatch)
    );

    let mut producer = base();
    producer.producer_version = "other-producer".to_owned();
    assert_eq!(
        produce(&manifest, producer),
        Err(RestSiteError::ProducerVersionMismatch)
    );

    let mut family = base();
    family.family.entity_kind = "shop".to_owned();
    assert_eq!(
        produce(&manifest, family),
        Err(RestSiteError::FamilyIdentityMismatch)
    );

    let mut count = base();
    count.family.definition_count = 0;
    assert_eq!(
        produce(&manifest, count),
        Err(RestSiteError::FamilyCountMismatch)
    );

    let mut binding = base();
    binding.manifest.catalog_generation = 999;
    assert_eq!(
        produce(&manifest, binding),
        Err(RestSiteError::ManifestMismatch)
    );
}
