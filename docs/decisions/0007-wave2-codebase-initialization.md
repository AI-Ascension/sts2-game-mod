# ADR 0007: Wave 2 codebase initialization

- Status: Accepted for Wave 2 preparation; runtime package superseded by ADR 0009
- Date: 2026-09-02

## Context

The mod target needs non-empty product source before host-specific implementation can begin. Its
ownership boundary is the managed loader, host integration, main-thread boundary, authoritative
owner-local HTTP adapter, and narrow Rust/native seam. The source must be useful for deterministic
validation without a proprietary STS2 host assembly or a game launch.

## Decision

Initialize three target-local Rust crates:

- `sts2-game-mod-host` owns owned host projections, a bounded FIFO main-thread queue, dispatch, and a
  versioned pointer-width ABI descriptor gate.
- `sts2-game-mod-http-adapter` owns a bounded, transport-free request adapter port. It does not open a
  socket or define a public route catalog.
- `sts2-game-mod` composes the ports and makes HTTP admission versus main-thread pumping explicit.

The crates use no cross-repository path dependencies. Tests use deterministic fake host, queue,
ABI, and adapter implementations. The current seam carries only opaque bytes and language- and
transport-neutral boundary values; game rules, gateway lifecycle/routing, MCP semantics, provider
behavior, storage, and real host callbacks remain outside this initialization.

The existing `experiments/managed-rust-interop` native member remains in the workspace through
inherited fields. The later runtime package and host evidence are intentionally outside this Wave 2
initialization decision. No proprietary assembly, generated build output, credentials, or copied
implementation source is added to the repository.

## Consequences

The target has compilable source-level ownership seams. This ADR alone proves only generic Rust
behavior and deterministic fakes; the managed load-smoke package and its separate evidence are
defined by ADR 0009. Thread affinity, HTTP serving, mutation settlement, and package safety remain
outside that load-smoke.
