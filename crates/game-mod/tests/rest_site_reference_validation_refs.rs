// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    REST_MAX_DEFINITION_BYTES, REST_MAX_OPTIONS, REST_MAX_REFERENCES, RestOptionInput,
    RestOptionKind, RestReferenceKind, RestSiteCatalog, RestSiteDefinitionInput, RestSiteError,
};
use support::{definition, gold_cost, manifest, option, produce, reference, snapshot, text};

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

#[test]
fn a_record_more_visible_than_its_target_is_refused() {
    let leaking = RestSiteDefinitionInput {
        references: vec![reference(RestReferenceKind::Site, "rest.gamma")],
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_with_sites(
            &["rest.alpha", "rest.gamma"],
            vec![leaking, support::gamma()]
        ),
        Err(RestSiteError::HiddenReferenceLeak {
            site_id: "rest.alpha".to_owned(),
            reference_kind: RestReferenceKind::Site,
        })
    );
}

#[test]
fn a_reference_absent_from_the_snapshot_or_manifest_is_refused() {
    let dangling_site = RestSiteDefinitionInput {
        references: vec![reference(RestReferenceKind::Site, "rest.absent")],
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_one(dangling_site),
        Err(RestSiteError::DanglingReference {
            site_id: "rest.alpha".to_owned(),
            reference_kind: RestReferenceKind::Site,
            id: "rest.absent".to_owned(),
        })
    );

    let mut dangling_option = option("option.heal", RestOptionKind::Heal);
    dangling_option.references = vec![reference(RestReferenceKind::Option, "option.absent")];
    assert_eq!(
        produce_one(one_option(dangling_option)),
        Err(RestSiteError::DanglingReference {
            site_id: "rest.alpha".to_owned(),
            reference_kind: RestReferenceKind::Option,
            id: "option.absent".to_owned(),
        })
    );

    let mut unknown_card = option("option.heal", RestOptionKind::Heal);
    unknown_card.definition = reference(RestReferenceKind::Card, "card.absent");
    assert_eq!(
        produce_one(one_option(unknown_card)),
        Err(RestSiteError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card.absent".to_owned(),
        })
    );
}

#[test]
fn local_option_references_resolve_inside_their_own_site() {
    let mut referencing = option("option.heal", RestOptionKind::Heal);
    referencing.references = vec![reference(RestReferenceKind::Option, "option.smith")];
    let definition = RestSiteDefinitionInput {
        observed_options: vec!["option.heal".to_owned(), "option.smith".to_owned()],
        options: vec![referencing, option("option.smith", RestOptionKind::Smith)],
        ..definition("rest.alpha", 3)
    };
    assert!(produce_one(definition).is_ok());
}

#[test]
fn an_oversized_definition_is_refused() {
    let mut healer = option("option.heal", RestOptionKind::Heal);
    healer.costs = (0..9)
        .map(|index| {
            let mut cost = gold_cost(&format!("cost.{index}"), 1);
            cost.label = text(&"x".repeat(16 * 1024));
            cost
        })
        .collect();
    assert!(
        matches!(
            produce_one(one_option(healer)),
            Err(RestSiteError::DefinitionTooLarge { .. })
        ),
        "a definition past {REST_MAX_DEFINITION_BYTES} bytes is refused"
    );
}

#[test]
fn local_input_bounds_and_identities_are_enforced() {
    let many = RestSiteDefinitionInput {
        options: (0..=REST_MAX_OPTIONS)
            .map(|index| option(&format!("option.{index}"), RestOptionKind::Heal))
            .collect(),
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_one(many),
        Err(RestSiteError::InvalidInput("options"))
    );

    let crowded = RestSiteDefinitionInput {
        references: (0..=REST_MAX_REFERENCES)
            .map(|index| reference(RestReferenceKind::Card, &format!("card.{index}")))
            .collect(),
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_one(crowded),
        Err(RestSiteError::InvalidInput("references"))
    );

    let malformed = RestSiteDefinitionInput {
        site_id: "rest alpha".to_owned(),
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_one(malformed),
        Err(RestSiteError::InvalidInput("site_id"))
    );

    let blank = RestSiteDefinitionInput {
        description: text(""),
        ..definition("rest.alpha", 3)
    };
    assert_eq!(
        produce_one(blank),
        Err(RestSiteError::InvalidInput("description"))
    );
}
