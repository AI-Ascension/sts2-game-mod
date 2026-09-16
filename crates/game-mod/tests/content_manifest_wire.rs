// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifestProducer,
    ContentOriginInput, ContentPackageInput, ContentSourceError,
};
use sts2_protocol::GameInformationContentManifestV1Codec;

#[derive(Clone)]
struct Catalog(ContentCatalogSnapshot);

impl ContentCatalogSource for Catalog {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.0.clone())
    }
}

struct Unavailable(ContentSourceError);

impl ContentCatalogSource for Unavailable {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Err(self.0)
    }
}

fn producer() -> ContentManifestProducer {
    ContentManifestProducer::new("sts2-game-mod:adapter-v1", ["card".to_owned()])
        .expect("valid producer")
}

fn snapshot(definition_count: usize) -> ContentCatalogSnapshot {
    let definitions = (0..definition_count)
        .map(|index| ContentDefinitionInput {
            entity_kind: "card".to_owned(),
            namespaced_id: format!("mod:card:{index}"),
            semantic_inputs: "damage=6;cost=1".to_owned(),
            localized_text: Some("Strike".to_owned()),
            origin: ContentOriginInput {
                package_id: Some("base:game".to_owned()),
                package_version: Some("0.107.1".to_owned()),
            },
            override_chain: Vec::new(),
        })
        .collect();
    ContentCatalogSnapshot {
        generation_before: 19,
        generation_after: 19,
        game_build: "sts2-build:0.107.1".to_owned(),
        locale: "en_US".to_owned(),
        packages: vec![ContentPackageInput {
            package_id: "base:game".to_owned(),
            package_version: Some("0.107.1".to_owned()),
            order: 0,
        }],
        available_entity_kinds: vec!["card".to_owned()],
        registry_definition_counts: [("card".to_owned(), definition_count)].into(),
        definitions,
    }
}

#[test]
fn canonical_producer_output_is_encoded_without_raw_semantic_or_localized_text() {
    let source = Catalog(snapshot(1));
    let produced = producer()
        .produce(&source)
        .expect("source-only canonical producer should succeed");
    let response = producer()
        .produce_wire(&source, "corr-content-manifest-1")
        .expect("wire encoding should succeed");
    assert_eq!(response.status, 200);

    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    let envelope = codec.decode(&response.body).expect("closed valid envelope");
    assert_eq!(envelope["kind"], "content_manifest_response");
    assert_eq!(envelope["correlation_id"], "corr-content-manifest-1");
    assert_eq!(
        envelope["manifest"]["inventory_revision"],
        produced.inventory_revision
    );
    assert_eq!(envelope["manifest"]["locale"], "en_US");
    assert_eq!(envelope["manifest"]["packages"][0]["order"], 0);
    assert_eq!(
        envelope["manifest"]["definitions"][0]["semantic_revision"],
        produced.definitions[0].semantic_revision
    );
    let body = std::str::from_utf8(&response.body).expect("codec emits UTF-8");
    assert!(!body.contains("damage=6"));
    assert!(!body.contains("Strike"));
}

#[test]
fn source_failures_map_to_closed_protocol_reasons_and_statuses() {
    let cases = [
        (
            ContentSourceError::Unavailable,
            503,
            "missing_capability",
            "source_unavailable",
        ),
        (
            ContentSourceError::AccessDenied,
            403,
            "access_denied",
            "source_access_denied",
        ),
        (
            ContentSourceError::Malformed,
            500,
            "malformed",
            "source_malformed",
        ),
    ];
    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    for (source_error, expected_status, expected_code, expected_reason) in cases {
        let response = producer()
            .produce_wire(&Unavailable(source_error), "corr-error-1")
            .expect("closed source error should encode");
        assert_eq!(response.status, expected_status);
        let envelope = codec.decode(&response.body).expect("closed valid error");
        assert_eq!(envelope["kind"], "error_response");
        assert_eq!(envelope["error"]["code"], expected_code);
        assert_eq!(envelope["error"]["reason"], expected_reason);
        assert!(envelope["manifest"].is_null());
    }
}

#[test]
fn invalid_catalog_is_not_repaired_or_emitted_as_success() {
    let mut snapshot = snapshot(1);
    snapshot.generation_after += 1;
    let response = producer()
        .produce_wire(&Catalog(snapshot), "corr-changed-1")
        .expect("catalog error should encode");
    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    let envelope = codec.decode(&response.body).expect("closed valid error");
    assert_eq!(response.status, 500);
    assert_eq!(envelope["kind"], "error_response");
    assert_eq!(envelope["error"]["code"], "malformed");
    assert_eq!(envelope["error"]["reason"], "catalog_changed");
}

#[test]
fn whole_payload_over_limit_returns_a_small_error_envelope_without_truncation() {
    let response = producer()
        .produce_wire(&Catalog(snapshot(60_000)), "corr-large-1")
        .expect("oversized result should map to a closed error");
    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    let envelope = codec.decode(&response.body).expect("bounded valid error");
    assert_eq!(response.status, 413);
    assert_eq!(envelope["kind"], "error_response");
    assert_eq!(envelope["error"]["code"], "result_limit_exceeded");
    assert_eq!(envelope["error"]["reason"], "serialized_payload_too_large");
    assert!(response.body.len() < 1024);
}

#[test]
fn malformed_correlation_is_refused_without_an_unvalidated_wire_body() {
    let result = producer().produce_wire(&Catalog(snapshot(0)), "unsafe correlation");
    assert!(result.is_err());
}

#[test]
fn producer_output_outside_protocol_array_bounds_maps_to_source_malformed() {
    let mut source = snapshot(0);
    source.packages = (0..=4096)
        .map(|index| ContentPackageInput {
            package_id: format!("mod:package:{index}"),
            package_version: None,
            order: index,
        })
        .collect();
    let response = producer()
        .produce_wire(&Catalog(source), "corr-bounds-1")
        .expect("schema-bound violation should return a closed error");
    let codec = GameInformationContentManifestV1Codec::new().expect("pinned protocol schema");
    let envelope = codec.decode(&response.body).expect("closed valid error");
    assert_eq!(response.status, 500);
    assert_eq!(envelope["error"]["code"], "malformed");
    assert_eq!(envelope["error"]["reason"], "source_malformed");
}
