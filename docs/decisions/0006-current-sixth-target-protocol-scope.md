# ADR 0006: Current sixth-target protocol scope

- Status: Accepted for the current build-completion run
- Date: 2026-09-02

## Context

The current run explicitly includes sts2-protocol as the sixth implementation target, superseding
the earlier decision-stage/deferred disposition. That inclusion does not make every boundary
contract shared or transfer ownership from the target that observes it.

## Decision

sts2-protocol is an accepted, narrowly owned repository target for genuinely shared
language-neutral and transport-neutral contracts, with its own versioning, consumers, and
conformance evidence. It must not absorb host objects, managed loader metadata, main-thread
behavior, game rules, gateway leases, MCP framing, or harness artifacts.

sts2-game-mod retains ownership of its host boundary and owner-local HTTP contract during this
wave. No protocol crate, schema, or implementation is added here. A candidate contract moves to
sts2-protocol only after the protocol owner records at least two independent consumers, stable
semantics, versioning, and conformance needs. Until then, the mod's local decisions remain local
and any unresolved shared contract is explicitly blocked rather than invented.

## Consequences

The mod can initialize host and HTTP work without creating a second wire authority. Protocol
compatibility remains distinct from game-host compatibility. Cross-target implementation must
consume only an accepted protocol contract and must preserve this ownership boundary.
