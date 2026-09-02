// SPDX-License-Identifier: MIT

/// Owned request data that may cross into the host boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostRequest {
    /// Monotonic identifier assigned by the owning adapter.
    pub request_id: u64,
    /// Opaque, bounded payload owned by the request.
    pub payload: Vec<u8>,
}

impl HostRequest {
    /// Creates an owned host request.
    #[must_use]
    pub fn new(request_id: u64, payload: impl Into<Vec<u8>>) -> Self {
        Self {
            request_id,
            payload: payload.into(),
        }
    }
}

/// Minimal owned projection of host state used by boundary orchestration.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostSnapshot {
    /// Host generation associated with the projection.
    pub generation: u64,
    /// Whether the host currently accepts work.
    pub ready: bool,
}

/// Result of host-side acceptance or completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostReceipt {
    /// Request identifier supplied by the caller.
    pub request_id: u64,
    /// Host generation that observed the request.
    pub generation: u64,
}

/// Host-side failure that remains distinct from queue admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostError {
    /// The host cannot accept work in its current lifecycle state.
    NotReady,
    /// The host rejected the request after main-thread dispatch.
    Rejected,
}

/// Port implemented by the managed or host-specific adapter.
pub trait HostPort {
    /// Reads an owned state projection.
    fn snapshot(&self) -> Result<HostSnapshot, HostError>;

    /// Applies one already-admitted request on the host thread.
    fn submit(&mut self, request: HostRequest) -> Result<HostReceipt, HostError>;
}
