// SPDX-License-Identifier: MIT

//! Restricted RFC 8785 canonical encoding for game-owned checkpoint payloads.
//!
//! This is a source-only witness for the `asc-jcs-state-v1` profile. It encodes a
//! bounded game-owned value model; it does not capture native state, prove host
//! compatibility, or implement restore.

mod encode;
mod error;
mod output;
mod parse;

use std::collections::BTreeMap;

pub use encode::{blob_digest, state_id, to_canonical_bytes};
pub use error::CanonicalError;
pub use parse::parse_canonical_text;

/// Largest integer magnitude representable exactly by an IEEE-754 double (2^53 - 1).
pub const CANONICAL_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

/// Pinned profile limit of 64 JSON containers, including tagged numeric objects.
///
/// The limit is enforced before recursive descent so that hostile nesting cannot
/// exhaust the process stack; inputs nested deeper are rejected with
/// [`CanonicalError::DepthExceeded`].
pub const CANONICAL_MAX_DEPTH: usize = 64;

fn validate_key(key: &str) -> Result<(), CanonicalError> {
    if !key.is_ascii() {
        return Err(CanonicalError::NonAsciiKey);
    }
    let mut bytes = key.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(CanonicalError::InvalidKey);
    }
    Ok(())
}

/// Game-owned value model restricted to the canonical exact-state profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalValue {
    /// JSON `null`.
    Null,
    /// JSON boolean.
    Bool(bool),
    /// Safe signed integer in `[-(2^53 - 1), 2^53 - 1]`.
    Integer(i64),
    /// UTF-8 text.
    Text(String),
    /// Ordered JSON array.
    Array(Vec<CanonicalValue>),
    /// JSON object with ordered keys matching `^[a-z][a-z0-9_]*$`.
    Object(BTreeMap<String, CanonicalValue>),
    /// Exact unsigned 64-bit integer encoded as a tagged object.
    Uint64(u64),
    /// Raw IEEE-754 double bits encoded as a tagged object.
    Float64Bits(u64),
}

impl CanonicalValue {
    /// Encodes this value with the restricted canonical profile.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalError`] for invalid values/keys, more than 64 nested
    /// containers, or output exceeding the 16 MiB checkpoint byte limit.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CanonicalError> {
        to_canonical_bytes(self)
    }
}
