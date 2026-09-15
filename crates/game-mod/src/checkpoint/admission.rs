// SPDX-License-Identifier: MIT

//! Source-only owner admission for exact native checkpoint capture.
//!
//! The capture port in [`super::port`] is inert until an authorized exact-host producer exists.
//! This module owns the two fail-closed gates that a producer must pass before it may publish a
//! checkpoint: the settlement barrier (no host mutation between boundary validation and
//! snapshotting) and the operation ledger (one receipt per logical operation, with a conflict when
//! the payload or request changes under one operation).
//!
//! Nothing here inspects a host object, advertises a native phase, persists an artifact on its own,
//! or implements restore. The default producer remains
//! [`UnavailableCheckpointProducer`], so every boundary stays unavailable until exact-host evidence
//! passes under ADR 0037, ADR 0042, and ADR 0043.

mod barrier;
mod controller;
mod ledger;
mod producer;

pub use barrier::{
    CheckpointBarrierGuard, CheckpointCaptureBarrier, CheckpointMutationFence, CheckpointSettlement,
};
pub use controller::{CheckpointAdmissionOutcome, CheckpointCaptureAdmission};
pub use ledger::{CheckpointAdmissionDecision, CheckpointAdmissionLedger};
pub use producer::{
    CheckpointCaptureProducer, CheckpointProducedCheckpoint, CheckpointProducerCapability,
    FixtureCheckpointProducer, UnavailableCheckpointProducer,
};
