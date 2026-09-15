// SPDX-License-Identifier: MIT

use super::capability::CheckpointCapabilities;
use super::error::CheckpointCaptureRejection;
use super::identity::CheckpointCaptureRequest;
use super::receipt::CheckpointCaptureReceipt;

/// Owner port for game-thread checkpoint capture.
pub trait CheckpointCapturePort {
    /// Returns the current bounded capability descriptor.
    fn capabilities(&self) -> CheckpointCapabilities;

    /// Captures an immutable owned artifact after boundary admission.
    fn capture(
        &mut self,
        request: CheckpointCaptureRequest,
    ) -> Result<CheckpointCaptureReceipt, CheckpointCaptureRejection>;
}

/// Explicitly unavailable implementation used until exact-host evidence exists.
#[derive(Debug, Default)]
pub struct UnavailableCheckpointCapture;

impl CheckpointCapturePort for UnavailableCheckpointCapture {
    fn capabilities(&self) -> CheckpointCapabilities {
        CheckpointCapabilities
    }

    fn capture(
        &mut self,
        request: CheckpointCaptureRequest,
    ) -> Result<CheckpointCaptureReceipt, CheckpointCaptureRejection> {
        let boundary = request.boundary();
        Err(self
            .capabilities()
            .for_boundary(boundary)
            .rejection(boundary))
    }
}
