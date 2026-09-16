// SPDX-License-Identifier: MIT

use serde_json::Value;
use std::collections::BTreeMap;
use std::error::Error;

use sts2_game_mod::{
    CANONICAL_MAX_SAFE_INTEGER, CanonicalError, CanonicalValue, CheckpointArtifactDescriptor,
    CheckpointManifest, CheckpointManifestBoundary, CheckpointManifestParts, CheckpointOrigin,
    blob_digest, parse_canonical_text, state_id, to_canonical_bytes,
};

const VECTORS: &str =
    include_str!("../../../protocol-artifact/exact-state-v1/selected-vectors.json");
const GOLDEN_MANIFEST: &str =
    include_str!("../../../protocol-artifact/exact-state-v1/golden-manifest.json");
const MANIFEST: &str = include_str!("../../../protocol-artifact/exact-state-v1/manifest.json");
const CHECKSUMS: &str = include_str!("../../../protocol-artifact/exact-state-v1/SHA256SUMS");

fn fixture() -> Result<Value, Box<dyn Error>> {
    Ok(serde_json::from_str(VECTORS)?)
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

fn string_field<'a>(value: &'a Value, field: &str) -> Result<&'a str, Box<dyn Error>> {
    value[field]
        .as_str()
        .ok_or_else(|| format!("{field} must be a string").into())
}

fn tagged_number(
    map: &serde_json::Map<String, Value>,
) -> Result<Option<CanonicalValue>, Box<dyn Error>> {
    if map.len() != 2 {
        return Ok(None);
    }
    let (Some(kind), Some(raw)) = (map.get("kind"), map.get("value")) else {
        return Ok(None);
    };
    let (Some(kind), Some(raw)) = (kind.as_str(), raw.as_str()) else {
        return Ok(None);
    };
    match kind {
        "uint64" => Ok(Some(CanonicalValue::Uint64(raw.parse::<u64>()?))),
        "float64_bits" => {
            if raw.len() != 16 {
                return Err("float64_bits must contain 16 hex digits".into());
            }
            Ok(Some(CanonicalValue::Float64Bits(u64::from_str_radix(
                raw, 16,
            )?)))
        }
        _ => Ok(None),
    }
}

fn typed(value: &Value) -> Result<CanonicalValue, Box<dyn Error>> {
    Ok(match value {
        Value::Null => CanonicalValue::Null,
        Value::Bool(flag) => CanonicalValue::Bool(*flag),
        Value::Number(number) => {
            let integer = number.as_i64().ok_or("number is not a safe integer")?;
            if integer.unsigned_abs() > CANONICAL_MAX_SAFE_INTEGER as u64 {
                return Err("number exceeds the safe integer range".into());
            }
            CanonicalValue::Integer(integer)
        }
        Value::String(text) => CanonicalValue::Text(text.clone()),
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(typed(item)?);
            }
            CanonicalValue::Array(out)
        }
        Value::Object(map) => {
            if let Some(number) = tagged_number(map)? {
                number
            } else {
                let mut out = BTreeMap::new();
                for (key, item) in map {
                    out.insert(key.clone(), typed(item)?);
                }
                CanonicalValue::Object(out)
            }
        }
    })
}

fn canonical_of(entry: &Value) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(to_canonical_bytes(&typed(&entry["value"])?)?)
}

fn pair_name(pair: &Value, index: usize) -> Result<&str, Box<dyn Error>> {
    pair.as_array()
        .and_then(|names| names.get(index))
        .and_then(Value::as_str)
        .ok_or_else(|| "pair must contain two names".into())
}

#[test]
fn positive_vectors_match_pinned_canonical_bytes_and_identities() -> Result<(), Box<dyn Error>> {
    assert_eq!(CANONICAL_MAX_SAFE_INTEGER, 9_007_199_254_740_991);
    let fixture = fixture()?;
    assert_eq!(fixture["profile"], "asc-jcs-state-v1");
    assert_eq!(
        fixture["source_revision"],
        "8a2e66f5d2190a0fca7f146dc3508e8d55515ea7"
    );

    let positives = fixture["positive"]
        .as_array()
        .ok_or("positive must be an array")?;
    assert_eq!(positives.len(), 26);
    for entry in positives {
        let name = string_field(entry, "name")?;
        let expected = decode_hex(string_field(entry, "canonical_hex")?)?;

        assert_eq!(
            to_canonical_bytes(&typed(&entry["value"])?)?,
            expected,
            "typed encoding differs for {name}"
        );
        let raw = serde_json::to_string(&entry["value"])?;
        assert_eq!(
            to_canonical_bytes(&parse_canonical_text(&raw)?)?,
            expected,
            "strict parse encoding differs for {name}"
        );
        assert_eq!(
            state_id(&expected),
            string_field(entry, "state_id")?,
            "state identity differs for {name}"
        );
        assert_eq!(
            blob_digest(&expected),
            string_field(entry, "blob_digest")?,
            "blob digest differs for {name}"
        );
    }
    Ok(())
}

#[test]
fn equivalence_and_distinctness_hold_under_canonical_encoding() -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let positives = fixture["positive"]
        .as_array()
        .ok_or("positive must be an array")?;
    let mut by_name = BTreeMap::new();
    for entry in positives {
        by_name.insert(string_field(entry, "name")?.to_owned(), entry.clone());
    }

    let equivalence_pairs = fixture["equivalence_pairs"]
        .as_array()
        .ok_or("equivalence_pairs must be an array")?;
    assert_eq!(equivalence_pairs.len(), 2);
    for pair in equivalence_pairs {
        let left = by_name
            .get(pair_name(pair, 0)?)
            .ok_or("left vector is missing")?;
        let right = by_name
            .get(pair_name(pair, 1)?)
            .ok_or("right vector is missing")?;
        assert_eq!(canonical_of(left)?, canonical_of(right)?);
        assert_eq!(
            state_id(&canonical_of(left)?),
            state_id(&canonical_of(right)?)
        );
    }

    let distinct_pairs = fixture["distinct_pairs"]
        .as_array()
        .ok_or("distinct_pairs must be an array")?;
    assert_eq!(distinct_pairs.len(), 8);
    for pair in distinct_pairs {
        let left = by_name
            .get(pair_name(pair, 0)?)
            .ok_or("left vector is missing")?;
        let right = by_name
            .get(pair_name(pair, 1)?)
            .ok_or("right vector is missing")?;
        assert_ne!(canonical_of(left)?, canonical_of(right)?);
        assert_ne!(
            state_id(&canonical_of(left)?),
            state_id(&canonical_of(right)?)
        );
    }
    Ok(())
}

#[test]
fn strict_parser_rejects_every_pinned_raw_rejection() -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let rejects = fixture["reject_raw"]
        .as_array()
        .ok_or("reject_raw must be an array")?;
    assert_eq!(rejects.len(), 14);
    for entry in rejects {
        let name = string_field(entry, "name")?;
        let text = string_field(entry, "text")?;
        let expected = match name {
            "duplicate_keys" => CanonicalError::DuplicateKey,
            "float" => CanonicalError::FloatNotAllowed,
            "exponent" => CanonicalError::ExponentNotAllowed,
            "negative_zero" => CanonicalError::NegativeZero,
            "unsafe_integer" => CanonicalError::UnsafeInteger,
            "unicode_key" => CanonicalError::NonAsciiKey,
            "uppercase_key" | "dash_key" => CanonicalError::InvalidKey,
            "nan" | "infinity" => CanonicalError::UnexpectedToken { offset: 9 },
            "surrogate" => CanonicalError::InvalidUnicodeEscape { offset: 9 },
            "trailing_comma" => CanonicalError::UnexpectedToken { offset: 7 },
            "leading_bom" => CanonicalError::UnexpectedToken { offset: 0 },
            "trailing_text" => CanonicalError::TrailingInput,
            other => return Err(format!("unexpected rejection vector {other}").into()),
        };
        assert_eq!(
            parse_canonical_text(text),
            Err(expected),
            "rejection differs for {name}"
        );
    }
    Ok(())
}

fn descriptor(value: &Value) -> Result<CheckpointArtifactDescriptor, Box<dyn Error>> {
    Ok(CheckpointArtifactDescriptor::new(
        string_field(value, "codec")?,
        string_field(value, "digest")?,
        string_field(value, "role")?,
        value["size_bytes"]
            .as_u64()
            .ok_or("size_bytes must be a u64")?,
    )?)
}

fn golden_manifest() -> Result<CheckpointManifest, Box<dyn Error>> {
    let golden: Value = serde_json::from_str(GOLDEN_MANIFEST)?;
    let boundary = &golden["boundary"];
    let origin = &golden["origin"];
    let restore = golden["restore_artifacts"]
        .as_array()
        .ok_or("restore_artifacts must be an array")?;
    let mut restore_artifacts = Vec::with_capacity(restore.len());
    for artifact in restore {
        restore_artifacts.push(descriptor(artifact)?);
    }
    Ok(CheckpointManifest::from_parts(CheckpointManifestParts {
        exact_state_digest: string_field(&golden, "exact_state_digest")?.to_owned(),
        canonical_payload: descriptor(&golden["canonical_payload"])?,
        restore_artifacts,
        compatibility_digest: string_field(&golden, "compatibility_digest")?.to_owned(),
        coverage_contract_digest: string_field(&golden, "coverage_contract_digest")?.to_owned(),
        boundary: CheckpointManifestBoundary::new(
            string_field(boundary, "kind")?,
            string_field(boundary, "phase")?,
            boundary.get("game_tick").and_then(Value::as_u64),
        )?,
        origin: CheckpointOrigin::new(
            string_field(origin, "run_id")?,
            origin["generation"]
                .as_u64()
                .ok_or("generation must be a u64")?,
        )?,
        parent_checkpoint_id: golden["parent_checkpoint_id"].as_str().map(str::to_owned),
    })?)
}

#[test]
fn manifest_derived_checkpoint_identity_remains_pinned() -> Result<(), Box<dyn Error>> {
    let manifest = golden_manifest()?;
    assert_eq!(
        manifest.to_canonical_bytes()?,
        GOLDEN_MANIFEST.trim_end_matches('\n').as_bytes()
    );
    assert_eq!(
        manifest.exact_checkpoint_id()?,
        "asc-checkpoint:v1:sha256:e64d4b9eb2fd555f50a1fde40454ae1f036484adfe48bd8ccadb46bb7e708b1f"
    );
    Ok(())
}

#[test]
fn absent_null_empty_and_unknown_stay_distinct() -> Result<(), Box<dyn Error>> {
    let fixture = fixture()?;
    let positives = fixture["positive"]
        .as_array()
        .ok_or("positive must be an array")?;
    let mut by_name = BTreeMap::new();
    for entry in positives {
        by_name.insert(string_field(entry, "name")?.to_owned(), entry.clone());
    }
    let pairs = fixture["distinct_pairs"]
        .as_array()
        .ok_or("distinct_pairs must be an array")?;
    for (left, right) in [
        ("unicode_nfc", "unicode_nfd"),
        ("null_value", "absent_value"),
        ("false_value", "zero_value"),
        ("array_ab", "array_ba"),
    ] {
        let left_vector = by_name.get(left).ok_or("guarantee vector is missing")?;
        let right_vector = by_name.get(right).ok_or("guarantee vector is missing")?;
        assert!(
            pairs
                .iter()
                .any(|pair| pair[0].as_str() == Some(left) && pair[1].as_str() == Some(right)),
            "protocol guarantee pair {left}/{right} is not pinned as distinct"
        );
        assert_ne!(
            canonical_of(left_vector)?,
            canonical_of(right_vector)?,
            "protocol guarantee pair {left}/{right} is not distinct"
        );
    }
    Ok(())
}

#[test]
fn artifact_inventory_pins_every_checked_in_file() -> Result<(), Box<dyn Error>> {
    let mut pinned = BTreeMap::new();
    for line in CHECKSUMS.lines() {
        let (hex, path) = line
            .split_once("  ")
            .ok_or("inventory line must be '<hex>  <path>'")?;
        assert_eq!(hex.len(), 64, "inventory digest must be 64 hex digits");
        assert!(
            hex.chars().all(|digit| digit.is_ascii_hexdigit()),
            "inventory digest must be hex for {path}"
        );
        assert!(
            pinned.insert(path.to_owned(), hex.to_owned()).is_none(),
            "duplicate inventory entry for {path}"
        );
    }
    for (path, text) in [
        ("manifest.json", MANIFEST),
        ("selected-vectors.json", VECTORS),
        ("golden-manifest.json", GOLDEN_MANIFEST),
    ] {
        let expected = pinned
            .remove(path)
            .ok_or("inventory is missing a pinned file")?;
        let digest = blob_digest(text.as_bytes());
        let actual = digest.strip_prefix("sha256:").ok_or("blob digest prefix")?;
        assert_eq!(actual, expected, "inventory digest differs for {path}");
    }
    assert!(
        pinned.is_empty(),
        "inventory covers a file that is not pinned"
    );
    Ok(())
}
