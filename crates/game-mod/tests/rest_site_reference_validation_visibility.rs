// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/rest_site_reference.rs"]
mod support;

use sts2_game_mod::{
    RestOptionInput, RestOptionKind, RestReferenceKind, RestSemanticReference, RestSiteCatalog,
    RestSiteDefinitionInput, RestSiteError, RestVisibility, RestVisibilityScope,
};
use support::{definition, manifest, option, produce, reference, snapshot, text};

#[derive(Clone, Copy)]
enum Carrier {
    Site,
    Option,
    Definition,
    Availability,
}

const CARRIERS: [Carrier; 4] = [
    Carrier::Site,
    Carrier::Option,
    Carrier::Definition,
    Carrier::Availability,
];

fn attach(site: &mut RestSiteDefinitionInput, carrier: Carrier, edge: RestSemanticReference) {
    match carrier {
        Carrier::Site => site.references.push(edge),
        Carrier::Option => site.options[0].references.push(edge),
        Carrier::Definition => site.options[0].definition = edge,
        Carrier::Availability => site.options[0].availability.references.push(edge),
    }
}

fn site_with_options(options: Vec<RestOptionInput>) -> RestSiteDefinitionInput {
    RestSiteDefinitionInput {
        observed_options: options
            .iter()
            .map(|option| option.option_id.clone())
            .collect(),
        options,
        ..definition("rest.alpha", 3)
    }
}

fn produce_sites(sites: Vec<RestSiteDefinitionInput>) -> Result<RestSiteCatalog, RestSiteError> {
    let ids: Vec<&str> = sites.iter().map(|site| site.site_id.as_str()).collect();
    let manifest = manifest(&ids);
    produce(&manifest, snapshot(&manifest, sites))
}

#[test]
fn all_local_option_carriers_enforce_the_visibility_order() {
    let visibilities = [
        (RestVisibility::Visible, 2),
        (RestVisibility::OwnerOnly, 1),
        (RestVisibility::Hidden, 0),
        (RestVisibility::Unknown, 0),
    ];
    for carrier in CARRIERS {
        for (owner_visibility, owner_rank) in visibilities {
            for (target_visibility, target_rank) in visibilities {
                let mut owner = option("option.owner", RestOptionKind::Heal);
                owner.visibility = owner_visibility;
                let mut target = option("option.secret", RestOptionKind::Smith);
                target.visibility = target_visibility;
                let mut site = site_with_options(vec![owner, target]);
                site.visibility = owner_visibility;
                let mut edge = reference(RestReferenceKind::Option, "option.secret");
                edge.label = text("secret-reference-label");
                attach(&mut site, carrier, edge);
                let result = produce_sites(vec![site]);
                if owner_rank > target_rank {
                    let error = result.expect_err("more-visible reference must be refused");
                    assert_eq!(
                        error,
                        RestSiteError::HiddenReferenceLeak {
                            site_id: "rest.alpha".to_owned(),
                            reference_kind: RestReferenceKind::Option,
                        }
                    );
                    for diagnostic in [format!("{error:?}"), error.to_string()] {
                        assert!(!diagnostic.contains("option.secret"));
                        assert!(!diagnostic.contains("secret-reference-label"));
                    }
                } else {
                    result.expect("same or less-visible reference is valid");
                }
            }
        }
    }
}

#[test]
fn all_site_carriers_include_reserved_content_aliases() {
    for kind in [
        RestReferenceKind::Site,
        RestReferenceKind::Content {
            entity_kind: "rest-site".to_owned(),
        },
    ] {
        for carrier in CARRIERS {
            let mut site = site_with_options(vec![option("option.owner", RestOptionKind::Heal)]);
            attach(&mut site, carrier, reference(kind.clone(), "rest.secret"));
            let mut hidden = support::gamma();
            hidden.site_id = "rest.secret".to_owned();
            assert_eq!(
                produce_sites(vec![site, hidden]),
                Err(RestSiteError::HiddenReferenceLeak {
                    site_id: "rest.alpha".to_owned(),
                    reference_kind: kind.clone(),
                })
            );
        }
    }
}

#[test]
fn reserved_alias_requires_a_same_snapshot_site() {
    let kind = RestReferenceKind::Content {
        entity_kind: "rest-site".to_owned(),
    };
    let mut site = definition("rest.alpha", 3);
    site.references
        .push(reference(kind.clone(), "rest.missing"));
    let manifest = manifest(&["rest.alpha", "rest.missing"]);
    let mut input = snapshot(&manifest, vec![site]);
    input.family.definition_count = 2;
    assert_eq!(
        produce(&manifest, input),
        Err(RestSiteError::DanglingReference {
            site_id: "rest.alpha".to_owned(),
            reference_kind: kind,
            id: "rest.missing".to_owned(),
        })
    );
}

#[test]
fn local_option_references_never_resolve_in_another_site() {
    for carrier in CARRIERS {
        let mut site = site_with_options(vec![option("option.owner", RestOptionKind::Heal)]);
        attach(
            &mut site,
            carrier,
            reference(RestReferenceKind::Option, "option.other"),
        );
        let mut other = site_with_options(vec![option("option.other", RestOptionKind::Heal)]);
        other.site_id = "rest.beta".to_owned();
        assert_eq!(
            produce_sites(vec![site, other]),
            Err(RestSiteError::DanglingReference {
                site_id: "rest.alpha".to_owned(),
                reference_kind: RestReferenceKind::Option,
                id: "option.other".to_owned(),
            })
        );
    }
}

#[test]
fn availability_checks_manifest_membership_and_reference_bounds() {
    let mut owner = option("option.owner", RestOptionKind::Heal);
    owner
        .availability
        .references
        .push(reference(RestReferenceKind::Card, "card.absent"));
    assert_eq!(
        produce_sites(vec![site_with_options(vec![owner.clone()])]),
        Err(RestSiteError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card.absent".to_owned(),
        })
    );
    owner.availability.references = vec![
        reference(RestReferenceKind::Card, "card.strike");
        sts2_game_mod::REST_MAX_REFERENCES + 1
    ];
    assert_eq!(
        produce_sites(vec![site_with_options(vec![owner])]),
        Err(RestSiteError::InvalidInput("references"))
    );
}

#[test]
fn accepted_references_survive_public_projection_unchanged() {
    let local = reference(RestReferenceKind::Option, "option.target");
    let alias = reference(
        RestReferenceKind::Content {
            entity_kind: "rest-site".to_owned(),
        },
        "rest.beta",
    );
    let external = reference(
        RestReferenceKind::Content {
            entity_kind: "card".to_owned(),
        },
        "card.strike",
    );
    let mut owner = option("option.owner", RestOptionKind::Heal);
    owner.definition = local.clone();
    owner.references.push(local.clone());
    owner.availability.references.push(external.clone());
    let mut site = site_with_options(vec![owner, option("option.target", RestOptionKind::Smith)]);
    site.references.push(alias.clone());
    let catalog = produce_sites(vec![site, definition("rest.beta", 3)]).expect("catalog");
    let identity = catalog
        .definition("rest.alpha")
        .expect("alpha")
        .reference
        .clone();
    let projected = catalog
        .reader(RestVisibilityScope::Public)
        .get(&identity, RestVisibilityScope::Public)
        .expect("public projection");
    assert_eq!(projected.references, vec![alias]);
    let owner = projected.options.get("option.owner").expect("owner");
    assert_eq!(owner.definition, local);
    assert_eq!(owner.references, vec![local]);
    assert_eq!(owner.availability.references, vec![external]);
}
