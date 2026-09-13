// SPDX-License-Identifier: MIT

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{
    BLOB_DIGEST_PREFIX, CHECKPOINT_CAPTURE_MAX_ID_BYTES, CHECKPOINT_ID_DOMAIN,
    CHECKPOINT_ID_PREFIX, STATE_DIGEST_PREFIX,
};

/// Schema identifier for the immutable checkpoint-manifest envelope.
pub const CHECKPOINT_MANIFEST_SCHEMA: &str = "ascension.checkpoint_manifest.v1";
/// Canonical profile used by the manifest envelope.
pub const CHECKPOINT_MANIFEST_PROFILE: &str = "asc-jcs-state-v1";
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// A manifest reference to one immutable artifact blob.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckpointArtifactDescriptor {
    codec: String,
    digest: String,
    role: String,
    size_bytes: u64,
}

impl CheckpointArtifactDescriptor {
    /// Creates a bounded blob descriptor.
    pub fn new(
        codec: impl Into<String>,
        digest: impl Into<String>,
        role: impl Into<String>,
        size_bytes: u64,
    ) -> Result<Self, CheckpointManifestError> {
        let descriptor = Self {
            codec: codec.into(),
            digest: digest.into(),
            role: role.into(),
            size_bytes,
        };
        validate_text("codec", &descriptor.codec)?;
        validate_digest("digest", &descriptor.digest, BLOB_DIGEST_PREFIX)?;
        validate_text("role", &descriptor.role)?;
        validate_safe_integer("size_bytes", descriptor.size_bytes)?;
        Ok(descriptor)
    }

    /// Returns the codec identifier.
    #[must_use]
    pub fn codec(&self) -> &str {
        &self.codec
    }

    /// Returns the blob digest.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    /// Returns the semantic artifact role.
    #[must_use]
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Returns the declared byte size.
    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }
}

/// Boundary metadata included in the checkpoint-manifest identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckpointManifestBoundary {
    #[serde(skip_serializing_if = "Option::is_none")]
    game_tick: Option<u64>,
    kind: String,
    phase: String,
}

impl CheckpointManifestBoundary {
    /// Creates a bounded boundary descriptor.
    pub fn new(
        kind: impl Into<String>,
        phase: impl Into<String>,
        game_tick: Option<u64>,
    ) -> Result<Self, CheckpointManifestError> {
        let boundary = Self {
            kind: kind.into(),
            phase: phase.into(),
            game_tick,
        };
        validate_text("boundary.kind", &boundary.kind)?;
        validate_text("boundary.phase", &boundary.phase)?;
        if let Some(game_tick) = boundary.game_tick {
            validate_safe_integer("boundary.game_tick", game_tick)?;
        }
        Ok(boundary)
    }
}

/// Run-origin metadata included in the checkpoint-manifest identity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckpointOrigin {
    generation: u64,
    run_id: String,
}

impl CheckpointOrigin {
    /// Creates a bounded run-origin descriptor.
    pub fn new(
        run_id: impl Into<String>,
        generation: u64,
    ) -> Result<Self, CheckpointManifestError> {
        let origin = Self {
            generation,
            run_id: run_id.into(),
        };
        validate_safe_integer("origin.generation", origin.generation)?;
        validate_text("origin.run_id", &origin.run_id)?;
        Ok(origin)
    }
}

/// Canonical checkpoint-manifest envelope whose identity includes closure references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct CheckpointManifest {
    boundary: CheckpointManifestBoundary,
    canonical_payload: CheckpointArtifactDescriptor,
    canonical_profile: &'static str,
    compatibility_digest: String,
    coverage_contract_digest: String,
    exact_state_digest: String,
    origin: CheckpointOrigin,
    parent_checkpoint_id: Option<String>,
    restore_artifacts: Vec<CheckpointArtifactDescriptor>,
    schema: &'static str,
}

/// Inputs for constructing a checkpoint manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointManifestParts {
    /// Exact-state digest referenced by the manifest.
    pub exact_state_digest: String,
    /// Canonical exact-state payload descriptor.
    pub canonical_payload: CheckpointArtifactDescriptor,
    /// Restore-closure descriptors in producer order.
    pub restore_artifacts: Vec<CheckpointArtifactDescriptor>,
    /// Compatibility contract digest.
    pub compatibility_digest: String,
    /// Coverage contract digest.
    pub coverage_contract_digest: String,
    /// Admitted boundary metadata.
    pub boundary: CheckpointManifestBoundary,
    /// Run-origin metadata.
    pub origin: CheckpointOrigin,
    /// Optional parent checkpoint identity.
    pub parent_checkpoint_id: Option<String>,
}

impl CheckpointManifest {
    /// Creates a manifest after validating every required closure reference.
    pub fn from_parts(parts: CheckpointManifestParts) -> Result<Self, CheckpointManifestError> {
        if parts.restore_artifacts.is_empty() {
            return Err(CheckpointManifestError::NoRestoreArtifacts);
        }
        let manifest = Self {
            boundary: parts.boundary,
            canonical_payload: parts.canonical_payload,
            canonical_profile: CHECKPOINT_MANIFEST_PROFILE,
            compatibility_digest: parts.compatibility_digest,
            coverage_contract_digest: parts.coverage_contract_digest,
            exact_state_digest: parts.exact_state_digest,
            origin: parts.origin,
            parent_checkpoint_id: parts.parent_checkpoint_id,
            restore_artifacts: parts.restore_artifacts,
            schema: CHECKPOINT_MANIFEST_SCHEMA,
        };
        validate_digest(
            "exact_state_digest",
            &manifest.exact_state_digest,
            STATE_DIGEST_PREFIX,
        )?;
        validate_digest(
            "compatibility_digest",
            &manifest.compatibility_digest,
            BLOB_DIGEST_PREFIX,
        )?;
        validate_digest(
            "coverage_contract_digest",
            &manifest.coverage_contract_digest,
            BLOB_DIGEST_PREFIX,
        )?;
        if let Some(parent_checkpoint_id) = &manifest.parent_checkpoint_id {
            validate_digest(
                "parent_checkpoint_id",
                parent_checkpoint_id,
                CHECKPOINT_ID_PREFIX,
            )?;
        }
        Ok(manifest)
    }

    /// Returns the exact-state digest referenced by the manifest.
    #[must_use]
    pub fn exact_state_digest(&self) -> &str {
        &self.exact_state_digest
    }

    /// Returns the canonical payload descriptor.
    #[must_use]
    pub const fn canonical_payload(&self) -> &CheckpointArtifactDescriptor {
        &self.canonical_payload
    }

    /// Returns the restore-closure descriptors in stable producer order.
    #[must_use]
    pub fn restore_artifacts(&self) -> &[CheckpointArtifactDescriptor] {
        &self.restore_artifacts
    }

    /// Returns canonical manifest bytes with no trailing newline.
    pub fn to_canonical_bytes(&self) -> Result<Vec<u8>, CheckpointManifestError> {
        serde_json::to_vec(self).map_err(|_| CheckpointManifestError::Serialization)
    }

    /// Computes the checkpoint identity over the canonical manifest envelope.
    pub fn exact_checkpoint_id(&self) -> Result<String, CheckpointManifestError> {
        let bytes = self.to_canonical_bytes()?;
        Ok(domain_digest(
            CHECKPOINT_ID_PREFIX,
            CHECKPOINT_ID_DOMAIN,
            &bytes,
        ))
    }
}

/// Rejection while constructing a checkpoint-manifest envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointManifestError {
    /// A required text field is empty.
    Empty { field: &'static str },
    /// A text field exceeds the owner bound.
    TooLong { field: &'static str },
    /// A digest has the wrong namespace or hexadecimal body.
    InvalidDigest { field: &'static str },
    /// The manifest has no restore closure.
    NoRestoreArtifacts,
    /// A numeric value exceeds the profile's safe range.
    UnsafeInteger { field: &'static str },
    /// The canonical JSON serializer failed.
    Serialization,
}

impl std::fmt::Display for CheckpointManifestError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty { field } => write!(formatter, "{field} is empty"),
            Self::TooLong { field } => write!(formatter, "{field} is too long"),
            Self::InvalidDigest { field } => write!(formatter, "{field} is not a valid digest"),
            Self::NoRestoreArtifacts => formatter.write_str("restore_artifacts is empty"),
            Self::UnsafeInteger { field } => write!(formatter, "{field} is outside the safe range"),
            Self::Serialization => formatter.write_str("manifest serialization failed"),
        }
    }
}

impl std::error::Error for CheckpointManifestError {}

fn validate_text(field: &'static str, value: &str) -> Result<(), CheckpointManifestError> {
    if value.is_empty() {
        return Err(CheckpointManifestError::Empty { field });
    }
    if value.len() > CHECKPOINT_CAPTURE_MAX_ID_BYTES {
        return Err(CheckpointManifestError::TooLong { field });
    }
    Ok(())
}

fn validate_safe_integer(field: &'static str, value: u64) -> Result<(), CheckpointManifestError> {
    if value > MAX_SAFE_INTEGER {
        return Err(CheckpointManifestError::UnsafeInteger { field });
    }
    Ok(())
}

fn validate_digest(
    field: &'static str,
    value: &str,
    prefix: &str,
) -> Result<(), CheckpointManifestError> {
    let Some(hex) = value.strip_prefix(prefix) else {
        return Err(CheckpointManifestError::InvalidDigest { field });
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(CheckpointManifestError::InvalidDigest { field });
    }
    Ok(())
}

fn domain_digest(prefix: &str, domain: &[u8], bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    format!("{prefix}{hex}")
}
