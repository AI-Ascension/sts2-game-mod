// SPDX-License-Identifier: MIT

//! Source-only owner boundary for exact native checkpoint capture.
//!
//! The port is deliberately fail-closed until an authorized exact-host capture
//! producer proves a complete closure for a boundary. It does not inspect host
//! objects or imply that any native phase is currently supported.

mod capability;
mod error;
mod identity;
mod manifest;
mod port;
mod receipt;

pub use capability::{
    CheckpointBoundary, CheckpointCapabilities, CheckpointCapability, CheckpointUnavailableReason,
};
pub use error::CheckpointCaptureRejection;
pub use identity::{CheckpointCaptureIdentity, CheckpointCaptureRequest, CheckpointIdentityError};
pub use manifest::{
    CHECKPOINT_MANIFEST_PROFILE, CHECKPOINT_MANIFEST_SCHEMA, CheckpointArtifactDescriptor,
    CheckpointManifest, CheckpointManifestBoundary, CheckpointManifestError,
    CheckpointManifestParts, CheckpointOrigin,
};
pub use port::{CheckpointCapturePort, UnavailableCheckpointCapture};
pub use receipt::{CheckpointCaptureReceipt, CheckpointDurability};

/// Canonical exact-state profile consumed by this owner boundary.
pub const CHECKPOINT_CAPTURE_PROFILE: &str = "asc-jcs-state-v1";
/// Exact-state payload schema consumed by this owner boundary.
pub const CHECKPOINT_CAPTURE_SCHEMA: &str = "ascension.exact_state.v1";
/// Maximum canonical payload accepted by the protocol contract.
pub const CHECKPOINT_CAPTURE_MAX_BYTES: usize = 16 * 1024 * 1024;
/// Maximum encoded size of a session, lease, run, profile, or operation identity.
pub const CHECKPOINT_CAPTURE_MAX_ID_BYTES: usize = 256;
/// Domain separator for the exact-state identity.
pub const CHECKPOINT_STATE_DOMAIN: &[u8] = b"AI-ASCENSION/EXACT-STATE/v1\0";
/// Domain separator for the checkpoint identity.
pub const CHECKPOINT_ID_DOMAIN: &[u8] = b"AI-ASCENSION/CHECKPOINT/v1\0";

pub(super) const STATE_DIGEST_PREFIX: &str = "asc-state:v1:sha256:";
pub(super) const CHECKPOINT_ID_PREFIX: &str = "asc-checkpoint:v1:sha256:";
pub(super) const BLOB_DIGEST_PREFIX: &str = "sha256:";
