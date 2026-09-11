// SPDX-License-Identifier: MIT

//! Owner-local identity and checks for the copied Runtime-map-v1 protocol artifact.
//!
//! The protocol target owns the normative schema. This module keeps the game-mod consumer bound
//! to one reviewed artifact revision and provides a fail-closed check for the copied artifact.
//! It does not validate runtime map payloads or provide payload-consumer conformance.

use serde_json::Value;
use sha2::{Digest, Sha256};
/// Protocol version required by every Runtime-map-v1 message.
pub const RUNTIME_MAP_V1_PROTOCOL_VERSION: &str = "runtime-map-v1";
/// Release-like artifact identity from the protocol-owner handoff.
pub const RUNTIME_MAP_V1_ARTIFACT: &str = "sts2-protocol/runtime-map-v1";
/// Repository-relative source recorded by the release-like artifact.
pub const RUNTIME_MAP_V1_SCHEMA_SOURCE: &str = "schemas/runtime-map-v1.schema.json";
/// Generator recorded by the hand-authored release-like artifact.
pub const RUNTIME_MAP_V1_GENERATOR: &str = "hand-authored";
/// Schema digest from the accepted protocol-owner handoff.
pub const RUNTIME_MAP_V1_SCHEMA_DIGEST: &str =
    "ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b";
/// Maximum JSON-safe generation and lease epoch.
pub const RUNTIME_MAP_V1_MAX_GENERATION: u64 = 9_007_199_254_740_991;
/// Maximum visible nodes in one projection.
pub const RUNTIME_MAP_V1_MAX_NODES: usize = 256;
/// Maximum directed edges in one projection.
pub const RUNTIME_MAP_V1_MAX_EDGES: usize = 1_024;
/// Maximum generation-bound action bindings in one projection.
pub const RUNTIME_MAP_V1_MAX_BINDINGS: usize = 256;
/// Maximum history or terminal-node entries in one projection.
pub const RUNTIME_MAP_V1_MAX_HISTORY: usize = 256;
/// Maximum encoded Runtime-map-v1 response size.
pub const RUNTIME_MAP_V1_MAX_MESSAGE_BYTES: usize = 256 * 1024;

const MANIFEST: &str = include_str!("../../../../protocol-artifact/runtime-map-v1/manifest.json");
const SCHEMA: &str = include_str!("../../../../protocol-artifact/runtime-map-v1/schema.json");
const SOURCE_SCHEMA: &str = include_str!("../../../../schemas/runtime-map-v1.schema.json");
const CONFORMANCE: &str = include_str!("../../../../conformance/cases/runtime-map-v1.json");
const CHECKSUMS: &str = include_str!("../../../../protocol-artifact/runtime-map-v1/SHA256SUMS");
const SNAPSHOT_REQUEST: &str =
    include_str!("../../../../protocol-artifact/runtime-map-v1/golden/snapshot-request.json");
const SNAPSHOT_RESPONSE: &str =
    include_str!("../../../../protocol-artifact/runtime-map-v1/golden/snapshot-response.json");
const VISIBLE_MAP: &str =
    include_str!("../../../../protocol-artifact/runtime-map-v1/golden/visible-map.json");

const EXPECTED_CHECKSUMS: [&str; 7] = [
    "0d6bc5f9268b42c852b2d4fea120593d69c748cd6fccd73d5833c7658b662091  ../../conformance/cases/runtime-map-v1.json",
    "ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b  ../../schemas/runtime-map-v1.schema.json",
    "e4a1bb88587fc67e2279651baa06334a4f1bc76edad5aec34f662f5eea1f21cf  manifest.json",
    "ceab0d2dfc471d1ec36d12edaf4654b8c7fdced06548bf47265e11c63f98115b  schema.json",
    "31b0d8c2cea8611f3b5d67e471f0ddeacca37d0e48d2b27dfef8d905c3dd0ca4  golden/snapshot-request.json",
    "95152c6aee09c40eaba7fcb2ab704f881f013cf75865b18a56fdb96a392925c6  golden/snapshot-response.json",
    "bbb959f20a5293032072ee9ab30c9489807ad185cec7d740f15dd929a9bfa622  golden/visible-map.json",
];

const EXPECTED_CONSUMERS: [&str; 5] = [
    "sts2-game-mod",
    "sts2-gateway",
    "sts2-harness",
    "sts2-mcp-server",
    "ascension-map-visualizer",
];

const EXPECTED_GOLDENS: [&str; 3] = [
    "golden/snapshot-request.json",
    "golden/snapshot-response.json",
    "golden/visible-map.json",
];

/// Failure while loading or checking the copied Runtime-map-v1 artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeMapArtifactError {
    /// A copied JSON file is malformed.
    InvalidJson,
    /// Manifest metadata or consumer inventory does not match.
    ManifestMismatch,
    /// The source and package schema copies or schema identity do not match.
    SchemaMismatch,
    /// The copied checksum inventory or file bytes do not match the accepted handoff.
    ChecksumMismatch,
    /// Conformance metadata or golden fixture identity does not match.
    FixtureMismatch,
}

impl std::fmt::Display for RuntimeMapArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied Runtime-map-v1 artifact is invalid")
    }
}

impl std::error::Error for RuntimeMapArtifactError {}

/// Verifies the copied protocol artifact for a Runtime-map-v1 caller.
pub fn verify_runtime_map_artifact() -> Result<(), RuntimeMapArtifactError> {
    let manifest = parse(MANIFEST)?;
    let consumers = manifest["consumers"]
        .as_array()
        .ok_or(RuntimeMapArtifactError::ManifestMismatch)?;
    let goldens = manifest["goldens"]
        .as_array()
        .ok_or(RuntimeMapArtifactError::ManifestMismatch)?;
    let consumers_match = consumers.len() == EXPECTED_CONSUMERS.len()
        && consumers
            .iter()
            .zip(EXPECTED_CONSUMERS)
            .all(|(value, expected)| value == expected);
    let goldens_match = goldens.len() == EXPECTED_GOLDENS.len()
        && goldens
            .iter()
            .zip(EXPECTED_GOLDENS)
            .all(|(value, expected)| value == expected);
    if manifest["artifact"] != RUNTIME_MAP_V1_ARTIFACT
        || manifest["protocol_version"] != RUNTIME_MAP_V1_PROTOCOL_VERSION
        || manifest["schema"] != "schema.json"
        || manifest["schema_digest"] != RUNTIME_MAP_V1_SCHEMA_DIGEST
        || manifest["provenance"]["source"] != RUNTIME_MAP_V1_SCHEMA_SOURCE
        || manifest["provenance"]["generator"] != RUNTIME_MAP_V1_GENERATOR
        || manifest["provenance"]["license"] != "MIT"
        || manifest["checksums"] != "SHA256SUMS"
        || !consumers_match
        || !goldens_match
    {
        return Err(RuntimeMapArtifactError::ManifestMismatch);
    }

    let schema = parse(SCHEMA)?;
    let conformance = parse(CONFORMANCE)?;
    if SOURCE_SCHEMA != SCHEMA
        || schema["$schema"] != "https://json-schema.org/draft/2020-12/schema"
        || schema["$id"] != "sts2-runtime-map-v1"
        || schema["oneOf"].as_array().map(Vec::len) != Some(3)
        || schema["$defs"]["text"]["maxLength"] != 128
        || schema["$defs"]["reason"]["maxLength"] != 256
        || conformance["case_id"] != "CT-RUNTIME-MAP-V1-001"
        || conformance["contract"] != "sts2.protocol/runtime-map-v1"
        || conformance["profile"] != RUNTIME_MAP_V1_PROTOCOL_VERSION
        || conformance["schema"] != RUNTIME_MAP_V1_SCHEMA_SOURCE
        || conformance["checksums"] != "artifacts/runtime-map-v1/SHA256SUMS"
        || conformance["consumers"] != manifest["consumers"]
    {
        return Err(RuntimeMapArtifactError::SchemaMismatch);
    }

    if CHECKSUMS.lines().collect::<Vec<_>>().as_slice() != EXPECTED_CHECKSUMS.as_slice() {
        return Err(RuntimeMapArtifactError::ChecksumMismatch);
    }
    verify_bytes([
        CONFORMANCE,
        SOURCE_SCHEMA,
        MANIFEST,
        SCHEMA,
        SNAPSHOT_REQUEST,
        SNAPSHOT_RESPONSE,
        VISIBLE_MAP,
    ])?;

    let request = parse(SNAPSHOT_REQUEST)?;
    let response = parse(SNAPSHOT_RESPONSE)?;
    let visible_map = parse(VISIBLE_MAP)?;
    if request["kind"] != "snapshot_request"
        || response["kind"] != "snapshot_response"
        || response["snapshot"].is_null()
        || visible_map["schema_version"] != "visible-map-v1"
        || response["snapshot"] != visible_map
    {
        return Err(RuntimeMapArtifactError::FixtureMismatch);
    }
    Ok(())
}

fn parse(text: &str) -> Result<Value, RuntimeMapArtifactError> {
    serde_json::from_str(text).map_err(|_| RuntimeMapArtifactError::InvalidJson)
}

fn verify_bytes(files: [&str; 7]) -> Result<(), RuntimeMapArtifactError> {
    for (contents, entry) in files.into_iter().zip(EXPECTED_CHECKSUMS) {
        let digest = hex_digest(contents.as_bytes());
        if entry.split_once("  ").map(|(expected, _)| expected) != Some(digest.as_str()) {
            return Err(RuntimeMapArtifactError::ChecksumMismatch);
        }
    }
    Ok(())
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_byte_changes_in_every_inventory_entry() {
        let files = [
            CONFORMANCE,
            SOURCE_SCHEMA,
            MANIFEST,
            SCHEMA,
            SNAPSHOT_REQUEST,
            SNAPSHOT_RESPONSE,
            VISIBLE_MAP,
        ];
        assert_eq!(verify_bytes(files), Ok(()));
        for index in 0..files.len() {
            // Valid JSON with unchanged semantics still has different artifact bytes.
            let changed = format!("{}\n", files[index]);
            let mut altered = files;
            altered[index] = &changed;
            assert_eq!(
                verify_bytes(altered),
                Err(RuntimeMapArtifactError::ChecksumMismatch)
            );
        }
    }
}
