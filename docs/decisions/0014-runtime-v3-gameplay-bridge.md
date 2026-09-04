# ADR 0014: Runtime-v3 host-thread gameplay bridge

- Status: Accepted as a source-only boundary; licensed-host validation remains unverified
- Date: 2026-09-04

## Context

The neutral Runtime-v3 profile needs a producer that can expose ordinary player-visible state and
the complete current semantic action catalog. The game host, not the mod policy or provider, must
own state, legality, mutation, thread affinity, and settlement. Co-op metadata must not weaken
those rules.

## Decision

Keep the managed Runtime-v3 bridge behind `RuntimeV3GameplayHost` and its injected host source and
main-thread queue. It serializes only the bounded fair-play projection, rejects unknown or
privileged fields, requires a generation-matching typed action from the host catalog, records
idempotent operation receipts, and reports `unknown` when dispatch or settlement cannot be proven.
Postconditions require a fresh generation and an independent transition witness. Co-op projection
is additive and suspends mutation for disagreement, missing peers, disconnect, or invalid peer
identity.

The checked-in managed files remain a source-only compatibility seam because no licensed STS2/Godot
assemblies are available in this workspace. No host object graph, save, executable, raw input,
reflection path, future RNG, or model policy crosses the boundary.

## Evidence

Rust contract/fake tests and static managed source checks are source-derived. Exact assembly build,
host-thread execution, target legality, co-op runtime, and effect settlement are unverified.
