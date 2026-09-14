// SPDX-License-Identifier: MIT

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Protocol version carried by every seeded-run message.
pub const PROTOCOL_VERSION: &str = "seeded-run-v1";
/// Shared artifact identity recorded in provenance.
pub const ARTIFACT: &str = "sts2-protocol/seeded-run-v1";
/// Schema source path recorded in provenance.
pub const SCHEMA_SOURCE: &str = "schemas/seeded-run-v1.schema.json";
/// Generator identity recorded in provenance.
pub const GENERATOR: &str = "hand-authored";

/// Failure building or sending a seeded-run request.
#[derive(Debug)]
pub enum ClientError {
    /// A field failed the bounded host contract.
    InvalidField(&'static str),
    /// JSON encoding failed.
    Encode,
    /// The runtime listener could not be reached or returned malformed data.
    Transport(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidField(field) => write!(formatter, "invalid field: {field}"),
            Self::Encode => write!(formatter, "request encoding failed"),
            Self::Transport(detail) => write!(formatter, "transport error: {detail}"),
        }
    }
}

impl std::error::Error for ClientError {}

/// One identity together with its digest.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct IdentityDigest {
    /// Bounded identity token.
    pub identity: String,
    /// Lowercase SHA-256 digest.
    pub digest: String,
}

/// Profile baseline bound into the selected context.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProfileBaseline {
    /// `fresh` or `existing`.
    pub kind: String,
    /// Bounded baseline identity.
    pub identity: String,
    /// Lowercase SHA-256 digest.
    pub digest: String,
}

/// Game and mod compatibility identities.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Compatibility {
    /// Game assembly identity and digest.
    pub game: IdentityDigest,
    /// Mod assembly identity and digest.
    #[serde(rename = "mod")]
    pub mod_identity: IdentityDigest,
}

/// Canonical selected-context fields, hashed without `context_digest`.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct CanonicalContext {
    /// Caller-selected context identity.
    pub context_id: String,
    /// Native game mode (`standard`).
    pub game_mode: String,
    /// Native character (`ironclad`).
    pub character: String,
    /// Ascension level.
    pub ascension: u32,
    /// Sorted modifier set.
    pub modifiers: Vec<String>,
    /// Ordered acts.
    pub acts: Vec<String>,
    /// Selection policy token.
    pub selection_policy: String,
    /// Profile baseline.
    pub profile_baseline: ProfileBaseline,
    /// `disabled` or `enabled`.
    pub save_policy: String,
    /// Compatibility identities.
    pub compatibility: Compatibility,
}

impl CanonicalContext {
    /// Returns the Rust/`seeded-run-v1` compatible SHA-256 over the canonical fields.
    pub fn digest(&self) -> Result<String, ClientError> {
        let bytes = serde_json::to_vec(self).map_err(|_| ClientError::Encode)?;
        Ok(hex_sha256(&bytes))
    }

    /// Validates the bounded host contract before a request is built.
    pub fn validate(&self) -> Result<(), ClientError> {
        if !is_identity(&self.context_id)
            || self.game_mode != "standard"
            || self.character != "ironclad"
            || self.ascension > 20
            || !is_context_text(&self.selection_policy)
            || !matches!(self.save_policy.as_str(), "disabled" | "enabled")
            || self.modifiers.len() > 32
            || self.acts.is_empty()
            || self.acts.len() > 8
            || self.acts.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(ClientError::InvalidField("selected_context"));
        }
        if self.modifiers.iter().any(|value| !is_context_text(value))
            || self.modifiers.windows(2).any(|pair| pair[0] >= pair[1])
            || self.acts.iter().any(|value| !is_context_text(value))
        {
            return Err(ClientError::InvalidField("selected_context_lists"));
        }
        if !matches!(self.profile_baseline.kind.as_str(), "fresh" | "existing")
            || !is_identity(&self.profile_baseline.identity)
            || !is_digest(&self.profile_baseline.digest)
        {
            return Err(ClientError::InvalidField("profile_baseline"));
        }
        for part in [&self.compatibility.game, &self.compatibility.mod_identity] {
            if !is_identity(&part.identity) || !is_digest(&part.digest) {
                return Err(ClientError::InvalidField("compatibility"));
            }
        }
        Ok(())
    }
}

/// Bounded identity grammar for `seeded-run-v1`.
#[must_use]
pub fn is_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".:/_-".contains(&byte))
}

/// Bounded context-text grammar for `seeded-run-v1`.
#[must_use]
pub fn is_context_text(value: &str) -> bool {
    is_identity(value)
}

/// Lowercase SHA-256 digest grammar.
#[must_use]
pub fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Returns the lowercase hex SHA-256 digest of `bytes`.
#[must_use]
pub fn hex_sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}
