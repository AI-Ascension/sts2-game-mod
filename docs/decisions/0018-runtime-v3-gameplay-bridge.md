# ADR 0018: Runtime-v3 host-thread gameplay bridge

- Status: Accepted as a source-only boundary; licensed-host validation remains unverified
- Date: 2026-09-04

## Context

The neutral Runtime-v3 profile needs a producer that can expose ordinary player-visible state and
the complete current semantic action catalog. The game host, not the mod policy or provider, must
own state, legality, mutation, thread affinity, and settlement. Co-op metadata must not weaken
those rules.

## Decision

Managed request methods separate envelope identity validation, typed action payload validation,
receipt replay, fresh dispatch admission, and recovery observation. Response construction separates
owned observation projection, provenance, transition projection, and bounded serialization.
Co-op projection validates peer membership before copying the owned result. These responsibility
splits retain validation order, wire fields, error codes, and receipt semantics while keeping new
or modified methods within the 60-line refactoring threshold; no policy exemption is used.

The 2026-09-05 integration preserves the independently merged v2 adapter from PR #26. V2
callback IDs remain 3, 4 and 5; the semantic gameplay callback becomes 6. Both handlers use the
same bound instance/caller/session/lease/epoch authorizer. An accepted or unknown semantic
receipt blocks new v2 admission. Semantic dispatch checks an injected v2-pending predicate
immediately before mutation, so a queue delay cannot bypass the shared exclusion. Exact receipt
replay, observation and read-only reconciliation remain available. The current initializer is
still unconfigured; no concrete host source or live gameplay evidence is added by this wiring.

The managed handler is split into partial files for routing, observation, dispatch,
request validation, response construction, and serialization. These files share one
host owner and remain subject to the normal managed file budgets; no licensed SDK
is required to validate this organization with the source-linked gameplay probe.

Keep the managed Runtime-v3 bridge behind `RuntimeV3GameplayHost` and its injected host source and
main-thread queue. It serializes only the bounded fair-play projection, rejects unknown or
privileged fields, requires a generation-matching typed action from the host catalog, records
idempotent operation receipts, and reports `unknown` when dispatch or settlement cannot be proven.
Postconditions require a fresh generation and an independent transition witness.
The isolated co-op projection helper computes a mutation-admission recommendation
for disagreement, missing peers, disconnect, or invalid identity. It is not yet
connected to gameplay admission or the wire profile; no integrated co-op claim is made.

Observation reads (including recovery reobservation) return the current authoritative
generation even when the caller has an older one. New mutations and legal-catalog
requests retain exact-generation admission checks.

A stale dispatch returns `dispatch_action_response` with `rejected`,
`stale_generation`, and the current authoritative observation/catalog. If the host
is unavailable, dispatch retains that response kind and operation identity with
`unknown`; neither path substitutes a reobservation response. The neutral catalog
response has no failure variant, so failed catalog reads use an owner-local HTTP
error body containing `correlation_id`, `error_code`, and `recovery: "reobserve"`:
409 for stale state/generation, 503 for an unavailable host. This is not a neutral
gameplay message or an observation; consumers must process non-success HTTP status
before attempting to decode a successful catalog envelope. No schema change is made.

The receipt key includes instance, session, lease, epoch, and operation identity.
Exact retries replay the stored receipt before fresh admission; changing the state,
action payload, or generation under that key is an idempotency conflict. Receipts
retain their observation/catalog snapshot rather than rereading changed host state.
The snapshots copy source collections; an `IReadOnlyList` alone does not establish
ownership of the underlying mutable list. Read-only reconciliation retrieves the
scoped receipt and queries independent completion, without repeating the mutation.

The source-only host port now receives the operation key on dispatch and supplies
an optional completion snapshot with a witness bound to that operation and action.
The bridge never manufactures a witness from a generation increment. Without that
independent host evidence the receipt stays unknown; read-only polling may establish
later completion without repeating the mutation. A concrete host adapter must implement
this completion port from an authoritative operation-completion callback or verified
action-specific postcondition; the synthetic probe does not establish STS2 semantics.

This is a safety correction to an unmerged, source-only internal port. It does not
change the completion witness wire shape. A separately coordinated protocol schema
correction changes the accepted digest; see the compatibility matrix. The ModEntry interop binding is separated from
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
