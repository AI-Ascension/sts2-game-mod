// SPDX-License-Identifier: MIT

use super::super::canonical::blob_digest;
use super::super::capability::{
    CheckpointCapabilities, CheckpointCapability, CheckpointUnavailableReason,
};
use super::super::error::CheckpointCaptureRejection;
use super::super::identity::CheckpointCaptureRequest;
use super::super::receipt::{CheckpointCaptureReceipt, CheckpointDurability};
use super::barrier::{CheckpointCaptureBarrier, CheckpointMutationFence, CheckpointSettlement};
use super::ledger::{CheckpointAdmissionDecision, CheckpointAdmissionLedger};
use super::producer::{CheckpointCaptureProducer, CheckpointProducerCapability};

/// Bounded outcome of one admitted capture operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CheckpointAdmissionOutcome {
    /// A new artifact was captured and, when the producer is durable, persisted.
    Captured(CheckpointCaptureReceipt),
    /// An identical operation returned the originally recorded receipt.
    Replayed(CheckpointCaptureReceipt),
}

/// Owner capture-admission controller for one host game thread.
///
/// The controller orders owner work against host mutation and binds one logical operation to one
/// receipt. It never inspects a host object itself: the attached producer owns native field and RNG
/// extraction, and only the producer's owned bytes and manifest leave the capture window.
#[derive(Debug)]
pub struct CheckpointCaptureAdmission<P> {
    barrier: CheckpointCaptureBarrier,
    ledger: CheckpointAdmissionLedger,
    producer: P,
}

impl<P: CheckpointCaptureProducer> CheckpointCaptureAdmission<P> {
    /// Creates a controller around one producer.
    #[must_use]
    pub fn new(producer: P) -> Self {
        Self::with_barrier(CheckpointCaptureBarrier::new(), producer)
    }

    /// Creates a controller around a caller-owned barrier and one producer.
    ///
    /// A host adapter that already owns the mutation path owns the barrier too, so it can hand out
    /// [`CheckpointCaptureBarrier::fence`] handles and still let the controller refuse a capture
    /// while other host work holds the barrier.
    #[must_use]
    pub fn with_barrier(barrier: CheckpointCaptureBarrier, producer: P) -> Self {
        Self {
            barrier,
            ledger: CheckpointAdmissionLedger::new(),
            producer,
        }
    }

    /// Returns the current fail-closed owner boundary classification.
    #[must_use]
    pub const fn capabilities(&self) -> CheckpointCapabilities {
        CheckpointCapabilities
    }

    /// Returns the owner mutation barrier.
    #[must_use]
    pub const fn barrier(&self) -> &CheckpointCaptureBarrier {
        &self.barrier
    }

    /// Returns a cloneable host mutation handle fenced by the barrier.
    #[must_use]
    pub fn fence(&self) -> CheckpointMutationFence {
        self.barrier.fence()
    }

    /// Returns the recorded-operation ledger.
    #[must_use]
    pub const fn ledger(&self) -> &CheckpointAdmissionLedger {
        &self.ledger
    }

    /// Returns the attached producer without permitting bypass of the barrier.
    #[must_use]
    pub const fn producer(&self) -> &P {
        &self.producer
    }

    /// Admits, produces, persists, and records one capture operation.
    ///
    /// The barrier is held from settlement validation through snapshotting, so any host mutation
    /// offered through [`Self::fence`] in that window is refused. An identical repeat of a recorded
    /// operation re-produces the payload under the barrier to detect hidden change and then returns
    /// the original receipt; a different payload under one operation is a conflict.
    ///
    /// The owner matrix decides first: an unsafe phase is refused for every producer, including a
    /// synthetic fixture. A boundary that is merely pending coverage inventory or host evidence is
    /// left to the attached producer, so its receipt or refusal carries that producer's own gate
    /// rather than the boundary matrix default.
    ///
    /// # Errors
    ///
    /// Returns [`CheckpointCaptureRejection::UnsafeBoundary`] for a non-quiescent settlement or an
    /// unclassifiable phase, [`CheckpointCaptureRejection::Busy`] while another capture holds the
    /// barrier, [`CheckpointCaptureRejection::UnsupportedBoundary`] when no admitted producer
    /// serves the boundary, [`CheckpointCaptureRejection::OperationConflict`] when one operation
    /// identity is reused with different bytes or request, and
    /// [`CheckpointCaptureRejection::PersistenceFailed`] when durable storage did not complete. A
    /// failure never records a receipt, so a retry of the same operation may still succeed.
    pub fn capture(
        &mut self,
        settlement: CheckpointSettlement,
        request: CheckpointCaptureRequest,
    ) -> Result<CheckpointAdmissionOutcome, CheckpointCaptureRejection> {
        let boundary = request.boundary();
        if self.capabilities().for_boundary(boundary).reason()
            == CheckpointUnavailableReason::UnsafeBoundary
        {
            return Err(CheckpointCaptureRejection::UnsafeBoundary);
        }
        match self.producer.capability() {
            CheckpointProducerCapability::SyntheticFixtureOnly => {}
            CheckpointProducerCapability::Unavailable(reason) => {
                return Err(CheckpointCapability::Unavailable { reason }.rejection(boundary));
            }
        }
        let _guard = self.barrier.try_hold(settlement)?;
        let produced = self.producer.produce(&request)?;
        let payload_digest = blob_digest(produced.canonical_bytes());
        match self.ledger.decide(&request, &payload_digest) {
            CheckpointAdmissionDecision::Replayed => {
                return match self.ledger.lookup(request.identity().operation_id()) {
                    Some(receipt) => Ok(CheckpointAdmissionOutcome::Replayed(receipt.clone())),
                    None => Err(CheckpointCaptureRejection::OperationConflict),
                };
            }
            CheckpointAdmissionDecision::Conflict => {
                return Err(CheckpointCaptureRejection::OperationConflict);
            }
            CheckpointAdmissionDecision::Full => {
                return Err(CheckpointCaptureRejection::AdmissionLedgerFull);
            }
            CheckpointAdmissionDecision::New => {}
        }
        let durability = self.producer.durability();
        if durability == CheckpointDurability::Durable && !self.producer.persist(&produced) {
            return Err(CheckpointCaptureRejection::PersistenceFailed);
        }
        let receipt = CheckpointCaptureReceipt::from_validated_canonical_bytes(
            &request,
            durability,
            produced.canonical_bytes().to_vec(),
            produced.manifest().clone(),
        )?;
        self.ledger
            .record(&request, payload_digest, receipt.clone())?;
        Ok(CheckpointAdmissionOutcome::Captured(receipt))
    }
}
