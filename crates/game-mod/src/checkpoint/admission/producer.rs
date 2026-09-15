// SPDX-License-Identifier: MIT

use std::cell::Cell;

use super::super::canonical::{blob_digest, state_id};
use super::super::error::CheckpointCaptureRejection;
use super::super::identity::CheckpointCaptureRequest;
use super::super::manifest::{
    CheckpointArtifactDescriptor, CheckpointManifest, CheckpointManifestBoundary,
    CheckpointManifestParts, CheckpointOrigin,
};
use super::super::receipt::CheckpointDurability;
use super::super::{CHECKPOINT_CAPTURE_PROFILE, CheckpointUnavailableReason};
use super::barrier::CheckpointMutationFence;

/// Producer capability for the owner checkpoint seam.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointProducerCapability {
    /// Only deterministic in-memory fixtures implement this boundary.
    SyntheticFixtureOnly,
    /// No supported producer is attached; native phases stay fail-closed.
    Unavailable(CheckpointUnavailableReason),
}

impl CheckpointProducerCapability {
    /// Returns whether a producer can yield an owned checkpoint.
    #[must_use]
    pub const fn is_available(self) -> bool {
        matches!(self, Self::SyntheticFixtureOnly)
    }
}

/// Owned canonical payload and closure manifest produced on the host game thread.
///
/// The value carries immutable owned bytes only. It never borrows a host object, and it is the
/// producer's responsibility to have validated the bytes as canonical before returning them.
#[derive(Clone, Eq, PartialEq)]
pub struct CheckpointProducedCheckpoint {
    canonical_bytes: Vec<u8>,
    manifest: CheckpointManifest,
}

impl CheckpointProducedCheckpoint {
    /// Creates an owned produced checkpoint.
    #[must_use]
    pub fn new(canonical_bytes: Vec<u8>, manifest: CheckpointManifest) -> Self {
        Self {
            canonical_bytes,
            manifest,
        }
    }

    /// Returns the produced canonical exact-state bytes.
    #[must_use]
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }

    /// Returns the produced restore-closure manifest.
    #[must_use]
    pub fn manifest(&self) -> &CheckpointManifest {
        &self.manifest
    }
}

impl std::fmt::Debug for CheckpointProducedCheckpoint {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CheckpointProducedCheckpoint")
            .field("canonical_bytes_len", &self.canonical_bytes.len())
            .finish()
    }
}

/// Owner-thread producer that yields canonical bytes and a closure manifest for one boundary.
pub trait CheckpointCaptureProducer {
    /// Reports capability without inspecting host objects.
    fn capability(&self) -> CheckpointProducerCapability;

    /// Reports whether produced artifacts are durably stored by this producer.
    fn durability(&self) -> CheckpointDurability;

    /// Produces the canonical payload and closure manifest for an admitted boundary.
    ///
    /// # Errors
    ///
    /// Returns a typed rejection when the producer cannot close the boundary. It must never
    /// substitute an earlier floor, turn, or public observation for the requested boundary.
    fn produce(
        &mut self,
        request: &CheckpointCaptureRequest,
    ) -> Result<CheckpointProducedCheckpoint, CheckpointCaptureRejection>;

    /// Persists a produced artifact and reports whether it is durably stored.
    fn persist(&mut self, produced: &CheckpointProducedCheckpoint) -> bool;
}

/// Explicitly unavailable producer used until exact-host evidence exists.
#[derive(Debug, Default)]
pub struct UnavailableCheckpointProducer;

impl CheckpointCaptureProducer for UnavailableCheckpointProducer {
    fn capability(&self) -> CheckpointProducerCapability {
        CheckpointProducerCapability::Unavailable(
            CheckpointUnavailableReason::ExactHostEvidenceRequired,
        )
    }

    fn durability(&self) -> CheckpointDurability {
        CheckpointDurability::InMemory
    }

    fn produce(
        &mut self,
        _request: &CheckpointCaptureRequest,
    ) -> Result<CheckpointProducedCheckpoint, CheckpointCaptureRejection> {
        Err(CheckpointCaptureRejection::UnsupportedCoverage)
    }

    fn persist(&mut self, _produced: &CheckpointProducedCheckpoint) -> bool {
        false
    }
}

/// Deterministic in-memory producer used by owner-local source tests.
///
/// It is explicitly synthetic: it reads no host object, serializes no native field, and its
/// restore descriptor is the pinned fixture illustration rather than a real closure. Each call
/// yields the next queued payload, repeating the last one once the queue is exhausted so a repeated
/// admission reproduces identical bytes.
#[derive(Clone)]
pub struct FixtureCheckpointProducer {
    payloads: Vec<Vec<u8>>,
    durability: CheckpointDurability,
    persistence: bool,
    produced: Cell<usize>,
    probe: Option<CheckpointMutationFence>,
    probe_outcome: Cell<Option<Result<(), CheckpointCaptureRejection>>>,
}

impl FixtureCheckpointProducer {
    /// Creates a synthetic producer with in-memory durability and successful persistence.
    #[must_use]
    pub fn new(payloads: Vec<Vec<u8>>) -> Self {
        Self {
            payloads,
            durability: CheckpointDurability::InMemory,
            persistence: true,
            produced: Cell::new(0),
            probe: None,
            probe_outcome: Cell::new(None),
        }
    }

    /// Sets the durability reported by this producer.
    #[must_use]
    pub const fn with_durability(mut self, durability: CheckpointDurability) -> Self {
        self.durability = durability;
        self
    }

    /// Sets whether [`CheckpointCaptureProducer::persist`] reports success.
    #[must_use]
    pub const fn with_persistence(mut self, persistence: bool) -> Self {
        self.persistence = persistence;
        self
    }

    /// Returns how many payloads have been produced.
    #[must_use]
    pub fn produced_count(&self) -> usize {
        self.produced.get()
    }

    /// Attaches a host mutation handle that production attempts once inside the capture window.
    ///
    /// The probe models host work that offers a gameplay mutation while a capture is validating and
    /// snapshotting. A correct barrier refuses it, which is how the owner proves the capture window
    /// excludes mutation without a live host.
    #[must_use]
    pub fn with_mutation_probe(mut self, fence: CheckpointMutationFence) -> Self {
        self.probe = Some(fence);
        self
    }

    /// Returns the outcome of the most recent mutation probe attempt.
    #[must_use]
    pub fn mutation_probe(&self) -> Option<Result<(), CheckpointCaptureRejection>> {
        self.probe_outcome.get()
    }
}

impl std::fmt::Debug for FixtureCheckpointProducer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FixtureCheckpointProducer")
            .field("payload_count", &self.payloads.len())
            .field("durability", &self.durability)
            .field("persistence", &self.persistence)
            .field("produced_count", &self.produced.get())
            .field("has_mutation_probe", &self.probe.is_some())
            .finish()
    }
}

impl CheckpointCaptureProducer for FixtureCheckpointProducer {
    fn capability(&self) -> CheckpointProducerCapability {
        CheckpointProducerCapability::SyntheticFixtureOnly
    }

    fn durability(&self) -> CheckpointDurability {
        self.durability
    }

    fn produce(
        &mut self,
        request: &CheckpointCaptureRequest,
    ) -> Result<CheckpointProducedCheckpoint, CheckpointCaptureRejection> {
        if let Some(probe) = &self.probe {
            self.probe_outcome.set(Some(probe.try_mutate()));
        }
        let index = self
            .produced
            .get()
            .min(self.payloads.len().saturating_sub(1));
        let Some(payload) = self.payloads.get(index) else {
            return Err(CheckpointCaptureRejection::InvalidCanonicalBytes);
        };
        self.produced.set(self.produced.get().saturating_add(1));
        let manifest = fixture_manifest(request, payload)?;
        Ok(CheckpointProducedCheckpoint::new(payload.clone(), manifest))
    }

    fn persist(&mut self, _produced: &CheckpointProducedCheckpoint) -> bool {
        self.persistence
    }
}

/// Builds the synthetic fixture manifest that closes the produced payload bytes.
fn fixture_manifest(
    request: &CheckpointCaptureRequest,
    payload: &[u8],
) -> Result<CheckpointManifest, CheckpointCaptureRejection> {
    let restore_digest = blob_digest(b"ascension.checkpoint.fixture.restore.v1");
    let compatibility_digest = blob_digest(b"ascension.checkpoint.fixture.compatibility.v1");
    let coverage_digest = blob_digest(b"ascension.checkpoint.fixture.coverage.v1");
    let payload_digest = blob_digest(payload);
    let size_bytes = payload.len() as u64;
    let payload_descriptor = CheckpointArtifactDescriptor::new(
        CHECKPOINT_CAPTURE_PROFILE,
        payload_digest,
        "exact_state_payload",
        size_bytes,
    )
    .map_err(|_| CheckpointCaptureRejection::InvalidManifest)?;
    let restore_descriptor = CheckpointArtifactDescriptor::new(
        "fixture-json-illustration",
        restore_digest,
        "fixture_snapshot",
        size_bytes,
    )
    .map_err(|_| CheckpointCaptureRejection::InvalidManifest)?;
    let boundary = CheckpointManifestBoundary::new("decision", request.boundary().code(), None)
        .map_err(|_| CheckpointCaptureRejection::InvalidManifest)?;
    let origin = CheckpointOrigin::new(request.identity().run_id(), 1)
        .map_err(|_| CheckpointCaptureRejection::InvalidManifest)?;
    CheckpointManifest::from_parts(CheckpointManifestParts {
        exact_state_digest: state_id(payload),
        canonical_payload: payload_descriptor,
        restore_artifacts: vec![restore_descriptor],
        compatibility_digest,
        coverage_contract_digest: coverage_digest,
        boundary,
        origin,
        parent_checkpoint_id: None,
    })
    .map_err(|_| CheckpointCaptureRejection::InvalidManifest)
}
