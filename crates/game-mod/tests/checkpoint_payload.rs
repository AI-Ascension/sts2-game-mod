// SPDX-License-Identifier: MIT

//! Source-only witness for the closed `checkpoint-payload-v1` contract.
//!
//! Every fixture is synthetic. Nothing here reads a host, proves native
//! coverage, or advertises a phase as capture-capable.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::path::{Path, PathBuf};

use sts2_game_mod::{
    CHECKPOINT_CAPTURE_PROFILE, CHECKPOINT_PAYLOAD_BOUNDARIES, CHECKPOINT_PAYLOAD_SCHEMA,
    CHECKPOINT_PAYLOAD_SCHEMA_DIGEST, CHECKPOINT_PAYLOAD_SCHEMA_SOURCE, CanonicalValue,
    CheckpointBoundary, CheckpointCapabilities, CheckpointCaptureRejection, CheckpointPayload,
    CheckpointPayloadError, PayloadFamilyCoverage, blob_digest, parse_canonical_text, state_id,
};

const SCHEMA: &str = include_str!("../../../schemas/checkpoint-payload-v1.schema.json");
const CASE: &str = include_str!("../../../conformance/cases/checkpoint-payload-v1.json");
const CHECKSUMS: &str =
    include_str!("../../../conformance/fixtures/checkpoint-payload-v1/SHA256SUMS");
const FIXTURES: &str = "conformance/fixtures/checkpoint-payload-v1";
const SETTLED: &str = "valid/settled-map-choice.json";
const LATER: &str = "valid/later-turn-combat.json";
const LATER_ADVANCED: &str = "valid/later-turn-combat-rng-cursor-advanced.json";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> Result<String, Box<dyn Error>> {
    Ok(std::fs::read_to_string(repo_root().join(relative))?)
}

fn fixture(relative: &str) -> Result<String, Box<dyn Error>> {
    read(&format!("{FIXTURES}/{relative}"))
}

fn payload(relative: &str) -> Result<CheckpointPayload, Box<dyn Error>> {
    Ok(CheckpointPayload::parse_canonical_text(&fixture(
        relative,
    )?)?)
}

fn case() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(CASE)?)
}

fn validator() -> Result<jsonschema::Validator, Box<dyn Error>> {
    let schema: Value = serde_json::from_str(SCHEMA)?;
    Ok(jsonschema::draft202012::new(&schema)?)
}

fn schema_valid(text: &str) -> Result<bool, Box<dyn Error>> {
    Ok(validator()?.is_valid(&serde_json::from_str::<Value>(text)?))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |acc, byte| format!("{acc}{byte:02x}"))
}

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
    value[field]
        .as_str()
        .ok_or_else(|| format!("{field} must be a string").into())
}

fn without_rng(value: &CanonicalValue) -> Result<CanonicalValue, Box<dyn Error>> {
    let CanonicalValue::Object(root) = value else {
        return Err("payload must lower to an object".into());
    };
    let mut root = root.clone();
    let CanonicalValue::Object(mut families) = root.remove("families").ok_or("families missing")?
    else {
        return Err("families must be an object".into());
    };
    families
        .remove("seed_and_rng")
        .ok_or("seed_and_rng missing")?;
    root.insert("families".into(), CanonicalValue::Object(families));
    Ok(CanonicalValue::Object(root))
}

fn assert_closed(node: &Value) {
    if let Value::Object(map) = node {
        if map.get("type") == Some(&Value::String("object".into())) {
            assert_eq!(
                map.get("additionalProperties"),
                Some(&Value::Bool(false)),
                "object node is not closed: {node}"
            );
        }
        map.values().for_each(assert_closed);
    } else if let Value::Array(items) = node {
        items.iter().for_each(assert_closed);
    }
}

#[test]
fn settled_map_choice_payload_round_trips_through_canonical_encoder() -> Result<(), Box<dyn Error>>
{
    let original = payload(SETTLED)?;
    assert_eq!(original.boundary(), CheckpointBoundary::SettledMapChoice);
    let bytes = original.to_canonical_bytes()?;
    let reparsed =
        CheckpointPayload::parse_canonical(&parse_canonical_text(std::str::from_utf8(&bytes)?)?)?;
    assert_eq!(reparsed, original);
    assert_eq!(reparsed.to_canonical_bytes()?, bytes);
    assert_eq!(CanonicalValue::from(&original), original.lower());
    let rebuilt = CheckpointPayload::new(original.boundary(), original.families().clone())?;
    assert_eq!(rebuilt.to_canonical_bytes()?, bytes);
    assert_eq!(
        state_id(&bytes),
        "asc-state:v1:sha256:08f93e5abf17b3e091278d65695cb79faf0f1ee5f89e460458ad22520cf4f690"
    );
    let tokens = original.families().coverage_tokens();
    assert_eq!(tokens.len(), 12);
    for (family, token) in tokens {
        let expected = match family {
            "combat_turn" | "powers" | "enemies_and_intents" => "not_applicable",
            _ => "captured",
        };
        assert_eq!(token, expected, "{family}");
    }
    Ok(())
}

#[test]
fn later_turn_combat_payload_changes_identity_when_rng_cursor_changes() -> Result<(), Box<dyn Error>>
{
    let base = payload(LATER)?;
    assert_eq!(base.boundary(), CheckpointBoundary::LaterTurnCombat);
    let mut families = base.families().clone();
    let PayloadFamilyCoverage::Captured(seed) = &mut families.seed_and_rng else {
        return Err("seed_and_rng must be captured".into());
    };
    let stream = seed.streams.first_mut().ok_or("stream missing")?;
    stream.cursor += 1;
    let advanced = CheckpointPayload::new(base.boundary(), families)?;
    let (base_bytes, advanced_bytes) = (base.to_canonical_bytes()?, advanced.to_canonical_bytes()?);
    assert_ne!(base_bytes, advanced_bytes);
    assert_ne!(state_id(&base_bytes), state_id(&advanced_bytes));
    assert_ne!(blob_digest(&base_bytes), blob_digest(&advanced_bytes));
    assert_eq!(without_rng(&base.lower())?, without_rng(&advanced.lower())?);
    assert_eq!(advanced, payload(LATER_ADVANCED)?);
    assert_eq!(
        state_id(&base_bytes),
        "asc-state:v1:sha256:ec6030cb4378705c70d33076ef8aed5aa27b0dd97cb7f1c3a52e45a065fecc0a"
    );
    assert_eq!(
        state_id(&advanced_bytes),
        "asc-state:v1:sha256:78c749738b505a7ddaae805f1f451e1ade1fd69deb0b60242fe54cf3305ca26e"
    );
    let turn = base
        .families()
        .combat_turn
        .value()
        .ok_or("combat_turn must be captured")?;
    assert!(turn.turn >= 2);
    Ok(())
}

#[test]
fn unknown_required_field_rejects_capture_with_unsupported_coverage() -> Result<(), Box<dyn Error>>
{
    let base = payload(SETTLED)?;
    let mut families = base.families().clone();
    families.relics = PayloadFamilyCoverage::Unknown;
    let error = CheckpointPayload::new(base.boundary(), families.clone())
        .err()
        .ok_or("unknown required family must reject")?;
    assert_eq!(
        error,
        CheckpointPayloadError::RequiredFamilyUnknown { family: "relics" }
    );
    assert_eq!(error.code(), "required_family_unknown");
    assert_eq!(
        error.rejection(),
        CheckpointCaptureRejection::UnsupportedCoverage
    );
    families.relics = PayloadFamilyCoverage::NotApplicable;
    let error = CheckpointPayload::new(base.boundary(), families)
        .err()
        .ok_or("missing required family must reject")?;
    assert_eq!(
        error,
        CheckpointPayloadError::FamilyNotCaptured { family: "relics" }
    );
    assert_eq!(
        error.rejection(),
        CheckpointCaptureRejection::UnsupportedCoverage
    );
    let text = fixture("invalid/required-family-unknown.json")?;
    assert!(!schema_valid(&text)?);
    assert_eq!(
        CheckpointPayload::parse_canonical_text(&text).err(),
        Some(CheckpointPayloadError::RequiredFamilyUnknown { family: "relics" })
    );
    Ok(())
}

#[test]
fn payload_schema_digest_is_pinned_in_manifest() -> Result<(), Box<dyn Error>> {
    let case = case()?;
    assert_eq!(
        sha256_hex(SCHEMA.as_bytes()),
        CHECKPOINT_PAYLOAD_SCHEMA_DIGEST
    );
    assert_eq!(
        string_field(&case, "schema_sha256")?,
        CHECKPOINT_PAYLOAD_SCHEMA_DIGEST
    );
    assert_eq!(
        string_field(&case, "schema")?,
        CHECKPOINT_PAYLOAD_SCHEMA_SOURCE
    );
    assert_eq!(read(CHECKPOINT_PAYLOAD_SCHEMA_SOURCE)?, SCHEMA);
    assert_eq!(
        string_field(&case, "payload_schema")?,
        CHECKPOINT_PAYLOAD_SCHEMA
    );
    assert_eq!(
        string_field(&case, "canonical_profile")?,
        CHECKPOINT_CAPTURE_PROFILE
    );
    assert_eq!(
        string_field(&case, "checksums")?,
        format!("{FIXTURES}/SHA256SUMS")
    );
    assert_eq!(case["setup"]["live_runtime"], false);
    assert_eq!(case["setup"]["proprietary_data"], false);
    let schema: Value = serde_json::from_str(SCHEMA)?;
    assert_eq!(schema["$id"], "sts2-checkpoint-payload-v1");
    assert_eq!(
        schema["properties"]["schema"]["const"],
        CHECKPOINT_PAYLOAD_SCHEMA
    );
    assert_eq!(
        schema["properties"]["canonical_profile"]["const"],
        CHECKPOINT_CAPTURE_PROFILE
    );
    assert_closed(&schema);
    let supported: Vec<&str> = CHECKPOINT_PAYLOAD_BOUNDARIES
        .iter()
        .map(|boundary| boundary.code())
        .collect();
    let case_supported: Vec<&str> = case["supported_boundaries"]
        .as_array()
        .ok_or("supported_boundaries")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(case_supported, supported);
    let schema_boundaries: Vec<&str> = schema["$defs"]["boundary"]["enum"]
        .as_array()
        .ok_or("boundary enum")?
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(schema_boundaries, supported);
    Ok(())
}

#[test]
fn unsupported_phase_has_no_payload_schema() -> Result<(), Box<dyn Error>> {
    let case = case()?;
    let base = payload(SETTLED)?;
    let mut checked = 0;
    for boundary in CheckpointBoundary::all().iter().copied() {
        let capability = CheckpointCapabilities.for_boundary(boundary);
        assert!(!capability.is_available(), "{}", boundary.code());
        if CHECKPOINT_PAYLOAD_BOUNDARIES.contains(&boundary) {
            assert_eq!(
                CheckpointPayload::schema_for(boundary),
                Some(CHECKPOINT_PAYLOAD_SCHEMA)
            );
            continue;
        }
        checked += 1;
        assert_eq!(CheckpointPayload::schema_for(boundary), None);
        let error = CheckpointPayload::new(boundary, base.families().clone())
            .err()
            .ok_or("unsupported boundary must reject")?;
        assert_eq!(
            error,
            CheckpointPayloadError::UnsupportedBoundary { boundary }
        );
        assert_eq!(error.rejection(), capability.rejection(boundary));
        assert_eq!(
            case["unsupported_boundaries"][boundary.code()],
            capability.reason().code()
        );
        let mut document: Value = serde_json::from_str(&fixture(SETTLED)?)?;
        document["boundary"] = Value::String(boundary.code().into());
        assert!(!validator()?.is_valid(&document), "{}", boundary.code());
    }
    assert_eq!(checked, 8);
    assert_eq!(
        case["unsupported_boundaries"].as_object().map(|m| m.len()),
        Some(8)
    );
    let unknown = CheckpointPayload::parse_canonical_text(
        &fixture(SETTLED)?.replace("\"settled_map_choice\"", "\"not_a_phase\""),
    );
    assert_eq!(unknown.err(), Some(CheckpointPayloadError::UnknownBoundary));
    Ok(())
}

#[test]
fn every_conformance_fixture_is_consumed() -> Result<(), Box<dyn Error>> {
    let case = case()?;
    let root = repo_root().join(FIXTURES);
    let mut on_disk = BTreeSet::new();
    for kind in ["valid", "invalid"] {
        for entry in std::fs::read_dir(root.join(kind))? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            on_disk.insert(format!("{FIXTURES}/{kind}/{name}"));
        }
    }
    let mut inventory = BTreeSet::new();
    for line in CHECKSUMS.lines() {
        let (expected, relative) = line.split_once("  ").ok_or("malformed SHA256SUMS")?;
        let bytes = std::fs::read(root.join(relative))?;
        assert_eq!(sha256_hex(&bytes), expected, "{relative}");
        inventory.insert(format!("{FIXTURES}/{relative}"));
    }
    assert_eq!(inventory, on_disk);
    let mut consumed = BTreeSet::new();
    let valid = case["valid_vectors"].as_array().ok_or("valid_vectors")?;
    for vector in valid {
        let relative = string_field(vector, "fixture")?;
        let text = read(relative)?;
        assert!(schema_valid(&text)?, "{relative}");
        let parsed = CheckpointPayload::parse_canonical_text(&text)?;
        assert_eq!(parsed.boundary().code(), string_field(vector, "boundary")?);
        let bytes = parsed.to_canonical_bytes()?;
        assert_eq!(Some(bytes.len() as u64), vector["canonical_bytes"].as_u64());
        assert_eq!(
            state_id(&bytes),
            string_field(vector, "exact_state_digest")?
        );
        assert_eq!(blob_digest(&bytes), string_field(vector, "blob_digest")?);
        consumed.insert(relative.to_owned());
    }
    let invalid = case["invalid_vectors"]
        .as_array()
        .ok_or("invalid_vectors")?;
    for vector in invalid {
        let relative = string_field(vector, "fixture")?;
        let text = read(relative)?;
        assert_eq!(
            schema_valid(&text)?,
            vector["schema_valid"].as_bool().ok_or("schema_valid")?,
            "{relative}"
        );
        let error = CheckpointPayload::parse_canonical_text(&text)
            .err()
            .ok_or_else(|| format!("{relative} must be rejected"))?;
        assert_eq!(
            error.code(),
            string_field(vector, "expected_error")?,
            "{relative}"
        );
        assert_eq!(
            error.rejection().code(),
            string_field(vector, "expected_rejection")?,
            "{relative}"
        );
        consumed.insert(relative.to_owned());
    }
    assert_eq!(consumed, on_disk);
    assert_eq!(valid.len(), 4);
    assert_eq!(invalid.len(), 9);
    Ok(())
}

#[test]
fn payload_debug_output_omits_seed_and_rng_values() -> Result<(), Box<dyn Error>> {
    let later = payload(LATER)?;
    let rendered = format!("{later:?}");
    assert!(rendered.contains("later_turn_combat"));
    assert!(rendered.contains("seed_and_rng: \"captured\""));
    assert!(!rendered.contains("SYNTHETIC-SEED"));
    assert!(!rendered.contains("18446744073709551615"));
    assert!(!rendered.contains("cursor"));
    Ok(())
}
