// SPDX-License-Identifier: MIT

/// Caller context used by the Runtime-v2 message constructors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeV2Context {
    /// Request correlation identity.
    pub correlation_id: String,
    /// Isolated runtime instance identity.
    pub instance_id: String,
    /// Session identity.
    pub session_id: String,
    /// Lease identity.
    pub lease_id: String,
    /// Lease epoch.
    pub lease_epoch: u64,
}

impl RuntimeV2Context {
    /// Creates a caller context for one request.
    #[must_use]
    pub fn new(
        correlation_id: impl Into<String>,
        instance_id: impl Into<String>,
        session_id: impl Into<String>,
        lease_id: impl Into<String>,
        lease_epoch: u64,
    ) -> Self {
        Self {
            correlation_id: correlation_id.into(),
            instance_id: instance_id.into(),
            session_id: session_id.into(),
            lease_id: lease_id.into(),
            lease_epoch,
        }
    }
}
