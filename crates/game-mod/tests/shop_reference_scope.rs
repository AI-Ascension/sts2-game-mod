// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used, dead_code)]

#[path = "support/shop_reference.rs"]
mod support;

use sts2_game_mod::{
    ShopCatalog, ShopCatalogError, ShopDefinitionInput, ShopEntryListQuery, ShopReferenceKind,
    ShopSaleState, ShopSemanticReference, ShopVisibility, ShopVisibilityScope,
};
use support::{
    card_entry, definition, fixture_definitions, fixture_manifest, manifest, produce, reference,
    restock_rule, snapshot, text, withheld_text,
};

fn local_catalog(
    kind: ShopReferenceKind,
    source: ShopVisibility,
    target: ShopVisibility,
) -> Result<ShopCatalog, ShopCatalogError> {
    let mut shop = definition("shop.alpha", 3);
    let mut entry = card_entry("entry.source");
    entry.visibility = source;
    if matches!(source, ShopVisibility::Hidden | ShopVisibility::Unknown) {
        entry.label = withheld_text();
    }
    let mut edge = reference(kind.clone(), "target.private");
    edge.label = text("restricted-sentinel");
    entry.references = vec![edge];
    shop.entries.push(entry);
    if kind == ShopReferenceKind::Entry {
        let mut target_entry = card_entry("target.private");
        target_entry.visibility = target;
        if matches!(target, ShopVisibility::Hidden | ShopVisibility::Unknown) {
            target_entry.label = withheld_text();
        }
        shop.entries.push(target_entry);
    } else {
        let mut rule = restock_rule("target.private", 4, Vec::new());
        rule.visibility = target;
        if matches!(target, ShopVisibility::Hidden | ShopVisibility::Unknown) {
            rule.label = withheld_text();
        }
        shop.restock.push(rule);
    }
    let manifest = manifest(&["shop.alpha"]);
    produce(&manifest, snapshot(&manifest, vec![shop]))
}

#[test]
fn every_local_reference_edge_obeys_the_visibility_lattice() {
    for kind in [ShopReferenceKind::Entry, ShopReferenceKind::RestockRule] {
        for (source, target) in [
            (ShopVisibility::Visible, ShopVisibility::OwnerOnly),
            (ShopVisibility::Visible, ShopVisibility::Hidden),
            (ShopVisibility::OwnerOnly, ShopVisibility::Hidden),
            (ShopVisibility::Visible, ShopVisibility::Unknown),
            (ShopVisibility::OwnerOnly, ShopVisibility::Unknown),
        ] {
            let error = local_catalog(kind.clone(), source, target).expect_err("restricted target");
            assert_eq!(
                error,
                ShopCatalogError::HiddenReferenceLeak {
                    shop_id: "shop.alpha".to_owned(),
                    reference_kind: kind.clone(),
                }
            );
            let diagnostic = format!("{error:?} {error}");
            assert!(!diagnostic.contains("target.private"));
            assert!(!diagnostic.contains("restricted-sentinel"));
        }
        for (source, target) in [
            (ShopVisibility::Visible, ShopVisibility::Visible),
            (ShopVisibility::OwnerOnly, ShopVisibility::OwnerOnly),
            (ShopVisibility::Hidden, ShopVisibility::Hidden),
            (ShopVisibility::OwnerOnly, ShopVisibility::Visible),
            (ShopVisibility::Hidden, ShopVisibility::OwnerOnly),
            (ShopVisibility::Hidden, ShopVisibility::Visible),
            (ShopVisibility::Unknown, ShopVisibility::Unknown),
            (ShopVisibility::Unknown, ShopVisibility::Hidden),
            (ShopVisibility::Hidden, ShopVisibility::Unknown),
        ] {
            assert!(local_catalog(kind.clone(), source, target).is_ok());
        }
    }
}

#[test]
fn top_level_references_cannot_retain_projected_out_local_targets() {
    for kind in [ShopReferenceKind::Entry, ShopReferenceKind::RestockRule] {
        let mut definitions = fixture_definitions();
        let id = if kind == ShopReferenceKind::Entry {
            "entry.private"
        } else {
            definitions[0].restock[0].visibility = ShopVisibility::Hidden;
            definitions[0].restock[0].label = withheld_text();
            "rule.restock"
        };
        definitions[0].references.push(reference(kind.clone(), id));
        let manifest = fixture_manifest();
        assert_eq!(
            produce(&manifest, snapshot(&manifest, definitions)),
            Err(ShopCatalogError::HiddenReferenceLeak {
                shop_id: "shop.alpha".to_owned(),
                reference_kind: kind,
            })
        );
    }
}

#[test]
fn allowed_owner_edges_remain_unavailable_to_public_reads() {
    for kind in [ShopReferenceKind::Entry, ShopReferenceKind::RestockRule] {
        let catalog = local_catalog(kind, ShopVisibility::OwnerOnly, ShopVisibility::OwnerOnly)
            .expect("owner-to-owner reference");
        let shop = catalog.definition("shop.alpha").expect("shop");
        let source = shop
            .entries
            .get("entry.source")
            .expect("source")
            .reference
            .clone();
        let mut reader = catalog.reader();
        let public = reader
            .get(&shop.reference, ShopVisibilityScope::Public)
            .expect("public");
        let page = reader
            .list_entries(&ShopEntryListQuery {
                shop: shop.reference.clone(),
                scope: ShopVisibilityScope::Public,
                limit: 16,
                continuation: None,
            })
            .expect("public page");
        let exact = reader.get_entry(&source, ShopVisibilityScope::Public);
        assert_eq!(exact, Err(ShopCatalogError::ExcludedByScope));
        let public_outputs = format!("{public:?} {page:?} {exact:?}");
        assert!(!public_outputs.contains("target.private"));
        assert!(!public_outputs.contains("restricted-sentinel"));
        let owner = reader
            .get_entry(&source, ShopVisibilityScope::Owner)
            .expect("owner");
        assert_eq!(owner.references[0].id, "target.private");
        assert_eq!(
            owner.references[0].label.value(),
            Some("restricted-sentinel")
        );
    }
}

fn nested_reference(shop: &mut ShopDefinitionInput, site: usize, edge: ShopSemanticReference) {
    match site {
        0 => shop.entries[7].restrictions[0].requirements[0].references = vec![edge],
        1 => shop.services[0].eligibility[0].references = vec![edge],
        2 => shop.services[0].limits[0].references = vec![edge],
        3 => {
            let ShopSaleState::Stacked { tiers, .. } = &mut shop.entries[5].sale else {
                unreachable!("fixture has a stacked sale");
            };
            tiers[0].source = edge;
        }
        _ => unreachable!("four nested reference sites"),
    }
}

#[test]
fn all_nested_reference_sites_reject_hidden_shops_and_missing_local_targets() {
    for site in 0..4 {
        for kind in [ShopReferenceKind::Shop, ShopReferenceKind::Entry] {
            let id = if kind == ShopReferenceKind::Shop {
                "shop.gamma"
            } else {
                "entry.missing"
            };
            let mut definitions = fixture_definitions();
            let mut edge = reference(kind.clone(), id);
            edge.label = text("nested-restricted-sentinel");
            nested_reference(&mut definitions[0], site, edge);
            let manifest = fixture_manifest();
            let error = produce(&manifest, snapshot(&manifest, definitions))
                .expect_err("nested references must be validated");
            if kind == ShopReferenceKind::Shop {
                assert_eq!(
                    error,
                    ShopCatalogError::HiddenReferenceLeak {
                        shop_id: "shop.alpha".to_owned(),
                        reference_kind: kind,
                    }
                );
                assert!(!format!("{error:?} {error}").contains("shop.gamma"));
            } else {
                assert_eq!(
                    error,
                    ShopCatalogError::DanglingReference {
                        shop_id: "shop.alpha".to_owned(),
                        reference_kind: kind,
                        id: id.to_owned(),
                    }
                );
            }
            assert!(!format!("{error:?} {error}").contains("nested-restricted-sentinel"));
        }
    }
}

#[test]
fn nested_references_preserve_allowed_targets_and_validate_reference_metadata() {
    for site in 0..4 {
        let manifest = fixture_manifest();
        let mut definitions = fixture_definitions();
        nested_reference(
            &mut definitions[0],
            site,
            reference(ShopReferenceKind::Entry, "entry.card"),
        );
        assert!(produce(&manifest, snapshot(&manifest, definitions)).is_ok());
        let mut definitions = fixture_definitions();
        nested_reference(
            &mut definitions[0],
            site,
            reference(ShopReferenceKind::Entry, ""),
        );
        assert!(produce(&manifest, snapshot(&manifest, definitions)).is_err());
    }
}

#[test]
fn discount_tier_sources_must_also_exist_in_the_manifest() {
    let manifest = fixture_manifest();
    let mut definitions = fixture_definitions();
    nested_reference(
        &mut definitions[0],
        3,
        reference(ShopReferenceKind::Card, "card.missing"),
    );
    assert_eq!(
        produce(&manifest, snapshot(&manifest, definitions)),
        Err(ShopCatalogError::UnknownManifestReference {
            entity_kind: "card".to_owned(),
            namespaced_id: "card.missing".to_owned(),
        })
    );
}
