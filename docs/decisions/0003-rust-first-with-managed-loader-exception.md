# ADR 0003: Rust-first implementation with a managed loader exception

- Status: Accepted boundary; runtime load-smoke added by ADR 0009
- Date: 2026-09-02

## Context

The system needs Rust for product, transport, policy, fixtures, and test tooling while the host
loader needs managed metadata. Mixing those roles would make ownership and provenance ambiguous.

## Decision

All product behavior, HTTP mapping, repository checks, fixtures-as-code, and test harnesses are
Rust. The sole language exception is a minimal managed loader that owns host metadata, callbacks,
native lifetime, and ABI translation. It must not own game rules, HTTP, MCP, persistence, or
orchestration.

The interop directory remains separately labeled and owns only the thin runtime loader/package
required by ADR 0009. This decision does not copy, vendor, or transliterate any other
implementation.

## Consequences

The policy gate can reject Python source and package metadata and can require SPDX headers on Rust
and managed source. Managed and native changes require paired review. A new language or broader
managed responsibility needs a superseding decision with a demonstrated boundary.
