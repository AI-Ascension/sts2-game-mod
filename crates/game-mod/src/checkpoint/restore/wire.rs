// SPDX-License-Identifier: MIT

use std::sync::OnceLock;

use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};

use super::{EXACT_RESTORE_MAX_FRAME_BYTES, EXACT_RESTORE_SCHEMA_DIGEST, SCHEMA_JSON};

pub(super) struct RequestFrame {
    pub(super) kind: String,
    pub(super) payload: Value,
    pub(super) operation_id: String,
    pub(super) expected_owner: Value,
    pub(super) message_id: String,
    pub(super) correlation_id: String,
    pub(super) request_digest: String,
}

pub(super) fn decode_request(bytes: &[u8]) -> Result<RequestFrame, WireError> {
    if bytes.is_empty() || bytes.len() > EXACT_RESTORE_MAX_FRAME_BYTES {
        return Err(WireError::FrameSize);
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(WireError::NonCanonical);
    }
    let value = parse_unique_value(bytes)?;
    validate_schema(&value)?;
    let canonical = canonical_json(&value)?;
    if canonical.as_slice() != bytes {
        return Err(WireError::NonCanonical);
    }
    if value["schema_digest"].as_str() != Some(EXACT_RESTORE_SCHEMA_DIGEST)
        || !value["kind"]
            .as_str()
            .is_some_and(|kind| kind.ends_with("_request"))
    {
        return Err(WireError::UnsupportedContract);
    }
    let operation_id = value["payload"]["operation_id"]
        .as_str()
        .ok_or(WireError::InvalidFrame)?
        .to_owned();
    let expected_owner = value["payload"]["expected_owner"].clone();
    let payload = value["payload"].clone();
    let kind = value["kind"]
        .as_str()
        .ok_or(WireError::InvalidFrame)?
        .to_owned();
    let message_id = value["message_id"]
        .as_str()
        .ok_or(WireError::InvalidFrame)?
        .to_owned();
    let correlation_id = value["correlation_id"]
        .as_str()
        .ok_or(WireError::InvalidFrame)?
        .to_owned();
    let request_digest = digest(&canonical);
    Ok(RequestFrame {
        kind,
        payload,
        operation_id,
        expected_owner,
        message_id,
        correlation_id,
        request_digest,
    })
}

pub(super) fn parse_unique_value(bytes: &[u8]) -> Result<Value, WireError> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let UniqueValue(value) =
        UniqueValue::deserialize(&mut deserializer).map_err(|_| WireError::InvalidJson)?;
    deserializer.end().map_err(|_| WireError::InvalidJson)?;
    Ok(value)
}

pub(super) fn encode_frame(value: &Value) -> Result<Vec<u8>, WireError> {
    validate_schema(value)?;
    let bytes = canonical_json(value)?;
    if bytes.len() > EXACT_RESTORE_MAX_FRAME_BYTES {
        return Err(WireError::FrameSize);
    }
    Ok(bytes)
}

pub(super) fn canonical_json(value: &Value) -> Result<Vec<u8>, WireError> {
    serde_json::to_vec(value).map_err(|_| WireError::InvalidFrame)
}

pub(super) fn digest(bytes: &[u8]) -> String {
    let value = Sha256::digest(bytes);
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in value {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

pub(super) fn response_base(
    kind: &str,
    correlation_id: &str,
    payload: Value,
) -> Result<Value, WireError> {
    let message_id = uuid_v4()?;
    Ok(serde_json::json!({
        "contract": "sts2-exact-restore-v1",
        "schema_digest": EXACT_RESTORE_SCHEMA_DIGEST,
        "message_id": message_id,
        "correlation_id": correlation_id,
        "kind": kind,
        "payload": payload
    }))
}

pub(super) fn uuid_v4() -> Result<String, WireError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| WireError::RandomUnavailable)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Ok(format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    ))
}

fn validate_schema(value: &Value) -> Result<(), WireError> {
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, String>> = OnceLock::new();
    let validator = VALIDATOR.get_or_init(|| {
        let schema: Value =
            serde_json::from_str(SCHEMA_JSON).map_err(|_| String::from("schema_json"))?;
        jsonschema::draft202012::new(&schema).map_err(|_| String::from("schema_compile"))
    });
    let validator = validator
        .as_ref()
        .map_err(|_| WireError::SchemaUnavailable)?;
    if validator.is_valid(value) {
        Ok(())
    } else {
        Err(WireError::InvalidFrame)
    }
}

#[derive(Debug)]
pub(super) enum WireError {
    FrameSize,
    InvalidJson,
    InvalidFrame,
    NonCanonical,
    UnsupportedContract,
    SchemaUnavailable,
    RandomUnavailable,
}

struct UniqueValue(Value);

impl<'de> Deserialize<'de> for UniqueValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueVisitor)
    }
}

struct UniqueVisitor;

impl<'de> Visitor<'de> for UniqueVisitor {
    type Value = UniqueValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(Number::from(value))))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Number(Number::from(value))))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(|number| UniqueValue(Value::Number(number)))
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueValue(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(UniqueValue(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(UniqueValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = Map::new();
        while let Some((key, UniqueValue(value))) = map.next_entry::<String, UniqueValue>()? {
            if values.insert(key, value).is_some() {
                return Err(serde::de::Error::custom("duplicate JSON member"));
            }
        }
        Ok(UniqueValue(Value::Object(values)))
    }
}
