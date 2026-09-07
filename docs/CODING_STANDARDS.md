# Coding standards

## Goals

Prefer small, cohesive modules whose ownership is obvious. Compatibility, host safety, provenance,
and diagnosable failure take precedence over implementation speed.

## Toolchain and file budgets

Rust uses edition 2024 and the pinned toolchain in rust-toolchain.toml. The committed lockfile
covers the policy tool and the preserved experiment. Managed projects use the root
Directory.Build.props settings: nullable analysis, deterministic output, analyzers, and warnings
as errors.

The policy checker counts nonblank physical lines:

| Artifact | Preferred | Hard |
| --- | ---: | ---: |
| production Rust | 300 | 400 |
| Rust tests | 400 | 600 |
| managed C# | 250 | 350 |
| managed C# tests | 350 | 500 |
| workflows | 160 | 200 |
| Markdown | 500 | 700 |

Refactor by responsibility before adding an exact-path exemption. Generated or imported material
needs provenance, regeneration, and a narrow policy entry.

## Rust and managed code

Use rustfmt and run Clippy with warnings denied. Public Rust items document behavior, errors,
panics, compatibility, and safety. Prefer typed values, explicit units and lifetimes, and
Result-based error handling. Production paths do not use unwrap, expect, panic, todo, or
unimplemented.

Unsafe Rust is denied by default and confined to the native/host boundary. Every unsafe block
documents pointer validity, ownership, aliasing, thread, allocator, and unload assumptions.
Fixed-width ABI types and explicit status values are required.

Managed code is the narrow exception. It may contain loader metadata, host callbacks, native
library lifetime, and ABI conversion. It must not grow domain, HTTP, MCP, persistence, or
orchestration behavior.

## Boundaries and modules

Keep transport decoding at the HTTP adapter, host objects at the host boundary, and application
policy in the host-independent core target. Composition roots wire dependencies but do not own
business rules. Avoid generic utils, helpers, managers, or services that hide mixed ownership.

Do not expose mutable global state. Make queue capacity, lock ordering, timeout, cancellation,
shutdown, and scheduler behavior explicit. Do not make a client timeout imply cancellation after
the host accepts work.

## Protocol, security, and observability

External field names, routes, status codes, errors, ordering, and null/missing behavior require
explicit serialization and deterministic fixtures. Validate untrusted input at the boundary and
map internal errors to stable sanitized wire errors once.

Do not add ambient network discovery, remote-by-default binding, fallback credentials, unbounded
request bodies, unbounded response output, or arbitrary host reflection. Logs may include bounded
correlation and lifecycle information but not credentials, saves, private paths, or raw host data.

## Provenance

Write original code and documentation. Do not copy, vendor, transliterate, or use another
implementation's source symbols as requirements. Record the origin, license, and generator for
imported or generated fixtures. Never place proprietary host files, game data, credentials, or
personal profiles in the tree.

## Review

Changes affecting ownership, dependencies, public HTTP behavior, ABI, host strategy, lifecycle,
security, or packaging require an architecture decision. Every change reports exact checks,
compatibility classification, and unverified evidence. See [TESTING.md](TESTING.md) and
[WORKFLOWS.md](WORKFLOWS.md).

## Aggregate naming authority

Use the aggregate [`NAMING_CONVENTIONS.md`](../../planning/naming_conventions/NAMING_CONVENTIONS.md)
and [`naming-registry.yaml`](../../planning/naming_conventions/naming-registry.yaml) for owner
prefixes, casing, identity namespaces, lifecycle vocabulary, evidence states, and exceptions. The
managed/native experiment is game-mod-owned; host-required assembly, loader, manifest, and ABI
spellings remain protected whenever the registry marks them external or consumed.
