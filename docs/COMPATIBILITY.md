# Compatibility policy and matrix

## Separate dimensions

Compatibility is tracked independently for the owner-local HTTP contract, managed loader metadata,
native ABI, STS2 host version, operating system and architecture, Rust/.NET runtime, configuration,
and package contents. A build result never silently broadens another dimension.

The initial game baseline is recorded in
[decision 0002](decisions/0002-initial-game-compatibility-baseline.md). It is a source-derived
planning target, not runtime proof from this target.

## Evidence levels

1. Build-only: source compiles with the declared toolchain and any authorized host reference.
2. Load smoke: a packaged managed entry point is discovered and starts in the exact game.
3. Focused runtime: a disposable profile exercises the affected host path.
4. Full conformance: all required host-dependent and owner-local suites pass.

Every claim records exact game version or commit, host assembly hash when used, operating system,
architecture, managed/runtime versions, source revision, artifact hash, date, inputs, outputs,
cleanup, and evidence level. Absence of evidence remains unverified.

## Current matrix

| Managed/native target | Game host | Platform | Evidence | Result |
| --- | --- | --- | --- | --- |
| Preserved interop experiment | No host file in target | Portable source shape | Source-level preparation | Does not claim game load |
| Future mod package | STS2 v0.107.1, commit 59260271 | Windows x86-64 | Unverified | Host/load/runtime work remains |

The project planning baseline names a host assembly identity, but this repository does not retain
that proprietary file. No support claim is made for beta builds, earlier versions, Linux, macOS,
or another architecture until an exact matrix row and evidence exist.

## Contract compatibility

The checked-in `protocol-artifact/poc-v1/` copy is consumed by exact protocol version, schema
digest, and provenance. It is an offline release-like input, not a package or runtime compatibility
claim. The game-mod POC mapping is source/test-confirmed only; host and game compatibility remain
unverified.

HTTP and ABI changes are classified as internal, additive-compatible, safety correction,
deprecated-compatible, or breaking. A route, field, status, error, ordering, ABI symbol, calling
convention, or host callback change needs a requirement, fixture, migration note, and version
decision. Game-host adaptation and HTTP compatibility are versioned independently.

The host boundary must preserve main-thread affinity, explicit acceptance versus completion, bounded
queue behavior, and sanitized errors. A client disconnect or timeout does not revoke host work
already accepted.

## Promotion and deprecation

Do not promote build-only evidence to load or runtime support. A deprecation names a replacement,
warning, first deprecated release, earliest removal release, and compatibility tests. A breaking
change needs explicit approval and coordinated updates to the adapter, schema, package, and
consumers.
