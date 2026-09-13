// SPDX-License-Identifier: MIT

use sha2::{Digest, Sha256};

use super::capability::CheckpointBoundary;
use super::error::CheckpointCaptureRejection;
use super::identity::{CheckpointCaptureIdentity, CheckpointCaptureRequest};
use super::{
    BLOB_DIGEST_PREFIX, CHECKPOINT_CAPTURE_MAX_BYTES, CHECKPOINT_ID_DOMAIN, CHECKPOINT_ID_PREFIX,
    CHECKPOINT_STATE_DOMAIN, STATE_DIGEST_PREFIX,
};

/// Distinguishes an in-memory capture from a durable artifact.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointDurability {
    /// The artifact has not been durably persisted.
    InMemory,
    /// The artifact has been durably persisted by a trusted owner.
    Durable,
}

/// A private exact-state artifact returned through the trusted capture port.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckpointCaptureReceipt {
    identity: CheckpointCaptureIdentity,
    boundary: CheckpointBoundary,
    durability: CheckpointDurability,
    canonical_bytes: Vec<u8>,
    state_digest: String,
    checkpoint_id: String,
    blob_digest: String,
}

impl CheckpointCaptureReceipt {
    /// Wraps bytes already validated by the protocol owner as canonical.
    ///
    /// This applies the profile's identity domains and size bound; it does not
    /// implement canonicalization or claim that arbitrary bytes are valid state.
    pub fn from_validated_canonical_bytes(
        request: &CheckpointCaptureRequest,
        durability: CheckpointDurability,
        canonical_bytes: Vec<u8>,
    ) -> Result<Self, CheckpointCaptureRejection> {
        if canonical_bytes.is_empty() {
            return Err(CheckpointCaptureRejection::InvalidCanonicalBytes);
        }
        if canonical_bytes.len() > CHECKPOINT_CAPTURE_MAX_BYTES {
            return Err(CheckpointCaptureRejection::Oversize {
                bytes: canonical_bytes.len(),
            });
        }
        let blob_digest = digest_with_prefix(BLOB_DIGEST_PREFIX, &[], &canonical_bytes);
        let state_digest = digest_with_prefix(
            STATE_DIGEST_PREFIX,
            CHECKPOINT_STATE_DOMAIN,
            &canonical_bytes,
        );
        let checkpoint_id =
            digest_with_prefix(CHECKPOINT_ID_PREFIX, CHECKPOINT_ID_DOMAIN, &canonical_bytes);
        Ok(Self {
            identity: request.identity().clone(),
            boundary: request.boundary(),
            durability,
            canonical_bytes,
            state_digest,
            checkpoint_id,
            blob_digest,
        })
    }

    /// Returns the identity fence copied into this receipt.
    #[must_use]
    pub fn identity(&self) -> &CheckpointCaptureIdentity {
        &self.identity
    }

    /// Returns the boundary copied into this receipt.
    #[must_use]
    pub const fn boundary(&self) -> CheckpointBoundary {
        self.boundary
    }

    /// Returns whether persistence has completed.
    #[must_use]
    pub const fn durability(&self) -> CheckpointDurability {
        self.durability
    }

    /// Returns the private canonical byte count without exposing payload bytes.
    #[must_use]
    pub fn canonical_len(&self) -> usize {
        self.canonical_bytes.len()
    }

    /// Returns the exact-state digest.
    #[must_use]
    pub fn state_digest(&self) -> &str {
        &self.state_digest
    }

    /// Returns the checkpoint-manifest identity.
    #[must_use]
    pub fn checkpoint_id(&self) -> &str {
        &self.checkpoint_id
    }

    /// Returns the raw canonical blob digest.
    #[must_use]
    pub fn blob_digest(&self) -> &str {
        &self.blob_digest
    }
}

fn digest_with_prefix(prefix: &str, domain: &[u8], bytes: &[u8]) -> String {
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
