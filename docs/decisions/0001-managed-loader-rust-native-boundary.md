# ADR 0001: Managed loader and Rust native boundary

- Status: Accepted for preparation; runtime-load gate outstanding
- Date: 2026-09-02

## Context

The selected host contract requires a managed .NET assembly carrying loader metadata. A normal Rust
cdylib is a native library and cannot provide that managed assembly metadata. The preserved
interop experiment demonstrates the shape of a narrow managed-to-native C ABI without containing
the host assembly.

## Decision

The future mod package will contain a thin managed loader, a Rust native component for each
supported platform, and a manifest with licenses and user documentation. The managed side owns
loader metadata, callback entry, native-library lifetime, and ABI translation only. Rust owns
host-independent policy, host adaptation, the local HTTP adapter, and composition. MCP remains a
separate process and never links to host code.

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
host lifecycle evidence proves it safe. The experiment is source-level preparation only: no game
package was installed, no host assembly is stored here, and no game load or runtime claim follows.
