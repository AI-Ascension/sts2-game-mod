// SPDX-License-Identifier: MIT

use std::collections::VecDeque;

use sts2_game_mod_host::MainThreadQueue;

use super::contract::{RuntimeV2Identity, RuntimeV2ValidationError};
use super::receipt::{OperationReceipt, QueuedOperation};

/// Maximum encoded request size accepted by the deterministic Runtime-v2 seam.
pub const RUNTIME_V2_MAX_REQUEST_BYTES: usize = 4 * 1024;
/// Maximum queue capacity accepted by this bounded fake seam.
pub const RUNTIME_V2_MAX_QUEUE_CAPACITY: usize = 1024;
/// Maximum retained operation receipts accepted by this bounded fake seam.
pub const RUNTIME_V2_MAX_RECEIPTS: usize = 1024;

/// Configuration for one isolated Runtime-v2 fake instance.
#[derive(Clone, Debug)]
pub struct RuntimeV2Config {
    /// Identity and lease expected by the instance.
    pub identity: RuntimeV2Identity,
    /// Maximum admitted operations waiting for the game thread.
    pub queue_capacity: usize,
    /// Maximum operation receipts retained for replay and reconciliation.
    pub receipt_capacity: usize,
    /// Maximum encoded request body.
    pub max_request_bytes: usize,
}

impl Default for RuntimeV2Config {
    fn default() -> Self {
        Self {
            identity: RuntimeV2Identity::new("instance-1", "session-1", "lease-1", 1),
            queue_capacity: 8,
            receipt_capacity: 16,
            max_request_bytes: RUNTIME_V2_MAX_REQUEST_BYTES,
        }
    }
}

impl RuntimeV2Config {
    pub(super) fn validate(&self) -> Result<(), RuntimeV2Error> {
        self.identity
            .validate()
            .map_err(RuntimeV2Error::InvalidConfig)?;
        if self.queue_capacity > RUNTIME_V2_MAX_QUEUE_CAPACITY
            || self.receipt_capacity > RUNTIME_V2_MAX_RECEIPTS
            || self.max_request_bytes == 0
            || self.max_request_bytes > RUNTIME_V2_MAX_REQUEST_BYTES
        {
            return Err(RuntimeV2Error::InvalidConfig(
                RuntimeV2ValidationError::RequestShape,
            ));
        }
        Ok(())
    }
}

/// Infrastructure failures at the Runtime-v2 fake boundary.
#[derive(Debug, Eq, PartialEq)]
pub enum RuntimeV2Error {
    /// The copied protocol artifact failed local verification.
    Artifact(super::artifact::RuntimeV2ArtifactError),
    /// The request body exceeded the local bound.
    RequestTooLarge,
    /// The request was not valid Runtime-v2 input.
    MalformedRequest,
    /// The request used a different protocol artifact identity.
    ArtifactMismatch,
    /// The fake host returned an invalid observation.
    InvalidObservation,
    /// The runtime configuration exceeded a local bound.
    InvalidConfig(RuntimeV2ValidationError),
    /// The requested operation is not retained.
    OperationNotFound,
    /// The requested operation is not waiting for dispatch.
    OperationNotQueued,
    /// A cancellation was attempted after admission.
    CancellationAfterAdmission,
    /// An encoded response could not be produced.
    Encoding,
    /// The state request identity or lease was not current.
    StaleIdentity,
}

impl std::fmt::Display for RuntimeV2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::Artifact(_) => "the copied Runtime-v2 artifact failed verification",
            Self::RequestTooLarge => "the Runtime-v2 request exceeds its byte bound",
            Self::MalformedRequest => "the Runtime-v2 request is malformed",
            Self::ArtifactMismatch => "the request does not use the copied Runtime-v2 artifact",
            Self::InvalidObservation => "the fake host returned an invalid observation",
            Self::InvalidConfig(_) => "the Runtime-v2 configuration exceeds a bound",
            Self::OperationNotFound => "the Runtime-v2 operation is not retained",
            Self::OperationNotQueued => "the Runtime-v2 operation is not queued",
            Self::CancellationAfterAdmission => "admitted Runtime-v2 work cannot be cancelled",
            Self::Encoding => "the Runtime-v2 response could not be encoded",
            Self::StaleIdentity => "the Runtime-v2 request has stale identity or lease data",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for RuntimeV2Error {}

/// Deterministic Runtime-v2 boundary with a bounded main-thread queue and receipt store.
#[derive(Debug)]
pub struct RuntimeV2Mod<G> {
    pub(super) game: G,
    pub(super) identity: RuntimeV2Identity,
    pub(super) queue: MainThreadQueue<QueuedOperation>,
    pub(super) receipts: VecDeque<OperationReceipt>,
    pub(super) receipt_capacity: usize,
    pub(super) max_request_bytes: usize,
}
