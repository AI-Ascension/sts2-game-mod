// SPDX-License-Identifier: MIT

use serde::Deserialize;
use std::collections::BTreeMap;
use sts2_game_mod::{
    ContentCatalogSnapshot, ContentCatalogSource, ContentDefinitionInput, ContentManifestProducer,
    ContentOriginInput, ContentPackageInput, ContentSourceError,
};

const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
const STATUS_OK: i32 = 200;
const STATUS_BAD_REQUEST: i32 = 400;
const STATUS_UNAVAILABLE: i32 = 503;
const STATUS_INTERNAL: i32 = 500;

#[derive(Clone, Debug, Deserialize)]
struct Input {
    generation_before: u64,
    generation_after: u64,
    game_build: String,
    locale: String,
    packages: Vec<Package>,
    available_entity_kinds: Vec<String>,
    registry_definition_counts: BTreeMap<String, usize>,
    definitions: Vec<Definition>,
    adapter_compatibility: String,
}

#[derive(Clone, Debug, Deserialize)]
struct Package {
    package_id: String,
    package_version: Option<String>,
    order: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct Definition {
    entity_kind: String,
    namespaced_id: String,
    semantic_inputs: String,
    localized_text: Option<String>,
    origin: Origin,
    override_chain: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Origin {
    package_id: Option<String>,
    package_version: Option<String>,
}

#[derive(Clone, Debug)]
struct FfiSource {
    snapshot: ContentCatalogSnapshot,
}

impl ContentCatalogSource for FfiSource {
    fn read_catalog(&self) -> Result<ContentCatalogSnapshot, ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}

fn convert(input: Input) -> ContentCatalogSnapshot {
    ContentCatalogSnapshot {
        generation_before: input.generation_before,
        generation_after: input.generation_after,
        game_build: input.game_build,
        locale: input.locale,
        packages: input
            .packages
            .into_iter()
            .map(|package| ContentPackageInput {
                package_id: package.package_id,
                package_version: package.package_version,
                order: package.order,
            })
            .collect(),
        available_entity_kinds: input.available_entity_kinds,
        registry_definition_counts: input.registry_definition_counts,
        definitions: input
            .definitions
            .into_iter()
            .map(|definition| ContentDefinitionInput {
                entity_kind: definition.entity_kind,
                namespaced_id: definition.namespaced_id,
                semantic_inputs: definition.semantic_inputs,
                localized_text: definition.localized_text,
                origin: ContentOriginInput {
                    package_id: definition.origin.package_id,
                    package_version: definition.origin.package_version,
                },
                override_chain: definition.override_chain,
            })
            .collect(),
    }
}

fn write_output(output: *mut u8, capacity: usize, length: *mut usize, bytes: &[u8]) -> i32 {
    if output.is_null() || length.is_null() || bytes.len() > capacity {
        return STATUS_INTERNAL;
    }
    // SAFETY: The caller supplies writable storage for `capacity` bytes and a valid length
    // pointer for the duration of this call; the bounds check above protects the copy.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), output, bytes.len());
        *length = bytes.len();
    }
    STATUS_OK
}

fn produce(input: &[u8], correlation: &str) -> (i32, Vec<u8>) {
    if input.len() > MAX_INPUT_BYTES {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"content_manifest_input_limit"}"#.to_vec(),
        );
    }
    let Ok(input) = serde_json::from_slice::<Input>(input) else {
        return (
            STATUS_BAD_REQUEST,
            br#"{"error_code":"content_manifest_input_malformed"}"#.to_vec(),
        );
    };
    let supported = vec!["card".to_owned()];
    let Ok(producer) = ContentManifestProducer::new(input.adapter_compatibility.clone(), supported)
    else {
        return (
            STATUS_INTERNAL,
            br#"{"error_code":"content_manifest_adapter_invalid"}"#.to_vec(),
        );
    };
    let source = FfiSource {
        snapshot: convert(input),
    };
    match producer.produce_wire(&source, correlation) {
        Ok(response) => (i32::from(response.status), response.body),
        Err(_) => (
            STATUS_UNAVAILABLE,
            br#"{"error_code":"content_manifest_source_unavailable"}"#.to_vec(),
        ),
    }
}

/// Produces the canonical content-manifest wire response from an owner-thread JSON snapshot.
///
/// This is an internal managed/native seam. The input is copied by the caller before this
/// function is invoked and the output is written only into caller-owned bounded storage.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sts2_game_mod_content_manifest_produce(
    input: *const u8,
    input_length: usize,
    correlation: *const u8,
    correlation_length: usize,
    output: *mut u8,
    output_capacity: usize,
    output_length: *mut usize,
) -> i32 {
    let result = std::panic::catch_unwind(|| {
        if input.is_null() || correlation.is_null() || output_length.is_null() {
            return (STATUS_BAD_REQUEST, Vec::new());
        }
        // SAFETY: The caller contract requires readable UTF-8 buffers for the declared lengths.
        let input = unsafe { std::slice::from_raw_parts(input, input_length) };
        let correlation = unsafe { std::slice::from_raw_parts(correlation, correlation_length) };
        let Ok(correlation) = std::str::from_utf8(correlation) else {
            return (STATUS_BAD_REQUEST, Vec::new());
        };
        produce(input, correlation)
    });
    match result {
        Ok((status, bytes)) => {
            if bytes.is_empty() {
                return status;
            }
            let write_status = write_output(output, output_capacity, output_length, &bytes);
            if write_status == STATUS_OK {
                status
            } else {
                write_status
            }
        }
        Err(_) => STATUS_INTERNAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn ffi_produces_canonical_manifest_for_bounded_owner_snapshot() {
        let input = serde_json::to_vec(&json!({
            "generation_before": 11,
            "generation_after": 11,
            "game_build": "v0.107.1",
            "locale": "en_US",
            "packages": [{
                "package_id": "base:game",
                "package_version": "0.107.1",
                "order": 0
            }],
            "available_entity_kinds": ["card", "relic"],
            "registry_definition_counts": { "card": 1, "relic": 1 },
            "definitions": [
                {
                    "entity_kind": "card",
                    "namespaced_id": "base:card:strike",
                    "semantic_inputs": "{\"id\":\"base:card:strike\"}",
                    "localized_text": "{\"title\":\"Strike\"}",
                    "origin": { "package_id": null, "package_version": null },
                    "override_chain": []
                },
                {
                    "entity_kind": "relic",
                    "namespaced_id": "base:relic:ring",
                    "semantic_inputs": "{\"id\":\"base:relic:ring\"}",
                    "localized_text": null,
                    "origin": { "package_id": null, "package_version": null },
                    "override_chain": []
                }
            ],
            "adapter_compatibility": "sts2-game-mod-modeldb-card-v1"
        }))
        .expect("fixture serializes");
        let correlation = b"corr-ffi";
        let mut output = vec![0_u8; 16 * 1024 * 1024];
        let mut output_length = 0;
        // SAFETY: All pointers reference live buffers for their declared lengths and the output
        // vector has the capacity passed to the ABI.
        let status = unsafe {
            sts2_game_mod_content_manifest_produce(
                input.as_ptr(),
                input.len(),
                correlation.as_ptr(),
                correlation.len(),
                output.as_mut_ptr(),
                output.len(),
                &mut output_length,
            )
        };
        assert_eq!(status, 200);
        let codec =
            sts2_protocol::GameInformationContentManifestV1Codec::new().expect("pinned schema");
        let envelope = codec
            .decode(&output[..output_length])
            .expect("producer output validates");
        assert_eq!(envelope["correlation_id"], "corr-ffi");
        assert_eq!(envelope["kind"], "content_manifest_response");
        assert_eq!(envelope["manifest"]["families"][1]["entity_kind"], "relic");
        assert_eq!(envelope["manifest"]["families"][1]["handled"], false);
        assert_eq!(
            envelope["manifest"]["definitions"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
}
