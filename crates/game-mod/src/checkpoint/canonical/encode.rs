// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use super::super::{BLOB_DIGEST_PREFIX, CHECKPOINT_STATE_DOMAIN, STATE_DIGEST_PREFIX};
use super::{CANONICAL_MAX_DEPTH, CANONICAL_MAX_SAFE_INTEGER, CanonicalError, CanonicalValue};

/// Encodes a value with deterministic object-key ordering and JCS escaping.
///
/// # Errors
///
/// Returns [`CanonicalError`] for an unsafe integer, a non-ASCII object key, or
/// nesting deeper than [`CANONICAL_MAX_DEPTH`].
pub fn to_canonical_bytes(value: &CanonicalValue) -> Result<Vec<u8>, CanonicalError> {
    let mut output = String::new();
    write_value(&mut output, value, 0)?;
    Ok(output.into_bytes())
}

fn guard_depth(depth: usize) -> Result<(), CanonicalError> {
    if depth >= CANONICAL_MAX_DEPTH {
        Err(CanonicalError::DepthExceeded)
    } else {
        Ok(())
    }
}

fn write_value(
    output: &mut String,
    value: &CanonicalValue,
    depth: usize,
) -> Result<(), CanonicalError> {
    match value {
        CanonicalValue::Null => output.push_str("null"),
        CanonicalValue::Bool(true) => output.push_str("true"),
        CanonicalValue::Bool(false) => output.push_str("false"),
        CanonicalValue::Integer(integer) => {
            if integer.unsigned_abs() > CANONICAL_MAX_SAFE_INTEGER as u64 {
                return Err(CanonicalError::UnsafeInteger);
            }
            output.push_str(&integer.to_string());
        }
        CanonicalValue::Text(text) => write_text(output, text),
        CanonicalValue::Array(items) => {
            guard_depth(depth)?;
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(output, item, depth + 1)?;
            }
            output.push(']');
        }
        CanonicalValue::Object(entries) => write_object(output, entries, depth)?,
        CanonicalValue::Uint64(number) => {
            output.push_str("{\"kind\":\"uint64\",\"value\":\"");
            output.push_str(&number.to_string());
            output.push_str("\"}");
        }
        CanonicalValue::Float64Bits(bits) => {
            output.push_str("{\"kind\":\"float64_bits\",\"value\":\"");
            output.push_str(&format!("{bits:016x}"));
            output.push_str("\"}");
        }
    }
    Ok(())
}

fn write_object(
    output: &mut String,
    entries: &BTreeMap<String, CanonicalValue>,
    depth: usize,
) -> Result<(), CanonicalError> {
    guard_depth(depth)?;
    if entries.keys().any(|key| !key.is_ascii()) {
        return Err(CanonicalError::NonAsciiKey);
    }
    output.push('{');
    for (index, (key, value)) in entries.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        write_text(output, key);
        output.push(':');
        write_value(output, value, depth + 1)?;
    }
    output.push('}');
    Ok(())
}

fn write_text(output: &mut String, text: &str) {
    output.push('"');
    for character in text.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{0008}' => output.push_str("\\b"),
            '\u{000c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if (control as u32) < 0x20 => {
                output.push_str(&format!("\\u{:04x}", control as u32));
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

/// Returns the domain-separated exact-state identity over canonical bytes.
#[must_use]
pub fn state_id(bytes: &[u8]) -> String {
    digest(STATE_DIGEST_PREFIX, CHECKPOINT_STATE_DOMAIN, bytes)
}

/// Returns the blob digest over canonical bytes.
#[must_use]
pub fn blob_digest(bytes: &[u8]) -> String {
    digest(BLOB_DIGEST_PREFIX, &[], bytes)
}

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
