# ADR 0055: checkpoint capture admission barrier and operation ledger

- Status: Proposed; source-only admission controller
- Date: 2026-09-15
- Tracking: game-mod #80
- Depends on: [ADR 0037](0037-native-checkpoint-coverage-inventory.md),
  [ADR 0042](0042-native-checkpoint-restore-boundary.md),
  [ADR 0043](0043-native-checkpoint-capture-port.md),
  [ADR 0054](0054-restricted-canonical-checkpoint-encoder.md)

## Context

ADR 0043 defined the inert capture port and kept every owner boundary unavailable. Issue #80 also
requires ordered owner work and one-receipt-per-operation binding: no host mutation may occur
between boundary validation and snapshotting, a duplicate admission of one logical operation must
return the originally recorded receipt, the same operation with different bytes or request must be
a conflict, a durable capture that fails to persist must not report success, and an in-memory
artifact must stay distinguishable from a durable one.

None of that existed. `CheckpointCaptureRejection::{Busy, UnsupportedCoverage, PersistenceFailed,
OperationConflict}` were declared but never produced, and no barrier, ledger, or admission path
existed. A producer attached to the port could therefore publish an artifact without any owner
record of the operation that produced it.

## Decision

`crates/game-mod` now owns a source-only `checkpoint::admission` module with three parts.

**Settlement and barrier.** `CheckpointSettlement` classifies the host game thread as `Quiescent`,
`MidEffect`, `EnemyExecution`, `PendingSelectionTransition`, or `Unknown`, and only `Quiescent`
admits a capture. `CheckpointCaptureBarrier::try_hold` classifies settlement before mutual
exclusion, so an unsafe host phase is never reported as mere contention, and returns an owned
`CheckpointBarrierGuard` that releases the barrier on drop. While the guard is live, host work
offered through a cloned `CheckpointMutationFence` handle is refused with `Busy`, and a nested hold
attempt is refused the same way. A rejected attempt leaves the barrier state untouched. The barrier
owns no host object, draws no RNG, mutates no gameplay state, and persists nothing; it only orders
owner work against host work on one non-`Send` game thread.

**Operation ledger.** `CheckpointAdmissionLedger` is bounded by the public
`CHECKPOINT_ADMISSION_MAX_OPERATIONS` (256) and keyed by the request's logical operation identity.
Each entry retains the request, the payload digest, and the receipt.
`CheckpointAdmissionLedger::decide` classifies a candidate without mutating the ledger as `New`,
`Replayed`, `Conflict`, or `Full`; `record` writes one entry and refuses past capacity with
`AdmissionLedgerFull`. The ledger stores owner-side receipt values only. It never persists an
artifact and never decides native capability.

**Producer seam and ordering.** `CheckpointCaptureProducer` separates capability, durability,
production, and persistence. The default `UnavailableCheckpointProducer` advertises
`Unavailable(ExactHostEvidenceRequired)` and closes nothing, so every boundary stays fail-closed.
`FixtureCheckpointProducer` is explicitly synthetic: it reads no host object, serializes no native
field, and can carry a host mutation handle that it attempts once inside the capture window, which
is how source tests prove the window excludes mutation without a live host.

`CheckpointCaptureAdmission::capture` then orders one operation: classify the boundary, consult the
producer, hold the barrier through production and persistence, classify the ledger decision,
re-produce under the barrier on a replay so a hidden payload change is detected as a conflict,
persist when the producer reports `Durable`, build the receipt, and only then record it. A
rejection never records a receipt, so retrying the same operation after a failure may still
succeed.

The owner boundary matrix decides before any producer: `EnemyTurn`, `Animation`, `Transition`, and
`Unknown` are refused as unsafe for every producer, including a synthetic fixture. A boundary that
is merely pending coverage inventory or exact-host evidence is left to the attached producer, so
its receipt or refusal carries that producer's own gate instead of the matrix default.

## Evidence and limits

`crates/game-mod/tests/checkpoint_admission.rs` adds 15 source-only tests over the new module:
settlement tokens, fence and release behavior, non-quiescent refusal without holding the barrier, a
nested hold refused as `Busy`, ledger replay, conflict, lookup, and capacity, private receipt bytes
with no gameplay mutation, a mutation probe refused inside the capture window, duplicate replay,
changed payload and changed request conflicts, pre-production refusal, durable persistence failure,
an unavailable producer refused across the complete boundary matrix, unsafe boundaries refused even
for a synthetic producer, an externally held barrier, an empty fixture, and replay at capacity.
`crates/game-mod/tests/checkpoint_admission_debug.rs` adds one more: the produced checkpoint, the
ledger, and the fixture producer render redacted `Debug` output that carries no canonical payload
bytes and no exact state or blob digest.

This is synthetic consumer evidence only. Nothing here captures a real game boundary, reads a
native field, inspects a proprietary assembly, persists an artifact to a real store, or restores a
run, and no acceptance criterion that requires native or exact-host evidence is closed by this
record. The admission controller leaves the ADR 0042 restore boundary untouched: it decides whether
a capture may be taken, never whether a captured artifact can be restored.

Native progress still requires an authorized exact build, its host assembly, a disposable isolated
profile, and a controlled-host trace under ADR 0037, ADR 0042, and ADR 0043.

## Consequences

The previously unproduced rejection variants are now reachable from owner code, so a future
authorized producer cannot publish a checkpoint without a barriered window and one recorded receipt.
The ledger bound is public, so an attaching producer knows the retention limit instead of
discovering it at capacity.

The barrier and ledger remain prerequisites, not evidence: attaching a real producer still needs
the exact-host gates above, and the synthetic fixture must never be presented as native capture
evidence.
