// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;

use super::super::CHECKPOINT_ADMISSION_MAX_OPERATIONS;
use super::super::error::CheckpointCaptureRejection;
use super::super::identity::CheckpointCaptureRequest;
use super::super::receipt::CheckpointCaptureReceipt;

/// One recorded capture operation, keyed by the logical operation identity.
#[derive(Clone, Eq, PartialEq)]
struct CheckpointAdmissionEntry {
    request: CheckpointCaptureRequest,
    payload_digest: String,
    receipt: CheckpointCaptureReceipt,
}

/// What the owner may do with one logical capture operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckpointAdmissionDecision {
    /// The operation is new and the ledger has room to record its receipt.
    New,
    /// The identical operation was already recorded; [`CheckpointAdmissionLedger::lookup`]
    /// returns the original receipt.
    Replayed,
    /// The operation identity was reused with a different request or a different payload.
    Conflict,
    /// The operation is new but the ledger is at capacity.
    Full,
}

/// Bounded owner record of admitted capture operations.
///
/// The ledger makes one logical operation binding: a repeated admission of the same request and
/// the same canonical payload returns the original receipt, while a reused operation identity with
/// a different boundary, identity fence, or payload is a conflict. It stores only owner-side
/// receipt values; it never persists an artifact and never decides native capability.
#[derive(Clone, Default)]
pub struct CheckpointAdmissionLedger {
    entries: BTreeMap<String, CheckpointAdmissionEntry>,
}

impl CheckpointAdmissionLedger {
    /// Creates an empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the maximum number of retained operations.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        CHECKPOINT_ADMISSION_MAX_OPERATIONS
    }

    /// Returns the number of retained operations.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns whether no operation has been recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the receipt recorded for an operation identity.
    #[must_use]
    pub fn lookup(&self, operation_id: &str) -> Option<&CheckpointCaptureReceipt> {
        self.entries.get(operation_id).map(|entry| &entry.receipt)
    }

    /// Classifies a candidate admission without changing the ledger.
    #[must_use]
    pub fn decide(
        &self,
        request: &CheckpointCaptureRequest,
        payload_digest: &str,
    ) -> CheckpointAdmissionDecision {
        let operation_id = request.identity().operation_id();
        match self.entries.get(operation_id) {
            Some(entry) => {
                if entry.request == *request && entry.payload_digest == payload_digest {
                    CheckpointAdmissionDecision::Replayed
                } else {
                    CheckpointAdmissionDecision::Conflict
                }
            }
            None if self.entries.len() >= CHECKPOINT_ADMISSION_MAX_OPERATIONS => {
                CheckpointAdmissionDecision::Full
            }
            None => CheckpointAdmissionDecision::New,
        }
    }

    /// Records a new operation receipt.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointCaptureRejection::OperationConflict`] when the operation identity is
    /// already recorded and [`CheckpointCaptureRejection::AdmissionLedgerFull`] when a new
    /// operation would exceed [`Self::capacity`]. A rejected record leaves the ledger unchanged.
    pub fn record(
        &mut self,
        request: &CheckpointCaptureRequest,
        payload_digest: String,
        receipt: CheckpointCaptureReceipt,
    ) -> Result<(), CheckpointCaptureRejection> {
        let operation_id = request.identity().operation_id();
        if self.entries.contains_key(operation_id) {
            return Err(CheckpointCaptureRejection::OperationConflict);
        }
        if self.entries.len() >= CHECKPOINT_ADMISSION_MAX_OPERATIONS {
            return Err(CheckpointCaptureRejection::AdmissionLedgerFull);
        }
        self.entries.insert(
            operation_id.to_owned(),
            CheckpointAdmissionEntry {
                request: request.clone(),
                payload_digest,
                receipt,
            },
        );
        Ok(())
    }
}

impl std::fmt::Debug for CheckpointAdmissionLedger {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CheckpointAdmissionLedger")
            .field("retained_operations", &self.entries.len())
            .field("capacity", &CHECKPOINT_ADMISSION_MAX_OPERATIONS)
            .field("operation_ids", &self.entries.keys())
            .finish()
    }
}
