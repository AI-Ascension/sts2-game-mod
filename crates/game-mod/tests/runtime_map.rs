// SPDX-License-Identifier: MIT

use std::{error::Error, path::Path, process::Command};

use serde_json::Value;
use sts2_game_mod::{
    RUNTIME_MAP_V1_ARTIFACT, RUNTIME_MAP_V1_GENERATOR, RUNTIME_MAP_V1_PROTOCOL_VERSION,
    RUNTIME_MAP_V1_SCHEMA_DIGEST, RUNTIME_MAP_V1_SCHEMA_SOURCE, verify_runtime_map_artifact,
};

const SOURCE_SCHEMA: &str = include_str!("../../../schemas/runtime-map-v1.schema.json");
const PACKAGE_SCHEMA: &str = include_str!("../../../protocol-artifact/runtime-map-v1/schema.json");
const MANIFEST: &str = include_str!("../../../protocol-artifact/runtime-map-v1/manifest.json");
const CONFORMANCE: &str = include_str!("../../../conformance/cases/runtime-map-v1.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/runtime-map-v1/SHA256SUMS");
const REQUEST: &str =
    include_str!("../../../protocol-artifact/runtime-map-v1/golden/snapshot-request.json");
const RESPONSE: &str =
    include_str!("../../../protocol-artifact/runtime-map-v1/golden/snapshot-response.json");
const VISIBLE_MAP: &str =
    include_str!("../../../protocol-artifact/runtime-map-v1/golden/visible-map.json");

#[test]
fn copied_runtime_map_artifact_has_exact_bytes_and_provenance() -> Result<(), Box<dyn Error>> {
    assert_eq!(verify_runtime_map_artifact(), Ok(()));
    assert_eq!(SOURCE_SCHEMA, PACKAGE_SCHEMA);
    assert_eq!(
        MANIFEST.parse::<Value>()?["artifact"],
        RUNTIME_MAP_V1_ARTIFACT
    );
    assert_eq!(
        MANIFEST.parse::<Value>()?["protocol_version"],
        RUNTIME_MAP_V1_PROTOCOL_VERSION
    );
    assert_eq!(
        MANIFEST.parse::<Value>()?["schema_digest"],
        RUNTIME_MAP_V1_SCHEMA_DIGEST
    );
    assert_eq!(
        MANIFEST.parse::<Value>()?["provenance"]["source"],
        RUNTIME_MAP_V1_SCHEMA_SOURCE
    );
    assert_eq!(
        MANIFEST.parse::<Value>()?["provenance"]["generator"],
        RUNTIME_MAP_V1_GENERATOR
    );
    assert_eq!(
        serde_json::from_str::<Value>(CONFORMANCE)?["schema"],
        RUNTIME_MAP_V1_SCHEMA_SOURCE
    );
    assert_eq!(
        serde_json::from_str::<Value>(RESPONSE)?["snapshot"],
        serde_json::from_str::<Value>(VISIBLE_MAP)?
    );
    assert_eq!(
        serde_json::from_str::<Value>(REQUEST)?["schema_digest"],
        RUNTIME_MAP_V1_SCHEMA_DIGEST
    );
    assert!(
        !CHECKSUMS.contains("6340f3cbe6c1b5728144fe89fdfdf8645acf2f59a77c0e0c30ebfeafc77515d8")
    );

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../protocol-artifact/runtime-map-v1");
    let output = Command::new("sha256sum")
        .args(["--check", "--strict", "SHA256SUMS"])
        .current_dir(&root)
        .output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8(output.stdout)?.lines().count(), 7);
    Ok(())
}
