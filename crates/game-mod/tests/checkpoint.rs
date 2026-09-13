// SPDX-License-Identifier: MIT

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::error::Error;

use sts2_game_mod::{
    CHECKPOINT_CAPTURE_MAX_BYTES, CHECKPOINT_CAPTURE_MAX_ID_BYTES, CHECKPOINT_CAPTURE_PROFILE,
    CHECKPOINT_CAPTURE_SCHEMA, CHECKPOINT_ID_DOMAIN, CHECKPOINT_STATE_DOMAIN, CheckpointBoundary,
    CheckpointCapabilities, CheckpointCapability, CheckpointCaptureIdentity, CheckpointCapturePort,
    CheckpointCaptureRejection, CheckpointCaptureRequest, CheckpointDurability,
    CheckpointIdentityError, CheckpointUnavailableReason, UnavailableCheckpointCapture,
};

const MANIFEST: &str = include_str!("../../../protocol-artifact/exact-state-v1/manifest.json");
const VECTORS: &str =
    include_str!("../../../protocol-artifact/exact-state-v1/selected-vectors.json");

fn digest(prefix: &str, domain: &[u8], bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    let output = hasher.finalize();
    let mut hex = String::with_capacity(output.len() * 2);
    for byte in output {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("{prefix}{hex}")
}

fn decode_hex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    if !value.len().is_multiple_of(2) {
        return Err("hex value has an odd length".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&value[index..index + 2], 16)
                .map_err(|error| Box::new(error) as Box<dyn Error>)
        })
        .collect()
}

fn positive_vectors() -> Result<Vec<Value>, Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(VECTORS)?;
    Ok(fixture["positive"]
        .as_array()
        .ok_or("positive vectors must be an array")?
        .clone())
}

fn by_name(vectors: &[Value]) -> Result<BTreeMap<String, Value>, Box<dyn Error>> {
    vectors
        .iter()
        .map(|entry| {
            let name = entry["name"]
                .as_str()
                .ok_or("vector name must be a string")?
                .to_owned();
            Ok((name, entry.clone()))
        })
        .collect()
}

fn string_field<'a>(entry: &'a Value, name: &str) -> Result<&'a str, Box<dyn Error>> {
    entry[name]
        .as_str()
        .ok_or_else(|| format!("{name} must be a string").into())
}

fn identity() -> Result<CheckpointCaptureIdentity, Box<dyn Error>> {
    Ok(CheckpointCaptureIdentity::new(
        "instance-1",
        "session-1",
        "lease-1",
        7,
        "run-1",
        "profile-1",
        "capture-1",
    )?)
}

#[test]
fn selected_protocol_vectors_match_the_pinned_witness() -> Result<(), Box<dyn Error>> {
    let manifest: Value = serde_json::from_str(MANIFEST)?;
    assert_eq!(manifest["artifact"], "sts2-protocol/exact-state-v1");
    assert_eq!(manifest["profile"], CHECKPOINT_CAPTURE_PROFILE);
    assert_eq!(manifest["schema"], CHECKPOINT_CAPTURE_SCHEMA);
    assert_eq!(
        manifest["source_revision"],
        "8a2e66f5d2190a0fca7f146dc3508e8d55515ea"
    );
    assert_eq!(
        manifest["source_vectors_sha256"],
        "393dc9bb5bb7fc00672ffba24e78768c242c5d1488e442e5882b68d00e7441fc"
    );
    assert_eq!(manifest["live_runtime"], false);

    let vectors = positive_vectors()?;
    assert_eq!(vectors.len(), 11);
    for entry in &vectors {
        let bytes = decode_hex(string_field(entry, "canonical_hex")?)?;
        assert_eq!(
            digest("sha256:", &[], &bytes),
            string_field(entry, "blob_digest")?,
            "blob digest differs for {}",
            string_field(entry, "name")?
        );
        assert_eq!(
            digest("asc-state:v1:sha256:", CHECKPOINT_STATE_DOMAIN, &bytes),
            string_field(entry, "state_id")?,
            "state digest differs for {}",
            string_field(entry, "name")?
        );
        assert_eq!(
            digest("asc-checkpoint:v1:sha256:", CHECKPOINT_ID_DOMAIN, &bytes),
            string_field(entry, "checkpoint_id")?,
            "checkpoint id differs for {}",
            string_field(entry, "name")?
        );
    }
    Ok(())
}

#[test]
fn equivalence_and_hidden_state_distinctions_remain_pinned() -> Result<(), Box<dyn Error>> {
    let vectors = positive_vectors()?;
    let by_name = by_name(&vectors)?;
    let fixture: Value = serde_json::from_str(VECTORS)?;

    for pair in fixture["equivalence_pairs"]
        .as_array()
        .ok_or("equivalence pairs must be an array")?
    {
        let left = by_name
            .get(pair[0].as_str().ok_or("left name")?)
            .ok_or("left vector")?;
        let right = by_name
            .get(pair[1].as_str().ok_or("right name")?)
            .ok_or("right vector")?;
        assert_eq!(left["canonical_hex"], right["canonical_hex"]);
        assert_eq!(left["state_id"], right["state_id"]);
        assert_eq!(left["checkpoint_id"], right["checkpoint_id"]);
    }

    for pair in fixture["distinct_pairs"]
        .as_array()
        .ok_or("distinct pairs must be an array")?
    {
        let left = by_name
            .get(pair[0].as_str().ok_or("left name")?)
            .ok_or("left vector")?;
        let right = by_name
            .get(pair[1].as_str().ok_or("right name")?)
            .ok_or("right vector")?;
        assert_ne!(left["canonical_hex"], right["canonical_hex"]);
        assert_ne!(left["state_id"], right["state_id"]);
    }
    Ok(())
}

#[test]
fn rejection_vectors_and_capture_capabilities_are_explicitly_fail_closed()
-> Result<(), Box<dyn Error>> {
    let fixture: Value = serde_json::from_str(VECTORS)?;
    let rejects = fixture["reject_raw"]
        .as_array()
        .ok_or("raw rejects must be an array")?;
    let names: Vec<_> = rejects
        .iter()
        .map(|entry| entry["name"].as_str().ok_or("reject name"))
        .collect::<Result<_, _>>()?;
    assert_eq!(
        names,
        [
            "duplicate_keys",
            "float",
            "exponent",
            "negative_zero",
            "unsafe_integer",
            "unicode_key",
            "trailing_text"
        ]
    );

    let capabilities = CheckpointCapabilities;
    for boundary in CheckpointBoundary::all() {
        let capability = capabilities.for_boundary(*boundary);
        assert!(!capability.is_available(), "{}", boundary.code());
        assert_eq!(
            match capability {
                CheckpointCapability::Unavailable { reason } => reason,
            },
            match boundary {
                CheckpointBoundary::SettledMapChoice
                | CheckpointBoundary::StablePlayerTurnCombat
                | CheckpointBoundary::LaterTurnCombat => {
                    CheckpointUnavailableReason::ExactHostEvidenceRequired
                }
                CheckpointBoundary::RewardCardSelection
                | CheckpointBoundary::Event
                | CheckpointBoundary::Shop
                | CheckpointBoundary::Rest => CheckpointUnavailableReason::CoverageInventoryPending,
                CheckpointBoundary::EnemyTurn
                | CheckpointBoundary::Animation
                | CheckpointBoundary::Transition
                | CheckpointBoundary::Unknown => CheckpointUnavailableReason::UnsafeBoundary,
            }
        );
    }

    let mut capture = UnavailableCheckpointCapture;
    let request =
        CheckpointCaptureRequest::new(identity()?, CheckpointBoundary::StablePlayerTurnCombat);
    assert_eq!(
        capture.capture(request),
        Err(CheckpointCaptureRejection::UnsupportedBoundary {
            boundary: CheckpointBoundary::StablePlayerTurnCombat,
            reason: CheckpointUnavailableReason::ExactHostEvidenceRequired,
        })
    );
    let unsafe_request = CheckpointCaptureRequest::new(identity()?, CheckpointBoundary::EnemyTurn);
    assert_eq!(
        capture.capture(unsafe_request),
        Err(CheckpointCaptureRejection::UnsafeBoundary)
    );
    Ok(())
}

#[test]
fn identity_fence_and_private_receipt_are_bounded() -> Result<(), Box<dyn Error>> {
    let identity = identity()?;
    let request =
        CheckpointCaptureRequest::new(identity.clone(), CheckpointBoundary::LaterTurnCombat);
    assert_eq!(request.identity(), &identity);
    assert_eq!(request.boundary(), CheckpointBoundary::LaterTurnCombat);
    assert_eq!(request.identity().operation_id(), "capture-1");
    assert_eq!(request.identity().lease_epoch(), 7);

    let receipt = sts2_game_mod::CheckpointCaptureReceipt::from_validated_canonical_bytes(
        &request,
        CheckpointDurability::InMemory,
        b"{\"a\":1,\"b\":2}".to_vec(),
    )?;
    assert_eq!(receipt.identity(), &identity);
    assert_eq!(receipt.boundary(), CheckpointBoundary::LaterTurnCombat);
    assert_eq!(receipt.durability(), CheckpointDurability::InMemory);
    assert_eq!(receipt.canonical_len(), b"{\"a\":1,\"b\":2}".len());
    assert_eq!(
        receipt.state_digest(),
        "asc-state:v1:sha256:4fef3232f4c1f205363ff2ebefe0cab072f8ae3081ec36545c6cbebba24b5d83"
    );
    assert_eq!(
        receipt.checkpoint_id(),
        "asc-checkpoint:v1:sha256:1327684c8555d59944c7c7ae1b1fb99eabeba1795ce3f72c6521790500809cfe"
    );
    assert_eq!(
        receipt.blob_digest(),
        "sha256:43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
    );

    assert_eq!(
        CheckpointCaptureIdentity::new("", "session", "lease", 1, "run", "profile", "op"),
        Err(CheckpointIdentityError::Empty {
            field: "instance_id"
        })
    );
    let long_id = "x".repeat(CHECKPOINT_CAPTURE_MAX_ID_BYTES + 1);
    assert_eq!(
        CheckpointCaptureIdentity::new(long_id, "session", "lease", 1, "run", "profile", "op"),
        Err(CheckpointIdentityError::TooLong {
            field: "instance_id"
        })
    );
    assert_eq!(
        sts2_game_mod::CheckpointCaptureReceipt::from_validated_canonical_bytes(
            &request,
            CheckpointDurability::InMemory,
            Vec::new()
        ),
        Err(CheckpointCaptureRejection::InvalidCanonicalBytes)
    );
    assert_eq!(
        sts2_game_mod::CheckpointCaptureReceipt::from_validated_canonical_bytes(
            &request,
            CheckpointDurability::InMemory,
            vec![0; CHECKPOINT_CAPTURE_MAX_BYTES + 1]
        ),
        Err(CheckpointCaptureRejection::Oversize {
            bytes: CHECKPOINT_CAPTURE_MAX_BYTES + 1
        })
    );
    Ok(())
}
