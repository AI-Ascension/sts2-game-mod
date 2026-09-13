// SPDX-License-Identifier: MIT

use super::{RngAuditReadError, RngAuditUnavailableReason, RngAuditWitness};

/// Host-thread port for a read-only seeded initialization audit.
///
/// Implementations must copy owned evidence from a stable host boundary. A read
/// may not advance a stream, create playable objects, mutate a profile, or
/// expose raw state through a model-facing projection.
pub trait RngAuditPort {
    /// Reads one validated private initialization witness.
    fn read_initialization_witness(&self) -> Result<RngAuditWitness, RngAuditReadError>;
}

/// Explicitly unavailable implementation used until authorized exact-host evidence exists.
#[derive(Debug, Default)]
pub struct UnavailableRngAudit;

impl RngAuditPort for UnavailableRngAudit {
    fn read_initialization_witness(&self) -> Result<RngAuditWitness, RngAuditReadError> {
        Err(RngAuditReadError::Unavailable(
            RngAuditUnavailableReason::ExactHostEvidenceRequired,
        ))
    }
}
