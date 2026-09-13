// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

#[path = "support/glossary.rs"]
mod fixture;

use fixture::*;
use sts2_game_mod::{
    ContentQueryLocale, GlossaryCatalogError, GlossaryCompleteness, GlossaryDefinitionText,
    GlossaryEvidence, GlossaryListQuery, GlossarySearchQuery, GlossaryTermReference,
    GlossaryUnresolvedReason,
};

#[test]
fn search_ranking_and_pagination_are_locale_and_scope_bound() {
    let raw_catalog = catalog();
    assert_eq!(
        raw_catalog.coverage().status,
        sts2_game_mod::GlossaryCoverageStatus::Unverified
    );
    let catalog = raw_catalog
        .with_content_index(&content_index())
        .expect("composed glossary coverage");
    assert_eq!(
        catalog.coverage().status,
        sts2_game_mod::GlossaryCoverageStatus::Partial
    );
    assert_eq!(catalog.coverage().term_count, 5);
    assert_eq!(catalog.coverage().related_reference_count, 3);
    assert_eq!(catalog.coverage().content_reference_count, 2);
    assert_eq!(catalog.coverage().definition_reference_count, 3);
    assert_eq!(catalog.coverage().unresolved_related_count, 1);
    assert_eq!(catalog.coverage().unresolved_content_reference_count, 1);
    assert_eq!(catalog.coverage().unresolved_definition_reference_count, 1);
    assert_eq!(catalog.coverage().unresolved_related_terms.len(), 1);
    assert_eq!(
        catalog.coverage().unresolved_related_terms[0].target_term_id,
        "status:missing"
    );
    assert_eq!(catalog.coverage().unresolved_content_references.len(), 1);
    assert_eq!(catalog.coverage().unresolved_definition_references.len(), 1);
    assert_eq!(
        catalog.coverage().unresolved_definition_references[0].term_id,
        "status:missing"
    );
    assert_eq!(catalog.definition_references().len(), 3);

    let mut reader = catalog.reader();
    let locale = ContentQueryLocale::new("en-US").expect("locale");
    let list = reader
        .list(&GlossaryListQuery {
            locale: locale.clone(),
            scope: public_scope(),
            limit: 8,
            continuation: None,
        })
        .expect("list");
    assert_eq!(list.total, 3);
    assert_eq!(
        list.entries
            .iter()
            .map(|entry| entry.reference.term_id.as_str())
            .collect::<Vec<_>>(),
        vec!["keyword:strength", "status:strength", "status:weak"]
    );
    let first = reader
        .search(&GlossarySearchQuery {
            locale: locale.clone(),
            literal: "strength".to_owned(),
            scope: public_scope(),
            limit: 1,
            continuation: None,
        })
        .expect("first page");
    assert_eq!(first.total, 2);
    assert_eq!(first.entries[0].rank, 0);
    assert_eq!(
        first.entries[0].summary.reference.term_id,
        "keyword:strength"
    );
    let continuation = first.continuation.clone().expect("second result");
    let second = reader
        .search(&GlossarySearchQuery {
            continuation: Some(continuation.clone()),
            ..GlossarySearchQuery {
                locale: locale.clone(),
                literal: "strength".to_owned(),
                scope: public_scope(),
                limit: 1,
                continuation: None,
            }
        })
        .expect("second page");
    assert_eq!(
        second.entries[0].summary.reference.term_id,
        "status:strength"
    );
    assert_eq!(second.completeness, GlossaryCompleteness::Complete);
    assert_eq!(
        reader.search(&GlossarySearchQuery {
            locale,
            literal: "strength".to_owned(),
            scope: public_scope(),
            limit: 1,
            continuation: Some(continuation),
        }),
        Err(GlossaryCatalogError::InvalidContinuation)
    );

    let mut reader = catalog.reader();
    let non_ascii = reader
        .search(&GlossarySearchQuery {
            locale: ContentQueryLocale::new("en-US").expect("locale"),
            literal: "ÉCLAIR".to_owned(),
            scope: public_scope(),
            limit: 8,
            continuation: None,
        })
        .expect("public search excludes reference term");
    assert_eq!(non_ascii.total, 0);
    let non_ascii_reference = reader
        .search(&GlossarySearchQuery {
            locale: ContentQueryLocale::new("en-US").expect("locale"),
            literal: "éclair".to_owned(),
            scope: reference_scope(),
            limit: 8,
            continuation: None,
        })
        .expect("reference search");
    assert_eq!(non_ascii_reference.total, 1);
    assert_eq!(
        non_ascii_reference.entries[0].summary.reference.term_id,
        "keyword:eclair"
    );
}

#[test]
fn exact_detail_keeps_evidence_placeholders_cycles_and_unresolved_edges_bounded() {
    let catalog = catalog();
    let reader = catalog.reader();
    let reference = GlossaryTermReference {
        catalog: catalog.binding().clone(),
        term_id: "status:strength".to_owned(),
    };
    let detail = reader.get(&reference, reference_scope()).expect("detail");
    assert_eq!(detail.parameter_placeholders, vec!["{amount}".to_owned()]);
    assert!(matches!(
        detail.definition,
        GlossaryDefinitionText::Available(_)
    ));
    assert!(matches!(
        detail.evidence,
        GlossaryEvidence::NativeTooltip { .. }
    ));
    assert_eq!(detail.related_terms.len(), 2);
    assert!(detail.related_terms.iter().any(|edge| {
        edge.term_id == "status:missing"
            && matches!(
                edge.resolution,
                sts2_game_mod::GlossaryRelatedTermResolution::Unresolved(
                    GlossaryUnresolvedReason::Missing
                )
            )
    }));
    let weak_reference = detail
        .related_terms
        .iter()
        .find_map(|edge| match &edge.resolution {
            sts2_game_mod::GlossaryRelatedTermResolution::Resolved(reference) => {
                Some(reference.clone())
            }
            sts2_game_mod::GlossaryRelatedTermResolution::Unresolved(_) => None,
        })
        .expect("resolved cycle edge");
    let weak = reader
        .get(&weak_reference, reference_scope())
        .expect("weak");
    assert_eq!(weak.related_terms[0].term_id, "status:strength");
    assert!(weak.content_references.is_empty());
    assert!(detail.content_references.iter().any(|reference| {
        reference.namespaced_id == "base:ironclad:strike"
            && matches!(
                reference.resolution,
                sts2_game_mod::GlossaryContentReferenceResolution::Resolved(_)
            )
    }));
}
