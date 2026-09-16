// SPDX-License-Identifier: MIT

use serde_json::{Value, json};
use sts2_protocol::{
    GAME_INFORMATION_CONTENT_MANIFEST_V1_ARTIFACT,
    GAME_INFORMATION_CONTENT_MANIFEST_V1_PROTOCOL_VERSION,
    GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST,
    GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_SOURCE, GameInformationContentManifestV1Codec,
    GameInformationContentManifestV1CodecError,
};

use super::{
    ContentDefinition, ContentManifest, ContentManifestError, ContentManifestProducer,
    ContentSourceError,
};

const PRODUCER: &str = "hand-authored";

/// Owner-local HTTP result for one complete, protocol-validated manifest envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentManifestWireResponse {
    /// HTTP status selected by the game-mod owner adapter.
    pub status: u16,
    /// Compact UTF-8 protocol envelope.
    pub body: Vec<u8>,
}

/// Failure to build a valid bounded protocol envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContentManifestWireError {
    /// The correlation identity or a mapped field cannot satisfy the pinned schema.
    InvalidEnvelope,
    /// The codec could not initialize the pinned schema or serialize an envelope.
    CodecFailure,
}

impl ContentManifestProducer {
    /// Produces and encodes one complete v1 response, mapping source and validation failures to
    /// closed protocol error tokens. Semantic inputs and localized text are never serialized.
    pub fn produce_wire<S: super::ContentCatalogSource>(
        &self,
        source: &S,
        correlation_id: &str,
    ) -> Result<ContentManifestWireResponse, ContentManifestWireError> {
        let codec = GameInformationContentManifestV1Codec::new()
            .map_err(|_| ContentManifestWireError::CodecFailure)?;
        match self.produce(source) {
            Ok(manifest) => match codec.encode(&manifest_value(&manifest, correlation_id)) {
                Ok(body) => Ok(ContentManifestWireResponse { status: 200, body }),
                Err(GameInformationContentManifestV1CodecError::MessageTooLarge) => encode_error(
                    &codec,
                    correlation_id,
                    413,
                    "result_limit_exceeded",
                    "serialized_payload_too_large",
                ),
                Err(GameInformationContentManifestV1CodecError::InvalidMessage) => {
                    encode_error(&codec, correlation_id, 500, "malformed", "source_malformed")
                }
                Err(GameInformationContentManifestV1CodecError::UnsupportedSchemaDigest) => {
                    Err(ContentManifestWireError::InvalidEnvelope)
                }
                Err(_) => Err(ContentManifestWireError::CodecFailure),
            },
            Err(error) => {
                let (status, code, reason) = map_manifest_error(&error);
                encode_error(&codec, correlation_id, status, code, reason)
            }
        }
    }
}

fn manifest_value(manifest: &ContentManifest, correlation_id: &str) -> Value {
    json!({
        "protocol_version": GAME_INFORMATION_CONTENT_MANIFEST_V1_PROTOCOL_VERSION,
        "schema_digest": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST,
        "provenance": {
            "artifact": GAME_INFORMATION_CONTENT_MANIFEST_V1_ARTIFACT,
            "source": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_SOURCE,
            "generator": PRODUCER
        },
        "correlation_id": correlation_id,
        "kind": "content_manifest_response",
        "manifest": {
            "game_build": manifest.game_build,
            "adapter_compatibility": manifest.adapter_compatibility,
            "catalog_generation": manifest.catalog_generation,
            "locale": manifest.locale,
            "packages": manifest.packages.iter().map(|package| json!({
                "package_id": package.package_id,
                "package_version": package.package_version,
                "order": package.order
            })).collect::<Vec<_>>(),
            "families": manifest.families.iter().map(|family| json!({
                "entity_kind": family.entity_kind,
                "handled": family.handled,
                "definition_count": family.definition_count
            })).collect::<Vec<_>>(),
            "definitions": manifest.definitions.iter().map(definition_value).collect::<Vec<_>>(),
            "content_set_revision": manifest.content_set_revision,
            "localized_text_revision": manifest.localized_text_revision,
            "inventory_revision": manifest.inventory_revision
        },
        "error": null
    })
}

fn definition_value(definition: &ContentDefinition) -> Value {
    json!({
        "entity_kind": definition.entity_kind,
        "namespaced_id": definition.namespaced_id,
        "handled": definition.handled,
        "origin": {
            "package_id": definition.origin.package_id,
            "package_version": definition.origin.package_version
        },
        "override_chain": definition.override_chain,
        "semantic_revision": definition.semantic_revision,
        "localized_text_revision": definition.localized_text_revision
    })
}

fn map_manifest_error(error: &ContentManifestError) -> (u16, &'static str, &'static str) {
    match error {
        ContentManifestError::Source(ContentSourceError::Unavailable) => {
            (503, "missing_capability", "source_unavailable")
        }
        ContentManifestError::Source(ContentSourceError::AccessDenied) => {
            (403, "access_denied", "source_access_denied")
        }
        ContentManifestError::Source(ContentSourceError::Malformed) => {
            (500, "malformed", "source_malformed")
        }
        ContentManifestError::CatalogChanged { .. } => (500, "malformed", "catalog_changed"),
        ContentManifestError::InvalidIdentity(_) => (500, "malformed", "invalid_identity"),
        ContentManifestError::InvalidSemanticInput => (500, "malformed", "invalid_semantic_input"),
        ContentManifestError::InvalidLocalizedText => (500, "malformed", "invalid_localized_text"),
        ContentManifestError::InvalidPackageVersion => {
            (500, "malformed", "invalid_package_version")
        }
        ContentManifestError::DuplicatePackage => (500, "malformed", "duplicate_package"),
        ContentManifestError::InvalidPackageOrder => (500, "malformed", "invalid_package_order"),
        ContentManifestError::DuplicateEntityKind => (500, "malformed", "duplicate_entity_kind"),
        ContentManifestError::UnknownEntityKind => (500, "malformed", "unknown_entity_kind"),
        ContentManifestError::DuplicateDefinition => (500, "malformed", "duplicate_definition"),
        ContentManifestError::UnknownOriginPackage => (500, "malformed", "unknown_origin_package"),
        ContentManifestError::OriginPackageVersionMismatch => {
            (500, "malformed", "origin_package_version_mismatch")
        }
        ContentManifestError::DuplicateOverrideReference => {
            (500, "malformed", "duplicate_override_reference")
        }
        ContentManifestError::RegistryCountCoverageMismatch
        | ContentManifestError::RegistryDefinitionCountMismatch => {
            (500, "malformed", "source_malformed")
        }
    }
}

fn encode_error(
    codec: &GameInformationContentManifestV1Codec,
    correlation_id: &str,
    status: u16,
    code: &'static str,
    reason: &'static str,
) -> Result<ContentManifestWireResponse, ContentManifestWireError> {
    let envelope = json!({
        "protocol_version": GAME_INFORMATION_CONTENT_MANIFEST_V1_PROTOCOL_VERSION,
        "schema_digest": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_DIGEST,
        "provenance": {
            "artifact": GAME_INFORMATION_CONTENT_MANIFEST_V1_ARTIFACT,
            "source": GAME_INFORMATION_CONTENT_MANIFEST_V1_SCHEMA_SOURCE,
            "generator": PRODUCER
        },
        "correlation_id": correlation_id,
        "kind": "error_response",
        "manifest": null,
        "error": { "code": code, "reason": reason }
    });
    let body = codec
        .encode(&envelope)
        .map_err(|_| ContentManifestWireError::InvalidEnvelope)?;
    Ok(ContentManifestWireResponse { status, body })
}
