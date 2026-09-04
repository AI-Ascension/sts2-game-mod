# ADR 0014: Runtime-v2 deterministic fake boundary

- Status: Accepted for source and fake conformance only
- Date: 2026-09-02
- Protocol handoff: `sts2-protocol` commit `8d4b2f574cf860a71f2a5e4ce3308ac069cb1527`

## Context

The Runtime-v2 contract distinguishes admission from settlement and requires stable operation
identity, fencing metadata, bounded observations, fresh post-action evidence, and reconciliation
after an uncertain delivery. This target needs a deterministic seam that can prove those invariants
without guessing the proprietary STS2 host API or touching a profile, save, or provider file.

## Decision

Keep the exact release-like Runtime-v2 bytes under `protocol-artifact/runtime-v2/` and the copied
source schema under `schemas/runtime-v2.schema.json`. The owner-local artifact verifier pins
protocol version `runtime-v2`, schema digest
`f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2`, source
`schemas/runtime-v2.schema.json`, and generator `hand-authored`. There is no sibling-checkout or
filesystem dependency.

`RuntimeV2Mod` owns a bounded in-memory receipt store and main-thread queue. `FakeRuntimeV2Game`
implements only the owned `RuntimeV2GamePort`: a valid player-turn `end_turn` advances the fake
observation once. Admission returns `accepted`; only a validated fresh post-action observation and
`turn_end_settled` witness produce `settled`. Replays use the retained receipt. Conflicting
operation reuse, stale identity/generation/lease data, illegal phases, and capacity exhaustion
fail closed. A simulated post-write disconnect returns `unknown`, and reconciliation can return
the retained settled receipt. A pre-dispatch timeout removes queued work so it cannot execute later
without a known outcome.

Runtime-v1 routes and tests remain unchanged. This decision adds no managed mapping and no live
host action. No concrete host gameplay API exists in this repository; live host mutation and live
host settlement are unverified. The fake seam is source/build/test evidence only.

## Consequences

The lifecycle and failure semantics are testable offline, including exactly-once in-memory fake
application and no-blind-retry behavior. A future host adapter must provide an explicit, reviewed
implementation of the port and fresh host evidence before any Runtime-v2 gameplay claim can be
made. Existing Runtime-v1 probe evidence does not broaden to Runtime-v2 or gameplay.
