// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::error::Error;

use sts2_game_mod::{
    CHECKPOINT_ADMISSION_MAX_OPERATIONS, CheckpointAdmissionDecision, CheckpointAdmissionLedger,
    CheckpointAdmissionOutcome, CheckpointBoundary, CheckpointCapabilities,
    CheckpointCaptureAdmission, CheckpointCaptureBarrier, CheckpointCaptureIdentity,
    CheckpointCaptureProducer, CheckpointCaptureReceipt, CheckpointCaptureRejection,
    CheckpointCaptureRequest, CheckpointDurability, CheckpointProducerCapability,
    CheckpointSettlement, CheckpointUnavailableReason, FixtureCheckpointProducer,
    UnavailableCheckpointProducer, blob_digest, state_id,
};

const PAYLOAD: &[u8] = b"{\"a\":1,\"b\":2}";
const OTHER_PAYLOAD: &[u8] = b"{\"a\":1,\"b\":3}";
const COMBAT: CheckpointBoundary = CheckpointBoundary::StablePlayerTurnCombat;

fn identity(operation_id: &str) -> CheckpointCaptureIdentity {
    CheckpointCaptureIdentity::new(
        "instance-1",
        "session-1",
        "lease-1",
        7,
        "run-1",
        "profile-1",
        operation_id,
    )
    .expect("identity")
}

fn request(operation_id: &str, boundary: CheckpointBoundary) -> CheckpointCaptureRequest {
    CheckpointCaptureRequest::new(identity(operation_id), boundary)
}

fn fixture(payloads: &[&[u8]]) -> FixtureCheckpointProducer {
    FixtureCheckpointProducer::new(payloads.iter().map(|payload| payload.to_vec()).collect())
}

fn controller(payloads: &[&[u8]]) -> CheckpointCaptureAdmission<FixtureCheckpointProducer> {
    CheckpointCaptureAdmission::new(fixture(payloads))
}

fn captured(outcome: CheckpointAdmissionOutcome) -> CheckpointCaptureReceipt {
    match outcome {
        CheckpointAdmissionOutcome::Captured(receipt) => Some(receipt),
        CheckpointAdmissionOutcome::Replayed(_) => None,
    }
    .expect("a new operation must be captured")
}

fn replayed(outcome: CheckpointAdmissionOutcome) -> CheckpointCaptureReceipt {
    match outcome {
        CheckpointAdmissionOutcome::Replayed(receipt) => Some(receipt),
        CheckpointAdmissionOutcome::Captured(_) => None,
    }
    .expect("a duplicate admission must replay")
}

fn unsafe_settlements() -> [CheckpointSettlement; 4] {
    [
        CheckpointSettlement::MidEffect,
        CheckpointSettlement::EnemyExecution,
        CheckpointSettlement::PendingSelectionTransition,
        CheckpointSettlement::Unknown,
    ]
}

#[test]
fn settlement_tokens_are_stable_and_classify_quiescence() {
    assert_eq!(CheckpointSettlement::Quiescent.code(), "quiescent");
    assert_eq!(CheckpointSettlement::MidEffect.code(), "mid_effect");
    assert_eq!(
        CheckpointSettlement::EnemyExecution.code(),
        "enemy_execution"
    );
    assert_eq!(
        CheckpointSettlement::PendingSelectionTransition.code(),
        "pending_selection_transition"
    );
    assert_eq!(CheckpointSettlement::Unknown.code(), "unknown");
    assert!(CheckpointSettlement::Quiescent.is_quiescent());
    for settlement in unsafe_settlements() {
        assert!(!settlement.is_quiescent(), "{}", settlement.code());
    }
}

#[test]
fn quiescent_hold_fences_host_mutation_until_release() {
    let barrier = CheckpointCaptureBarrier::new();
    let fence = barrier.fence();
    assert!(!barrier.is_held());
    assert_eq!(fence.try_mutate(), Ok(()));
    assert_eq!(barrier.admitted_mutations(), 1);

    let guard = barrier
        .try_hold(CheckpointSettlement::Quiescent)
        .expect("quiescent hold");
    assert_eq!(guard.settlement(), CheckpointSettlement::Quiescent);
    assert!(fence.is_fenced());
    assert_eq!(fence.try_mutate(), Err(CheckpointCaptureRejection::Busy));
    assert_eq!(barrier.admitted_mutations(), 1);

    drop(guard);
    assert!(!fence.is_fenced());
    assert!(!barrier.is_held());
    assert_eq!(fence.try_mutate(), Ok(()));
    assert_eq!(barrier.admitted_mutations(), 2);
}

#[test]
fn a_nested_hold_is_refused_as_busy_and_releases_cleanly() {
    let barrier = CheckpointCaptureBarrier::new();
    let guard = barrier
        .try_hold(CheckpointSettlement::Quiescent)
        .expect("first hold");
    assert_eq!(
        barrier.try_hold(CheckpointSettlement::Quiescent).err(),
        Some(CheckpointCaptureRejection::Busy)
    );
    assert_eq!(
        barrier.try_hold(CheckpointSettlement::MidEffect).err(),
        Some(CheckpointCaptureRejection::UnsafeBoundary)
    );
    assert!(barrier.is_held());
    drop(guard);
    assert!(!barrier.is_held());
    assert!(barrier.try_hold(CheckpointSettlement::Quiescent).is_ok());
}

#[test]
fn ledger_replays_an_identical_operation_and_conflicts_on_any_change() -> Result<(), Box<dyn Error>>
{
    let mut admission = controller(&[PAYLOAD]);
    let operation = request("op-1", COMBAT);
    let receipt = captured(admission.capture(CheckpointSettlement::Quiescent, operation.clone())?);
    let digest = blob_digest(PAYLOAD);

    let mut ledger = CheckpointAdmissionLedger::new();
    assert!(ledger.is_empty());
    assert_eq!(
        ledger.decide(&operation, &digest),
        CheckpointAdmissionDecision::New
    );
    ledger.record(&operation, digest.clone(), receipt.clone())?;
    assert_eq!(ledger.len(), 1);
    assert_eq!(
        ledger.decide(&operation, &digest),
        CheckpointAdmissionDecision::Replayed
    );
    assert_eq!(ledger.lookup("op-1"), Some(&receipt));
    assert_eq!(
        ledger.decide(&operation, &blob_digest(OTHER_PAYLOAD)),
        CheckpointAdmissionDecision::Conflict
    );
    assert_eq!(
        ledger.decide(
            &request("op-1", CheckpointBoundary::SettledMapChoice),
            &digest
        ),
        CheckpointAdmissionDecision::Conflict
    );
    assert_eq!(
        ledger.decide(&request("op-2", COMBAT), &digest),
        CheckpointAdmissionDecision::New
    );
    assert_eq!(ledger.lookup("op-2"), None);
    assert_eq!(
        ledger.record(&operation, digest, receipt),
        Err(CheckpointCaptureRejection::OperationConflict)
    );
    assert_eq!(ledger.len(), 1);
    Ok(())
}

#[test]
fn capture_returns_a_private_receipt_without_mutating_gameplay() -> Result<(), Box<dyn Error>> {
    let mut admission = controller(&[PAYLOAD]);
    let receipt =
        captured(admission.capture(CheckpointSettlement::Quiescent, request("op-1", COMBAT))?);

    assert_eq!(receipt.identity().operation_id(), "op-1");
    assert_eq!(receipt.boundary(), COMBAT);
    assert_eq!(receipt.canonical_len(), PAYLOAD.len());
    assert_eq!(receipt.state_digest(), state_id(PAYLOAD));
    assert_eq!(receipt.blob_digest(), blob_digest(PAYLOAD));
    assert_eq!(receipt.durability(), CheckpointDurability::InMemory);
    assert_eq!(
        receipt.checkpoint_id(),
        &receipt.manifest().exact_checkpoint_id()?
    );
    assert_eq!(admission.ledger().len(), 1);
    assert_eq!(admission.producer().produced_count(), 1);
    assert_eq!(admission.barrier().admitted_mutations(), 0);
    assert!(!admission.barrier().is_held());

    let debug = format!("{receipt:?}");
    assert!(debug.contains("canonical_bytes_len"));
    assert!(!debug.contains("asc-state:v1:sha256:"));
    assert!(!debug.contains("asc-checkpoint:v1:sha256:"));
    Ok(())
}

#[test]
fn capture_holds_the_barrier_against_a_host_mutation_inside_the_window()
-> Result<(), Box<dyn Error>> {
    let barrier = CheckpointCaptureBarrier::new();
    let fence = barrier.fence();
    let mut admission = CheckpointCaptureAdmission::with_barrier(
        barrier,
        fixture(&[PAYLOAD]).with_mutation_probe(fence.clone()),
    );

    captured(admission.capture(CheckpointSettlement::Quiescent, request("op-1", COMBAT))?);
    assert_eq!(
        admission.producer().mutation_probe(),
        Some(Err(CheckpointCaptureRejection::Busy))
    );
    assert_eq!(admission.fence().admitted_mutations(), 0);
    assert!(!admission.barrier().is_held());

    assert_eq!(fence.try_mutate(), Ok(()));
    assert_eq!(fence.admitted_mutations(), 1);
    Ok(())
}

#[test]
fn a_duplicate_capture_replays_the_recorded_receipt() -> Result<(), Box<dyn Error>> {
    let mut admission = controller(&[PAYLOAD]);
    let operation = request("op-1", COMBAT);
    let first = captured(admission.capture(CheckpointSettlement::Quiescent, operation.clone())?);
    let second = replayed(admission.capture(CheckpointSettlement::Quiescent, operation)?);

    assert_eq!(first, second);
    assert_eq!(admission.ledger().len(), 1);
    assert_eq!(admission.producer().produced_count(), 2);
    assert_eq!(admission.barrier().admitted_mutations(), 0);
    Ok(())
}

#[test]
fn one_operation_identity_conflicts_on_changed_bytes_or_request() -> Result<(), Box<dyn Error>> {
    let mut admission = controller(&[PAYLOAD, OTHER_PAYLOAD]);
    let operation = request("op-1", COMBAT);
    captured(admission.capture(CheckpointSettlement::Quiescent, operation.clone())?);

    assert_eq!(
        admission
            .capture(CheckpointSettlement::Quiescent, operation)
            .err(),
        Some(CheckpointCaptureRejection::OperationConflict)
    );
    assert_eq!(
        admission
            .capture(
                CheckpointSettlement::Quiescent,
                request("op-1", CheckpointBoundary::SettledMapChoice)
            )
            .err(),
        Some(CheckpointCaptureRejection::OperationConflict)
    );
    assert_eq!(admission.ledger().len(), 1);
    Ok(())
}

#[test]
fn non_quiescent_settlements_never_produce_or_hold_the_barrier() {
    let barrier = CheckpointCaptureBarrier::new();
    let mut admission = controller(&[PAYLOAD]);
    for settlement in unsafe_settlements() {
        assert_eq!(
            barrier.try_hold(settlement).err(),
            Some(CheckpointCaptureRejection::UnsafeBoundary),
            "{}",
            settlement.code()
        );
        assert_eq!(
            admission.capture(settlement, request("op-1", COMBAT)).err(),
            Some(CheckpointCaptureRejection::UnsafeBoundary),
            "{}",
            settlement.code()
        );
    }
    assert!(!barrier.is_held());
    assert_eq!(barrier.admitted_mutations(), 0);
    assert_eq!(admission.producer().produced_count(), 0);
    assert_eq!(admission.ledger().len(), 0);
    assert!(!admission.barrier().is_held());
}

#[test]
fn durable_capture_requires_successful_persistence() -> Result<(), Box<dyn Error>> {
    let operation = request("op-1", COMBAT);
    let mut failing = CheckpointCaptureAdmission::new(
        fixture(&[PAYLOAD])
            .with_durability(CheckpointDurability::Durable)
            .with_persistence(false),
    );
    assert_eq!(
        failing
            .capture(CheckpointSettlement::Quiescent, operation.clone())
            .err(),
        Some(CheckpointCaptureRejection::PersistenceFailed)
    );
    assert_eq!(failing.ledger().len(), 0);
    assert_eq!(failing.producer().produced_count(), 1);
    assert!(!failing.barrier().is_held());

    let mut storing = CheckpointCaptureAdmission::new(
        fixture(&[PAYLOAD]).with_durability(CheckpointDurability::Durable),
    );
    let receipt = captured(storing.capture(CheckpointSettlement::Quiescent, operation)?);
    assert_eq!(receipt.durability(), CheckpointDurability::Durable);
    assert_eq!(storing.ledger().len(), 1);
    Ok(())
}

#[test]
fn the_unavailable_producer_keeps_every_boundary_fail_closed() {
    let mut admission = CheckpointCaptureAdmission::new(UnavailableCheckpointProducer);
    assert_eq!(
        admission.producer().capability(),
        CheckpointProducerCapability::Unavailable(
            CheckpointUnavailableReason::ExactHostEvidenceRequired
        )
    );
    let capabilities = CheckpointCapabilities;
    for boundary in CheckpointBoundary::all() {
        let expected = match capabilities.for_boundary(*boundary).reason() {
            // An unsafe phase is refused by the owner matrix before any producer is consulted.
            CheckpointUnavailableReason::UnsafeBoundary => {
                CheckpointCaptureRejection::UnsafeBoundary
            }
            // A merely pending boundary is decided by the producer's own fail-closed gate, so a
            // synthetic fixture producer may still serve it while this producer refuses.
            _ => CheckpointCaptureRejection::UnsupportedBoundary {
                boundary: *boundary,
                reason: CheckpointUnavailableReason::ExactHostEvidenceRequired,
            },
        };
        assert_eq!(
            admission
                .capture(CheckpointSettlement::Quiescent, request("op-1", *boundary))
                .err(),
            Some(expected),
            "{}",
            boundary.code()
        );
    }
    assert_eq!(admission.ledger().len(), 0);
    assert!(admission.fence().try_mutate().is_ok());
}

#[test]
fn unsafe_boundaries_are_rejected_even_for_a_synthetic_producer() {
    let mut admission = controller(&[PAYLOAD]);
    for boundary in [
        CheckpointBoundary::EnemyTurn,
        CheckpointBoundary::Animation,
        CheckpointBoundary::Transition,
        CheckpointBoundary::Unknown,
    ] {
        assert_eq!(
            admission
                .capture(CheckpointSettlement::Quiescent, request("op-1", boundary))
                .err(),
            Some(CheckpointCaptureRejection::UnsafeBoundary),
            "{}",
            boundary.code()
        );
    }
    assert_eq!(admission.producer().produced_count(), 0);
    assert_eq!(admission.ledger().len(), 0);
}

#[test]
fn an_externally_held_barrier_makes_the_controller_refuse_as_busy() -> Result<(), Box<dyn Error>> {
    let barrier = CheckpointCaptureBarrier::new();
    let guard = barrier
        .try_hold(CheckpointSettlement::Quiescent)
        .expect("external hold");
    let mut admission = CheckpointCaptureAdmission::with_barrier(barrier, fixture(&[PAYLOAD]));

    assert_eq!(
        admission
            .capture(CheckpointSettlement::Quiescent, request("op-1", COMBAT))
            .err(),
        Some(CheckpointCaptureRejection::Busy)
    );
    assert_eq!(admission.producer().produced_count(), 0);

    drop(guard);
    captured(admission.capture(CheckpointSettlement::Quiescent, request("op-1", COMBAT))?);
    assert_eq!(admission.ledger().len(), 1);
    Ok(())
}

#[test]
fn an_empty_fixture_has_no_canonical_payload_to_offer() {
    let mut admission: CheckpointCaptureAdmission<FixtureCheckpointProducer> =
        CheckpointCaptureAdmission::new(FixtureCheckpointProducer::new(Vec::new()));
    assert_eq!(
        admission
            .capture(CheckpointSettlement::Quiescent, request("op-1", COMBAT))
            .err(),
        Some(CheckpointCaptureRejection::InvalidCanonicalBytes)
    );
    assert_eq!(admission.ledger().len(), 0);
}

#[test]
fn the_ledger_is_bounded_and_refuses_a_new_operation_at_capacity() -> Result<(), Box<dyn Error>> {
    let capacity = CheckpointAdmissionLedger::new().capacity();
    assert_eq!(capacity, CHECKPOINT_ADMISSION_MAX_OPERATIONS);

    let mut admission = controller(&[PAYLOAD]);
    for index in 0..capacity {
        captured(admission.capture(
            CheckpointSettlement::Quiescent,
            request(&format!("op-{index}"), COMBAT),
        )?);
    }
    assert_eq!(admission.ledger().len(), capacity);
    assert_eq!(
        admission
            .capture(
                CheckpointSettlement::Quiescent,
                request("op-overflow", COMBAT)
            )
            .err(),
        Some(CheckpointCaptureRejection::AdmissionLedgerFull)
    );
    assert_eq!(admission.ledger().len(), capacity);

    let replayed = admission.capture(CheckpointSettlement::Quiescent, request("op-0", COMBAT))?;
    assert!(matches!(replayed, CheckpointAdmissionOutcome::Replayed(_)));
    Ok(())
}
