// SPDX-License-Identifier: MIT

use serde_json::Value;

/// Version consumed by the game-mod POC mapping.
pub const POC_PROTOCOL_VERSION: &str = "poc-v1";
/// Schema digest supplied by the protocol release-like artifact.
pub const POC_SCHEMA_DIGEST: &str =
    "242b8f9233e915a55ea8d2e72ca476c1258169a67e62de72ee5aed848a6a0a19";
/// Release-like artifact identity, not a Rust package dependency.
pub const POC_ARTIFACT: &str = "sts2-protocol/poc-v1";
/// Repository-relative source recorded in every POC message.
pub const POC_SCHEMA_SOURCE: &str = "schemas/poc-v1.schema.json";
/// Package-relative schema path recorded by the copied artifact manifest.
pub const POC_SCHEMA_PACKAGE: &str = "schema.json";
/// Generator recorded in the hand-authored release-like artifact.
pub const POC_GENERATOR: &str = "hand-authored";
/// Maximum fake budget represented by the bounded POC contract.
pub const POC_MAX_UNITS: u16 = 8;
/// Maximum settled-effect count represented by the bounded POC contract.
pub const POC_MAX_SETTLED_EFFECTS: u16 = 4;
/// Maximum generation representable as a JSON-safe integer in the POC contract.
pub const POC_MAX_GENERATION: u64 = 9_007_199_254_740_991;

const MANIFEST: &str = include_str!("../../../protocol-artifact/poc-v1/manifest.json");
const SCHEMA: &str = include_str!("../../../protocol-artifact/poc-v1/schema.json");
const SOURCE_SCHEMA: &str = include_str!("../../../schemas/poc-v1.schema.json");
const STATE_REQUEST: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/state-request.json");
const STATE_RESPONSE: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/state-response.json");
const ACTION_REQUEST: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-request.json");
const ACTION_ACCEPTED: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-accepted.json");
const ACTION_REJECTED: &str =
    include_str!("../../../protocol-artifact/poc-v1/golden/action-rejected.json");
const INVALID_ACTION: &str =
    include_str!("../../../protocol-artifact/poc-v1/fixtures/invalid-action.json");
const CONFORMANCE: &str = include_str!("../../../conformance/cases/poc-v1.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/poc-v1/SHA256SUMS");

const EXPECTED_CHECKSUMS: [&str; 10] = [
    "55fc488f4387d5eb9f0bd185f80b862c7dc00ca8fac2af5d43ee738d437ce627  ../../conformance/cases/poc-v1.json",
    "242b8f9233e915a55ea8d2e72ca476c1258169a67e62de72ee5aed848a6a0a19  ../../schemas/poc-v1.schema.json",
    "29b245f9e0df6c6f158e82e7a770e90e8153b427b3e18e7b00c2340b7a812abf  fixtures/invalid-action.json",
    "733e4fba7a457bfaf7d1da689369f10974bfde39e4dbae0c1254a6e95ed55a6e  golden/action-accepted.json",
    "3c8681361dd87b01969f82aae4ca00f3551e2f07e3215777bba552e2fd4d31ca  golden/action-rejected.json",
    "0ee20e4b8692e8462288faeacb2f2e78bf986c57d60d89479a31a01cf889286e  golden/action-request.json",
    "46c74fc562031c98f38cc7901f60e06022ec14c6d55b814ae809b571aa58f738  golden/state-request.json",
    "816b698fe1d6acd867ef1319d4a51623b9b0d2fa81d82dcfc317c45b6836e2c6  golden/state-response.json",
    "30c8b85a87ff453e9709156ccde65d74722b7c48c0b61a802a28d04277dd3725  manifest.json",
    "242b8f9233e915a55ea8d2e72ca476c1258169a67e62de72ee5aed848a6a0a19  schema.json",
];

/// Verifies the local protocol artifact before the mod accepts POC messages.
pub fn verify_poc_artifact() -> Result<(), ArtifactError> {
    let manifest = parse(MANIFEST)?;
    if manifest["artifact"] != POC_ARTIFACT
        || manifest["protocol_version"] != POC_PROTOCOL_VERSION
        || manifest["schema_digest"] != POC_SCHEMA_DIGEST
        || manifest["schema"] != POC_SCHEMA_PACKAGE
        || manifest["provenance"]["source"] != POC_SCHEMA_SOURCE
        || manifest["provenance"]["generator"] != POC_GENERATOR
        || manifest["provenance"]["license"] != "MIT"
    {
        return Err(ArtifactError::ManifestMismatch);
    }
    let consumers_match = manifest["consumers"].as_array().map(|consumers| {
        let expected = [
            "sts2-game-core",
            "sts2-game-mod",
            "sts2-gateway",
            "sts2-harness",
            "sts2-mcp-server",
        ];
        consumers.len() == expected.len()
            && consumers
                .iter()
                .zip(expected)
                .all(|(consumer, expected)| consumer == expected)
    });
    if consumers_match != Some(true) {
        return Err(ArtifactError::ManifestMismatch);
    }
    let schema = parse(SCHEMA)?;
    if SOURCE_SCHEMA != SCHEMA
        || schema["$schema"] != "https://json-schema.org/draft/2020-12/schema"
        || schema["$id"] != "sts2-poc-v1"
        || schema["oneOf"].as_array().map(Vec::len) != Some(4)
        || schema["$defs"]["base"]["properties"]["generation"]["maximum"] != POC_MAX_GENERATION
        || [
            "state_request",
            "state_response",
            "action_request",
            "action_response",
        ]
        .iter()
        .any(|shape| schema["$defs"][shape]["unevaluatedProperties"] != false)
    {
        return Err(ArtifactError::SchemaMismatch);
    }
    let checksum_lines: Vec<_> = CHECKSUMS.lines().collect();
    if checksum_lines.as_slice() != EXPECTED_CHECKSUMS.as_slice() {
        return Err(ArtifactError::ChecksumMismatch);
    }
    let state_request = parse(STATE_REQUEST)?;
    let state_response = parse(STATE_RESPONSE)?;
    let action_request = parse(ACTION_REQUEST)?;
    let action_accepted = parse(ACTION_ACCEPTED)?;
    let action_rejected = parse(ACTION_REJECTED)?;
    let invalid_action = parse(INVALID_ACTION)?;
    if state_request["kind"] != "state_request"
        || state_response["kind"] != "state_response"
        || action_request["kind"] != "action_request"
        || action_request["action"]["units"] != 1
        || action_accepted["kind"] != "action_response"
        || action_accepted["status"] != "accepted"
        || action_rejected["kind"] != "action_response"
        || action_rejected["status"] != "rejected"
        || invalid_action["kind"] != "action_request"
        || invalid_action["action"]["action_id"] != "use_budget"
        || invalid_action["action"]["units"] != 0
    {
        return Err(ArtifactError::FixtureMismatch);
    }
    let conformance = parse(CONFORMANCE)?;
    if conformance["case_id"] != "CT-POC-V1-001"
        || conformance["schema"] != POC_SCHEMA_SOURCE
        || conformance["goldens"].as_array().map(Vec::len) != Some(5)
        || conformance["invalid"] != "artifacts/poc-v1/fixtures/invalid-action.json"
        || conformance["checksums"] != "artifacts/poc-v1/SHA256SUMS"
    {
        return Err(ArtifactError::FixtureMismatch);
    }
    Ok(())
}

/// A deterministic failure while loading the copied artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArtifactError {
    InvalidJson,
    ManifestMismatch,
    SchemaMismatch,
    ChecksumMismatch,
    FixtureMismatch,
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("copied POC artifact is invalid")
    }
}

impl std::error::Error for ArtifactError {}

fn parse(text: &str) -> Result<Value, ArtifactError> {
    serde_json::from_str(text).map_err(|_| ArtifactError::InvalidJson)
}
