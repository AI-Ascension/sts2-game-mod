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

Observation reads (including recovery reobservation) return the current authoritative
generation even when the caller has an older one. New mutations and legal-catalog
requests retain exact-generation admission checks.

The receipt key includes instance, session, lease, epoch, and operation identity.
Exact retries replay the stored receipt before fresh admission; changing the state,
action payload, or generation under that key is an idempotency conflict. Receipts
retain their observation/catalog snapshot rather than rereading changed host state.

The source-only host port now receives the operation key on dispatch and supplies
an optional completion snapshot with a witness bound to that operation and action.
The bridge never manufactures a witness from a generation increment. Without that
independent host evidence the receipt stays unknown; read-only polling may establish
later completion without repeating the mutation. A concrete host adapter must implement
this completion port from an authoritative operation-completion callback or verified
action-specific postcondition; the synthetic probe does not establish STS2 semantics.

This is a safety correction to an unmerged, source-only internal port. The wire
profile and digest are unchanged. The ModEntry interop binding is separated from
the managed handler so the actual handler can compile and execute without game DLLs.

The checked-in managed files remain a source-only compatibility seam because no licensed STS2/Godot
assemblies are available in this workspace. No host object graph, save, executable, raw input,
reflection path, future RNG, or model policy crosses the boundary.

## Evidence

Rust contract/fake tests and static managed source checks are source-derived. Exact assembly build,
host-thread execution, target legality, co-op runtime, and effect settlement are unverified.

The host-independent RuntimeV3ValidationProbe exercises the actual managed handler
with synthetic host completion events, including unknown outcomes, delayed completion,
receipt replay/conflicts/isolation, stale queued actions, refresh, and malformed input.
