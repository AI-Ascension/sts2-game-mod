# ADR 0007: Wave 2 codebase initialization

- Status: Accepted for Wave 2 preparation
- Date: 2026-09-02

## Context

The mod target needs non-empty product source before host-specific implementation can begin. Its
ownership boundary is the managed loader, host integration, main-thread boundary, authoritative
owner-local HTTP adapter, and narrow Rust/native seam. The source must be useful for deterministic
validation without a proprietary STS2 host assembly or a game launch.

## Decision

Initialize three target-local Rust crates:

- `sts2-mod-host` owns owned host projections, a bounded FIFO main-thread queue, dispatch, and a
  versioned pointer-width ABI descriptor gate.
- `sts2-mod-http-adapter` owns a bounded, transport-free request adapter port. It does not open a
  socket or define a public route catalog.
- `sts2-game-mod` composes the ports and makes HTTP admission versus main-thread pumping explicit.

The crates use no cross-repository path dependencies. Tests use deterministic fake host, queue,
ABI, and adapter implementations. The current seam carries only opaque bytes and language- and
transport-neutral boundary values; game rules, gateway lifecycle/routing, MCP semantics, provider
behavior, storage, and real host callbacks remain outside this initialization.

The existing `experiments/managed-rust-interop` preparation remains unchanged and remains a
workspace member through inherited fields. No proprietary assembly, generated build output,
credentials, copied implementation source, or game package is added.

## Consequences

The target now has compilable source-level ownership seams that later managed composition can
consume. A green local build proves only generic Rust behavior and deterministic fakes. Real loader
discovery, host ABI compatibility, thread affinity, HTTP serving, mutation settlement, and package
safety remain unverified and require later requirements plus authorized host evidence.
