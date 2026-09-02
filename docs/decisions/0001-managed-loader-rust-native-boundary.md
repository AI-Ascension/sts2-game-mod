# ADR 0001: Managed loader and Rust native boundary

- Status: Accepted boundary; load-smoke package added by ADR 0009
- Date: 2026-09-02

## Context

The selected host contract requires a managed .NET assembly carrying loader metadata. A normal Rust
cdylib is a native library and cannot provide that managed assembly metadata. The preserved
interop experiment demonstrates the shape of a narrow managed-to-native C ABI without containing
the host assembly.

## Decision

The mod load-smoke package contains a thin managed loader, a Windows x86-64 Rust native companion,
and a manifest with target-owned metadata. The managed side owns loader metadata, callback entry,
native-library lifetime, and ABI translation only. Rust owns host-independent policy, host
adaptation, the local HTTP adapter, and composition. MCP remains a separate process and never links
to host code. Additional supported platforms and gameplay behavior require separate evidence.

The ABI is versioned and narrow. Managed objects, host objects, Rust references, exceptions,
allocator-owned buffers, and unbounded strings do not cross it. Fixed-width values, explicit
pointer/length ownership, paired release rules, stable status values, and panic containment are
required before implementation.

## Alternatives

- A pure Rust managed assembly was rejected because the required managed metadata is not a normal
  Rust cdylib output.
- A native engine extension was rejected because it is a different loader contract.
- A sidecar process is deferred because it adds IPC, authentication, lifecycle, and failure
  contracts before the in-process seam is understood.
- Managed implementation of all behavior is a fallback that conflicts with the Rust-first goal.

## Consequences and evidence

Unsafe code and marshalling remain isolated and testable. Native unload remains conservative until
host lifecycle evidence proves it safe. ADR 0009 records the bounded load-smoke package and its
separate host evidence; no host assembly is stored here, and load-smoke does not claim gameplay.
